use std::{borrow::Borrow, mem};

use crate::{Error, Result};

/// Persistent array, that can also be used as mutable vector.
pub struct Vector<T>
where
    T: Sized,
{
    len: usize,
    root: NodeRef<T>,
    auto_rebalance: bool,
    leaf_cap: usize,
}

impl<T> Clone for Vector<T> {
    fn clone(&self) -> Vector<T> {
        Vector {
            len: self.len,
            root: NodeRef::clone(&self.root),
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        }
    }
}

#[cfg(any(feature = "arbitrary", feature = "fuzzing", test))]
impl<T> arbitrary::Arbitrary for Vector<T>
where
    T: Clone + arbitrary::Arbitrary,
{
    fn arbitrary(u: &mut arbitrary::unstructured::Unstructured) -> arbitrary::Result<Self> {
        let k = std::mem::size_of::<T>();
        let leaf_cap = *u.choose(&[k, k * 2, k * 100, k * 1000, k * 10000])?;
        let auto_reb = *u.choose(&[true, false])?; // auto_rebalance

        let arr: Vec<T> = u.arbitrary()?;
        let mut arr = Vector::from_slice(&arr, Some(leaf_cap));
        arr.set_auto_rebalance(auto_reb);
        Ok(arr)
    }
}

impl<T> IntoIterator for Vector<T>
where
    T: Clone,
{
    type Item = T;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let mut iter = IntoIter {
            stack: Vec::default(),
            node: None,
            off: 0,
        };
        Node::build_into_iter_stack(&self.root, &mut iter);
        iter
    }
}

impl<T> Vector<T>
where
    T: Sized,
{
    /// Create a new empty Vector.
    pub fn new() -> Vector<T> {
        Vector {
            len: 0,
            root: Node::empty_leaf(),
            auto_rebalance: true,
            leaf_cap: crate::LEAF_CAP,
        }
    }

    /// Construct a new vector with an initial array of values.
    pub fn from_slice(slice: &[T], leaf_node_size: Option<usize>) -> Vector<T>
    where
        T: Clone,
    {
        let n = max_leaf_items::<T>(leaf_node_size.unwrap_or(crate::LEAF_CAP));

        let zs: Vec<NodeRef<T>> = slice
            .chunks(n)
            .map(|x| NodeRef::new(Node::from(x)))
            .collect();

        let depth = (zs.len() as f64).log2().ceil() as usize;
        let mut iter = zs.into_iter();
        let item = iter.next();
        let (root, _) = Node::build_bottoms_up(depth, item, &mut iter);

        assert!(iter.next().is_none());

        Vector {
            len: slice.len(),
            root,
            auto_rebalance: true,
            leaf_cap: leaf_node_size.unwrap_or(crate::LEAF_CAP),
        }
    }

    /// Set the size of the leaf node in bytes. Number of items inside
    /// the leaf node is computed as `(leaf_size / mem::size_of::<T>()) + 1`
    /// Setting a large value will make the tree shallow giving better
    /// read performance, at the expense of write performance.
    pub fn set_leaf_size(&mut self, leaf_size: usize) -> &mut Self {
        self.leaf_cap = leaf_size;
        self
    }

    /// Auto rebalance is enabled by default. This has some penalty for write
    /// heavy situations, since every write op will try to rebalance the tree
    /// when it goes too much off-balance. Application can disable
    /// auto-rebalance to get maximum efficiency, and call [Self::rebalance]
    /// method as and when required. Make sure *you know what you are doing*
    /// before disabling auto-rebalance.
    pub fn set_auto_rebalance(&mut self, rebalance: bool) -> &mut Self {
        self.auto_rebalance = rebalance;
        self
    }
}

impl<T> Vector<T>
where
    T: Sized,
{
    /// Return the length of the vector, that is, number of elements in the
    /// vector.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Return the memory foot-print for this instance.
    pub fn footprint(&self) -> usize {
        mem::size_of_val(self) + self.root.footprint()
    }

    /// Return a reference to the element at that position or `IndexFail` error
    /// if out of bounds.
    pub fn get(&self, index: usize) -> Result<&T> {
        if index < self.len {
            Ok(self.root.get(index))
        } else {
            err_at!(IndexFail, msg: "index {} out of bounds", index)?
        }
    }

    /// Insert an element at `off` position within the vector, or `IndexFail`
    /// error if out of bounds. Call this for copy-on-write insert, especially
    /// when `Vector` is shared among multiple owners. In cases of
    /// single-ownership use `insert_mut`, which does in-place mutation, for
    /// better performance.
    pub fn insert(&mut self, off: usize, value: T) -> Result<()>
    where
        T: Clone,
    {
        let (root, _) = if off <= self.len {
            let rn = Rebalance::new(self);
            self.root.insert(off, value, &rn)?
        } else {
            err_at!(IndexFail, msg: "index {} out of bounds", off)?
        };

        self.root = root;
        self.len += 1;

        Ok(())
    }

    /// Insert an element at `off` position within the vector, or `IndexFail`
    /// error if out of bounds. Call this for in-place insert and only when
    /// `Vector` is under single ownership. In cases of shared-ownership
    /// use `insert` api which does copy-on-write.
    pub fn insert_mut(&mut self, off: usize, value: T) -> Result<()>
    where
        T: Clone,
    {
        if off <= self.len {
            let rn = Rebalance::new(self);

            let depth = NodeRef::get_mut(&mut self.root)
                .unwrap()
                .insert_mut(off, value, &rn)?;

            let (root, _) = Node::auto_rebalance(NodeRef::clone(&self.root), depth, false, &rn)?;
            self.root = root;
        } else {
            err_at!(IndexFail, msg: "index {} out of bounds", off)?;
        };

        self.len += 1;

        Ok(())
    }

    /// Update the element at `off` position within the vector, or `IndexFail`
    /// error if out of bounds. Call this for copy-on-write update, especially
    /// when `Vector` is shared among multiple owners. In cases of
    /// single-ownership use `update_mut`, which does in-place mutation, for
    /// better performance.
    pub fn update(&mut self, off: usize, value: T) -> Result<T>
    where
        T: Clone,
    {
        let (root, val) = if off < self.len {
            self.root.update(off, value)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        self.root = root;
        Ok(val)
    }

    /// Update an element at `off` position within the vector, or `IndexFail`
    /// error if out of bounds. Call this for in-place update and only when
    /// `Vector` is under single ownership. In cases of shared-ownership
    /// use `update` api which does copy-on-write.
    pub fn update_mut(&mut self, off: usize, value: T) -> Result<T>
    where
        T: Clone,
    {
        if off < self.len {
            Ok(NodeRef::get_mut(&mut self.root)
                .unwrap()
                .update_mut(off, value))
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)
        }
    }

    /// Remove and return the element at `off` position within the vector,
    /// or `IndexFail` error if out of bounds. Call this for copy-on-write
    /// remove, especially when `Vector` is shared among multiple owners.
    /// In cases of single-ownership use `remove_mut`, which does in-place
    /// mutation, for better performance.
    pub fn remove(&mut self, off: usize) -> Result<T>
    where
        T: Clone,
    {
        let (root, val) = if off < self.len {
            self.root.remove(off)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        self.root = root;
        self.len -= 1;
        Ok(val)
    }

    /// Remove and return the element at `off` position within the vector,
    /// or `IndexFail` error if out of bounds. Call this for in-place update
    /// and only when `Vector` is under single ownership. In cases of
    /// shared-ownership use `remove` api which does copy-on-write.
    pub fn remove_mut(&mut self, off: usize) -> Result<T>
    where
        T: Clone,
    {
        let val = if off < self.len {
            NodeRef::get_mut(&mut self.root).unwrap().remove_mut(off)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        self.len -= 1;
        Ok(val)
    }

    /// Return an iterator over each element in Vector.
    pub fn iter(&self) -> Iter<T> {
        Iter::new(&self.root)
    }

    /// When auto-rebalance is disabled, use this method to rebalance the tree.
    pub fn rebalance(&self) -> Result<Self> {
        let rn = Rebalance::new(self);
        let root = NodeRef::clone(&self.root);
        let (root, _) = Node::auto_rebalance(root, 0, true, &rn)?;
        let val = Vector {
            len: self.len,
            root,
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        };
        Ok(val)
    }

    // return only nodes that is referenced in multiple-versions. and
    // the total number of nodes in the tree.
    #[cfg(feature = "fuzzing")]
    pub fn fetch_multiversions(&self) -> (Vec<*const u8>, usize) {
        assert_eq!(strong_count(&self.root), 1);

        let mut acc = vec![];
        let n = self.root.fetch_multiversions(&mut acc);
        (acc, n)
    }
}

enum Node<T>
where
    T: Sized,
{
    M {
        weight: usize,
        left: NodeRef<T>,
        right: NodeRef<T>,
    },
    Z {
        data: Vec<T>,
    },
}

impl<'a, T> From<&'a [T]> for Node<T>
where
    T: Clone,
{
    fn from(val: &'a [T]) -> Self {
        Node::Z { data: val.to_vec() }
    }
}

impl<T> Node<T>
where
    T: Sized,
{
    fn newm(left: NodeRef<T>, right: NodeRef<T>, weight: usize) -> NodeRef<T> {
        NodeRef::new(Node::M {
            left,
            right,
            weight,
        })
    }

    fn empty_leaf() -> NodeRef<T> {
        NodeRef::new(Node::Z {
            data: Vec::default(),
        })
    }

    fn len(&self) -> usize {
        match self {
            Node::M { weight, right, .. } => weight + right.len(),
            Node::Z { data } => data.len(),
        }
    }

    fn footprint(&self) -> usize {
        let n = mem::size_of_val(self);
        n + match self {
            Node::Z { data } => data.capacity() * mem::size_of::<T>(),
            Node::M { left, right, .. } => left.footprint() + right.footprint(),
        }
    }

    fn get(&self, off: usize) -> &T {
        match self {
            Node::M { weight, left, .. } if off < *weight => left.get(off),
            Node::M { weight, right, .. } => right.get(off - *weight),
            Node::Z { data } => &data[off],
        }
    }

    // return (value, max_depth)
    fn insert(&self, off: usize, val: T, rn: &Rebalance) -> Result<(NodeRef<T>, usize)>
    where
        T: Clone,
    {
        let (node, depth) = match self {
            Node::M {
                weight,
                left,
                right,
            } => {
                let weight = *weight;
                let (weight, left, right, depth) = if off < weight {
                    let (left, depth) = left.insert(off, val, rn)?;
                    (weight + 1, left, NodeRef::clone(right), depth)
                } else {
                    let off = off - weight;
                    let (right, depth) = right.insert(off, val, rn)?;
                    (weight, NodeRef::clone(left), right, depth)
                };
                (Node::newm(left, right, weight), depth + 1)
            }
            Node::Z { data } if data.len() < max_leaf_items::<T>(rn.leaf_cap) => {
                let mut ndata = data[..off].to_vec();
                ndata.push(val);
                ndata.extend_from_slice(&data[off..]);
                (NodeRef::new(Node::Z { data: ndata }), 1)
            }
            Node::Z { data } => (Self::split_insert(data, off, val), 2),
        };

        let (node, depth) = Node::auto_rebalance(node, depth, false, rn)?;

        Ok((node, depth))
    }

    fn insert_mut(&mut self, off: usize, val: T, rn: &Rebalance) -> Result<usize>
    where
        T: Clone,
    {
        let depth = match self {
            Node::M {
                weight,
                left,
                right,
            } => {
                if off < *weight {
                    let depth = NodeRef::get_mut(left).unwrap().insert_mut(off, val, rn)?;
                    *weight += 1;
                    depth
                } else {
                    let off = off - *weight;
                    NodeRef::get_mut(right).unwrap().insert_mut(off, val, rn)?
                }
            }
            Node::Z { data } if data.len() < max_leaf_items::<T>(rn.leaf_cap) => {
                data.insert(off, val);
                1
            }
            Node::Z { data } => {
                *self = NodeRef::try_unwrap(Self::split_insert(data, off, val))
                    .ok()
                    .unwrap();
                2
            }
        };
        Ok(depth)
    }

    fn update(&self, off: usize, value: T) -> (NodeRef<T>, T)
    where
        T: Clone,
    {
        match self {
            Node::M {
                weight,
                left,
                right,
            } if off < *weight => {
                let (left, old) = left.update(off, value);
                (Node::newm(left, NodeRef::clone(right), *weight), old)
            }
            Node::M {
                weight,
                left,
                right,
            } => {
                let (right, old) = right.update(off - *weight, value);
                (Node::newm(NodeRef::clone(left), right, *weight), old)
            }
            Node::Z { data } => {
                let old = data[off].clone();

                let mut data = data.to_vec();
                data[off] = value;
                (NodeRef::new(Node::Z { data }), old)
            }
        }
    }

    fn update_mut(&mut self, off: usize, value: T) -> T
    where
        T: Clone,
    {
        match self {
            Node::M { weight, left, .. } if off < *weight => {
                NodeRef::get_mut(left).unwrap().update_mut(off, value)
            }
            Node::M { weight, right, .. } => NodeRef::get_mut(right)
                .unwrap()
                .update_mut(off - *weight, value),
            Node::Z { data } => {
                let old = data[off].clone();
                data[off] = value;
                old
            }
        }
    }

    fn remove(&self, off: usize) -> (NodeRef<T>, T)
    where
        T: Clone,
    {
        match self {
            Node::M {
                weight,
                left,
                right,
            } => {
                let weight = *weight;
                if off < weight {
                    let (left, old) = left.remove(off);
                    (Node::newm(left, NodeRef::clone(right), weight - 1), old)
                } else {
                    let (right, old) = right.remove(off - weight);
                    (Node::newm(NodeRef::clone(left), right, weight), old)
                }
            }
            Node::Z { data } => {
                let old = data[off].clone();

                let mut ndata = data[..off].to_vec();
                ndata.extend_from_slice(&data[(off + 1)..]);
                (NodeRef::new(Node::Z { data: ndata }), old)
            }
        }
    }

    fn remove_mut(&mut self, off: usize) -> T
    where
        T: Clone,
    {
        match self {
            Node::M {
                weight,
                left,
                right,
            } => {
                if off < *weight {
                    *weight -= 1;
                    NodeRef::get_mut(left).unwrap().remove_mut(off)
                } else {
                    NodeRef::get_mut(right).unwrap().remove_mut(off - *weight)
                }
            }
            Node::Z { data } => {
                let old = data[off].clone();
                data.remove(off);
                if (data.len() * 2) < data.capacity() {
                    data.shrink_to_fit()
                }
                old
            }
        }
    }

    fn split_insert(data: &[T], off: usize, val: T) -> NodeRef<T>
    where
        T: Clone,
    {
        let (mut ld, mut rd) = {
            let m = data.len() / 2;
            match data.len() {
                0 => (vec![], vec![]),
                1 => (data.to_vec(), vec![]),
                _ => (data[..m].to_vec(), data[m..].to_vec()),
            }
        };
        let weight = match ld.len() {
            w if off < w => {
                ld.insert(off, val);
                ld.len()
            }
            w => {
                rd.insert(off - w, val);
                w
            }
        };
        let left = NodeRef::new(Node::Z { data: ld });
        let right = NodeRef::new(Node::Z { data: rd });
        NodeRef::new(Node::M {
            weight,
            left,
            right,
        })
    }

    fn auto_rebalance(
        node: NodeRef<T>,
        depth: usize,
        force: bool,
        rn: &Rebalance,
    ) -> Result<(NodeRef<T>, usize)> {
        let doit = {
            let b = force;
            b || (rn.auto_rebalance == true) && rn.can_rebalance(depth)
        };

        match doit {
            false => Ok((node, depth)),
            true => {
                let zs = Self::collect_leaf_nodes(&node);
                let depth = (zs.len() as f64).log2().ceil() as usize;
                let mut iter = zs.into_iter();
                let item = iter.next();
                let (nroot, _) = Node::build_bottoms_up(depth, item, &mut iter);
                assert!(iter.next().is_none());
                Ok((nroot, depth))
            }
        }
    }

    fn collect_leaf_nodes(root: &NodeRef<T>) -> Vec<NodeRef<T>> {
        let (mut stack, mut acc) = (vec![], vec![]);
        let mut node = root;
        loop {
            match node.borrow() {
                Node::Z { .. } if stack.len() == 0 => {
                    acc.push(NodeRef::clone(&node));
                    break acc;
                }
                Node::Z { .. } => {
                    acc.push(NodeRef::clone(&node));
                    node = stack.pop().unwrap();
                }
                Node::M { left, right, .. } => {
                    stack.push(right);
                    node = left;
                }
            }
        }
    }

    fn build_bottoms_up(
        depth: usize,
        item: Option<NodeRef<T>>,
        ziter: &mut impl Iterator<Item = NodeRef<T>>,
    ) -> (NodeRef<T>, usize) {
        let (root, n) = match (depth, item) {
            (_, None) => (NodeRef::new(Node::Z { data: vec![] }), 0),
            (0, Some(l)) => {
                let n = l.len();
                (l, n)
            }
            (_, item) => {
                let (left, weight) = Self::build_bottoms_up(depth - 1, item, ziter);
                let (right, m) = Self::build_bottoms_up(depth - 1, ziter.next(), ziter);
                let node = Node::M {
                    weight,
                    left,
                    right,
                };
                (NodeRef::new(node), weight + m)
            }
        };

        (root, n)
    }

    fn build_iter_stack<'a, 'b>(node: &'a Node<T>, iter: &'b mut Iter<'a, T>) {
        match node {
            Node::M { left, right, .. } => {
                iter.stack.push(&right);
                Self::build_iter_stack(left, iter);
            }
            node @ Node::Z { .. } => {
                iter.node = Some(node);
            }
        }
    }

    fn build_into_iter_stack(node: &NodeRef<T>, iter: &mut IntoIter<T>) {
        match node.as_ref() {
            Node::M { left, right, .. } => {
                iter.stack.push(NodeRef::clone(right));
                Self::build_into_iter_stack(left, iter);
            }
            Node::Z { .. } => {
                iter.node = Some(NodeRef::clone(node));
            }
        }
    }

    // only used with src/bin/fuzzy program
    #[cfg(feature = "fuzzing")]
    fn fetch_multiversions(&self, acc: &mut Vec<*const u8>) -> usize {
        match self {
            Node::M { left, right, .. } => {
                if strong_count(left) > 1 {
                    acc.push(as_ptr(left));
                }
                let mut n = left.fetch_multiversions(acc);

                if strong_count(right) > 1 {
                    acc.push(as_ptr(right));
                }
                n += right.fetch_multiversions(acc);
                n + 1
            }
            Node::Z { .. } => 1,
        }
    }
}

struct Rebalance {
    n_leafs: f64,
    auto_rebalance: bool,
    leaf_cap: usize,
}

impl Rebalance {
    fn new<T: Sized>(r: &Vector<T>) -> Rebalance {
        let n_leafs = r.len / max_leaf_items::<T>(r.leaf_cap);
        Rebalance {
            n_leafs: n_leafs as f64,
            auto_rebalance: r.auto_rebalance,
            leaf_cap: r.leaf_cap,
        }
    }

    fn can_rebalance(&self, depth: usize) -> bool {
        match depth {
            n if n < 30 => false,
            _ if (depth as f64) > (self.n_leafs.log2() * 3_f64) => true,
            _ => false,
        }
    }
}

pub struct Iter<'a, T> {
    stack: Vec<&'a Node<T>>,
    node: Option<&'a Node<T>>,
    off: usize,
}

impl<'a, T> Iter<'a, T> {
    fn new(root: &'a Node<T>) -> Iter<'a, T> {
        let mut iter = Iter {
            stack: Vec::default(),
            node: None,
            off: 0,
        };
        Node::build_iter_stack(root, &mut iter);
        iter
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.node {
            Some(Node::Z { data }) if self.off < data.len() => {
                let item = &data[self.off];
                self.off += 1;
                Some(item)
            }
            Some(Node::Z { .. }) | None => match self.stack.pop() {
                Some(node) => {
                    self.off = 0;
                    Node::build_iter_stack(node, self);
                    self.next()
                }
                None => None,
            },
            Some(_) => unreachable!(),
        }
    }
}

pub struct IntoIter<T> {
    stack: Vec<NodeRef<T>>,
    node: Option<NodeRef<T>>,
    off: usize,
}

impl<T> Iterator for IntoIter<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match self.node.as_ref().map(|x| x.as_ref()) {
            Some(Node::Z { data }) if self.off < data.len() => {
                let item = data[self.off].clone();
                self.off += 1;
                Some(item)
            }
            Some(Node::Z { .. }) | None => match self.stack.pop() {
                Some(node) => {
                    self.off = 0;
                    Node::build_into_iter_stack(&node, self);
                    self.next()
                }
                None => None,
            },
            Some(_) => unreachable!(),
        }
    }
}

fn max_leaf_items<T>(cap: usize) -> usize {
    let s = mem::size_of::<T>();
    (cap / s) + 1
}

#[cfg(feature = "fuzzing")]
enum Op {
    //
}

#[cfg(test)]
#[path = "ppar_test.rs"]
mod ppar_test;
