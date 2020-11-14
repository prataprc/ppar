use std::{borrow::Borrow, mem};

use super::*;
use crate::{Error, Result};

/// Persistent array using rope-data-structure.
pub struct Vector<T>
where
    T: Sized,
{
    len: usize,
    root: Ref<Node<T>>,
    auto_rebalance: bool,
    leaf_cap: usize,
}

impl<T> Clone for Vector<T> {
    fn clone(&self) -> Vector<T> {
        Vector {
            len: self.len,
            root: Ref::clone(&self.root),
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        }
    }
}

impl<T> From<Vector<T>> for Vec<T>
where
    T: Clone,
{
    fn from(val: Vector<T>) -> Vec<T> {
        let mut arr = vec![];

        let root = Ref::clone(&val.root);
        for leaf in Node::collect_leaf_nodes(root, false, val.leaf_cap) {
            match leaf.borrow() {
                Node::Z { data } => arr.extend_from_slice(data),
                _ => unreachable!(),
            }
        }

        arr
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

        let mut leafs: Vec<Ref<Node<T>>> =
            slice.chunks(n).map(|x| Ref::new(Node::from(x))).collect();
        leafs.reverse();

        let depth = (leafs.len() as f64).log2().ceil() as usize;
        let (root, _) = Node::build_bottoms_up(depth, &mut leafs);
        assert!(leafs.len() == 0);

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
    ///
    /// **causes panic when used under shared-ownership**
    pub fn insert_mut(&mut self, off: usize, value: T) -> Result<()>
    where
        T: Clone,
    {
        if off <= self.len {
            let rn = Rebalance::new(self);

            let depth = Ref::get_mut(&mut self.root)
                .unwrap()
                .insert_mut(off, value, &rn)?;

            let packed = false;
            let force = false;
            let (root, _) =
                Node::auto_rebalance(Ref::clone(&self.root), depth, packed, force, &rn)?;

            self.root = root;
            self.len += 1;
            Ok(())
        } else {
            err_at!(IndexFail, msg: "index {} out of bounds", off)?
        }
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
    ///
    /// **causes panic when used under shared-ownership**
    pub fn update_mut(&mut self, off: usize, value: T) -> Result<T>
    where
        T: Clone,
    {
        if off < self.len {
            Ok(Ref::get_mut(&mut self.root).unwrap().update_mut(off, value))
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
    ///
    /// **causes panic when used under shared-ownership**
    pub fn remove_mut(&mut self, off: usize) -> Result<T>
    where
        T: Clone,
    {
        let val = if off < self.len {
            Ref::get_mut(&mut self.root).unwrap().remove_mut(off)
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

    /// Splits the collection into two at the given index.
    ///
    /// Returns a new Vector containing the elements in the range [at, len).
    /// After the call, the original vector will be left containing the
    /// elements [0, at) with its previous capacity unchanged.
    ///
    /// Optionally, application can call [Self::rebalance] on `self` and
    /// the returned vector to make the vectors fully balanced.
    pub fn split_off(&mut self, off: usize) -> Result<Vector<T>>
    where
        T: Clone,
    {
        if off > self.len {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)
        } else if off == self.len {
            Ok(Vector {
                len: 0,
                root: Node::empty_leaf(),
                auto_rebalance: self.auto_rebalance,
                leaf_cap: self.leaf_cap,
            })
        } else {
            let (node, root, n) = self.root.split_off(off, self.len);
            self.root = node;
            self.len -= n;
            Ok(Vector {
                len: n,
                root,
                auto_rebalance: self.auto_rebalance,
                leaf_cap: self.leaf_cap,
            })
        }
    }

    /// Join `other` Vector into this vector.
    ///
    /// Call [Self::rebalance] on `self` to make the vectors fully balanced.
    pub fn append(&mut self, other: Vector<T>)
    where
        T: Clone,
    {
        let other = if other.leaf_cap != self.leaf_cap {
            println!("append long");
            let arr: Vec<T> = other.into();
            Vector::from_slice(&arr, Some(self.leaf_cap))
        } else {
            other
        };

        let root = {
            let left = Ref::clone(&self.root);
            let right = Ref::clone(&other.root);
            Node::newm(left, right, self.len)
        };
        self.root = root;
        self.len += other.len;
    }

    /// When auto-rebalance is disabled, use this method to rebalance the tree.
    /// Calling it with `packed` as true will make sure that the leaf nodes
    /// are fully packed when rebuilding the tree.
    pub fn rebalance(&self, packed: bool) -> Result<Self>
    where
        T: Clone,
    {
        let rn = Rebalance::new(self);
        let root = Ref::clone(&self.root);
        let (root, _depth) = Node::auto_rebalance(root, 0, packed, true, &rn)?;
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
        let mut acc = vec![];
        let n = self.root.fetch_multiversions(&mut acc);
        (acc, n)
    }

    #[cfg(any(test, feature = "fuzzing"))]
    #[allow(dead_code)]
    pub fn pretty_print(&self) {
        self.root.pretty_print("".to_string(), self.len)
    }
}

enum Node<T>
where
    T: Sized,
{
    M {
        weight: usize,
        left: Ref<Node<T>>,
        right: Ref<Node<T>>,
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
    fn newm(left: Ref<Node<T>>, right: Ref<Node<T>>, weight: usize) -> Ref<Node<T>> {
        Ref::new(Node::M {
            left,
            right,
            weight,
        })
    }

    fn empty_leaf() -> Ref<Node<T>> {
        Ref::new(Node::Z {
            data: Vec::default(),
        })
    }

    fn len(&self) -> usize {
        match self {
            Node::M { weight, right, .. } => weight + right.len(),
            Node::Z { data } => data.len(),
        }
    }

    fn cow(&self) -> Node<T>
    where
        T: Clone,
    {
        match self {
            Node::Z { data } => Node::Z {
                data: data.to_vec(),
            },
            _ => unreachable!(),
        }
    }

    fn pack(&mut self, other: &Self, cap: usize) -> Option<Self>
    where
        T: Clone,
    {
        use std::cmp::min;

        match (self, other) {
            (Node::Z { data }, Node::Z { data: other }) => {
                let other = if data.len() < cap {
                    let n = min(cap - data.len(), other.len());
                    data.extend_from_slice(&other[..n]);
                    &other[n..]
                } else {
                    other
                };
                if other.len() > 0 {
                    Some(Node::Z {
                        data: other.to_vec(),
                    })
                } else {
                    None
                }
            }
            (_, _) => unreachable!(),
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
    fn insert(&self, off: usize, val: T, rn: &Rebalance) -> Result<(Ref<Node<T>>, usize)>
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
                    (weight + 1, left, Ref::clone(right), depth)
                } else {
                    let off = off - weight;
                    let (right, depth) = right.insert(off, val, rn)?;
                    (weight, Ref::clone(left), right, depth)
                };
                (Node::newm(left, right, weight), depth + 1)
            }
            Node::Z { data } if data.len() < max_leaf_items::<T>(rn.leaf_cap) => {
                let mut ndata = data[..off].to_vec();
                ndata.push(val);
                ndata.extend_from_slice(&data[off..]);
                (Ref::new(Node::Z { data: ndata }), 1)
            }
            Node::Z { data } => (Self::split_insert(data, off, val), 2),
        };

        let (node, depth) = Node::auto_rebalance(node, depth, false, false, rn)?;

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
                    let depth = Ref::get_mut(left).unwrap().insert_mut(off, val, rn)?;
                    *weight += 1;
                    depth
                } else {
                    let off = off - *weight;
                    Ref::get_mut(right).unwrap().insert_mut(off, val, rn)?
                }
            }
            Node::Z { data } if data.len() < max_leaf_items::<T>(rn.leaf_cap) => {
                data.insert(off, val);
                1
            }
            Node::Z { data } => {
                *self = Ref::try_unwrap(Self::split_insert(data, off, val))
                    .ok()
                    .unwrap();
                2
            }
        };
        Ok(depth)
    }

    fn update(&self, off: usize, value: T) -> (Ref<Node<T>>, T)
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
                (Node::newm(left, Ref::clone(right), *weight), old)
            }
            Node::M {
                weight,
                left,
                right,
            } => {
                let (right, old) = right.update(off - *weight, value);
                (Node::newm(Ref::clone(left), right, *weight), old)
            }
            Node::Z { data } => {
                let old = data[off].clone();

                let mut data = data.to_vec();
                data[off] = value;
                (Ref::new(Node::Z { data }), old)
            }
        }
    }

    fn update_mut(&mut self, off: usize, value: T) -> T
    where
        T: Clone,
    {
        match self {
            Node::M { weight, left, .. } if off < *weight => {
                Ref::get_mut(left).unwrap().update_mut(off, value)
            }
            Node::M { weight, right, .. } => Ref::get_mut(right)
                .unwrap()
                .update_mut(off - *weight, value),
            Node::Z { data } => {
                let old = data[off].clone();
                data[off] = value;
                old
            }
        }
    }

    fn remove(&self, off: usize) -> (Ref<Node<T>>, T)
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
                    (Node::newm(left, Ref::clone(right), weight - 1), old)
                } else {
                    let (right, old) = right.remove(off - weight);
                    (Node::newm(Ref::clone(left), right, weight), old)
                }
            }
            Node::Z { data } => {
                let old = data[off].clone();

                let mut ndata = data[..off].to_vec();
                ndata.extend_from_slice(&data[(off + 1)..]);
                (Ref::new(Node::Z { data: ndata }), old)
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
                    Ref::get_mut(left).unwrap().remove_mut(off)
                } else {
                    Ref::get_mut(right).unwrap().remove_mut(off - *weight)
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

    fn split_insert(data: &[T], off: usize, val: T) -> Ref<Node<T>>
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
        Ref::new(Node::M {
            weight,
            left: Ref::new(Node::Z { data: ld }),
            right: Ref::new(Node::Z { data: rd }),
        })
    }

    fn split_off(&self, off: usize, len: usize) -> (Ref<Node<T>>, Ref<Node<T>>, usize)
    where
        T: Clone,
    {
        match self {
            Node::M {
                left,
                right,
                weight,
            } if off < *weight => {
                let (left, root, n) = left.split_off(off, *weight);
                let root = Node::newm(root, Ref::clone(right), n);
                let node = Node::newm(left, Node::empty_leaf(), weight - n);
                (node, root, n + (len - weight))
            }
            Node::M {
                left,
                right,
                weight,
            } => {
                let (right, root, n) = right.split_off(off - weight, len - weight);
                let node = Node::newm(Ref::clone(left), right, *weight);
                (node, root, n)
            }
            Node::Z { data } if off == 0 => {
                let node = Node::empty_leaf();
                let root = Ref::new(Node::Z {
                    data: data.to_vec(),
                });
                (node, root, data.len())
            }
            Node::Z { data } => {
                let node = Ref::new(Node::Z {
                    data: data[..off].to_vec(),
                });
                let root = Ref::new(Node::Z {
                    data: data[off..].to_vec(),
                });
                (node, root, data[off..].len())
            }
        }
    }

    fn auto_rebalance(
        node: Ref<Node<T>>,
        depth: usize,
        packed: bool,
        force: bool,
        rn: &Rebalance,
    ) -> Result<(Ref<Node<T>>, usize)>
    where
        T: Clone,
    {
        let doit = force || (rn.auto_rebalance == true) && rn.can_rebalance(depth);

        match doit {
            false => Ok((node, depth)),
            true => {
                let mut leafs = Node::collect_leaf_nodes(node, packed, rn.leaf_cap);
                leafs.reverse();

                let depth = (leafs.len() as f64).log2().ceil() as usize;
                let (nroot, _) = Node::build_bottoms_up(depth, &mut leafs);
                assert!(leafs.len() == 0);

                Ok((nroot, depth))
            }
        }
    }

    fn collect_leaf_nodes(root: Ref<Node<T>>, packed: bool, leaf_cap: usize) -> Vec<Ref<Node<T>>>
    where
        T: Clone,
    {
        let (mut stack, mut acc) = (vec![], vec![]);
        let mut node = root;
        let leafs = loop {
            match node.borrow() {
                Node::Z { .. } if stack.len() == 0 => {
                    acc.push(Ref::clone(&node));
                    break acc;
                }
                Node::Z { .. } => {
                    acc.push(Ref::clone(&node));
                    node = stack.pop().unwrap();
                }
                Node::M { left, right, .. } => {
                    stack.push(Ref::clone(right));
                    node = Ref::clone(left);
                }
            }
        };

        if packed {
            let mut packed_leafs: Vec<Node<T>> = vec![];
            let cap = max_leaf_items::<T>(leaf_cap);
            for leaf in leafs.into_iter() {
                match packed_leafs.last_mut() {
                    None => packed_leafs.push(leaf.cow()),
                    Some(last) => match last.pack(leaf.borrow(), cap) {
                        Some(next) => packed_leafs.push(next),
                        None => (),
                    },
                }
            }
            packed_leafs.into_iter().map(Ref::new).collect()
        } else {
            leafs
        }
    }

    fn build_bottoms_up(depth: usize, leafs: &mut Vec<Ref<Node<T>>>) -> (Ref<Node<T>>, usize) {
        let (root, n) = match (depth, leafs.len()) {
            (0, 0) => (Ref::new(Node::Z { data: vec![] }), 0),
            (0, 1) | (1, 1) => {
                let node = leafs.pop().unwrap();
                let n = node.len();
                (node, n)
            }
            (1, n) if n >= 2 => {
                let (left, right) = (leafs.pop().unwrap(), leafs.pop().unwrap());

                let weight = left.len();
                let n = weight + right.len();

                let node = Node::M {
                    weight,
                    left,
                    right,
                };

                (Ref::new(node), n)
            }
            (_, 1) => Self::build_bottoms_up(1, leafs),
            (_, 2) => Self::build_bottoms_up(1, leafs),
            (depth, _) => {
                let (left, weight) = Self::build_bottoms_up(depth - 1, leafs);
                match leafs.len() {
                    0 => (left, weight),
                    1 => {
                        let right = leafs.pop().unwrap();
                        let m = right.len();
                        let node = Node::M {
                            weight,
                            left,
                            right,
                        };
                        (Ref::new(node), weight + m)
                    }
                    _ => {
                        let (right, m) = Self::build_bottoms_up(depth - 1, leafs);
                        let node = Node::M {
                            weight,
                            left,
                            right,
                        };
                        (Ref::new(node), weight + m)
                    }
                }
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

    fn build_into_iter_stack(node: &Ref<Node<T>>, iter: &mut IntoIter<T>) {
        match node.as_ref() {
            Node::M { left, right, .. } => {
                iter.stack.push(Ref::clone(right));
                Self::build_into_iter_stack(left, iter);
            }
            Node::Z { .. } => {
                iter.node = Some(Ref::clone(node));
            }
        }
    }

    // only used with src/bin/fuzzy program
    #[cfg(feature = "fuzzing")]
    fn fetch_multiversions(&self, acc: &mut Vec<*const u8>) -> usize {
        match self {
            Node::M { left, right, .. } => {
                if Ref::strong_count(left) > 1 {
                    let ptr = Ref::as_ptr(left);
                    acc.push(ptr as *const u8);
                }
                let mut n = left.fetch_multiversions(acc);

                if Ref::strong_count(right) > 1 {
                    let ptr = Ref::as_ptr(right);
                    acc.push(ptr as *const u8);
                }
                n += right.fetch_multiversions(acc);
                n + 1
            }
            Node::Z { .. } => 1,
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    #[allow(dead_code)]
    fn pretty_print(&self, mut prefix: String, len: usize) {
        match self {
            Node::M {
                left,
                right,
                weight,
            } => {
                println!("{}nodem:{}", prefix, len);
                prefix.push_str("  ");
                left.pretty_print(prefix.clone(), *weight);
                right.pretty_print(prefix, len - *weight);
            }
            Node::Z { data } => {
                println!("{}nodez:{}", prefix, data.len());
            }
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
            n if n < crate::REBALANCE_THRESHOLD => false,
            _ if (depth as f64) > (self.n_leafs.log2() * 3_f64) => true,
            _ => false,
        }
    }
}

/// An iterator for Vector.
///
/// Created by the iter method on Vector.
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

/// An iterator that moves elements out of Vector.
///
/// Created by the into_iter method on Vector (provided by the
/// IntoIterator trait).
pub struct IntoIter<T> {
    stack: Vec<Ref<Node<T>>>,
    node: Option<Ref<Node<T>>>,
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
    (cap / s) + if cap % s == 0 { 0 } else { 1 }
}

#[cfg(any(feature = "fuzzing", test))]
pub fn validate<T>(arr: &Vector<T>, refv: &[T])
where
    T: std::fmt::Debug + Clone + Eq + PartialEq,
{
    let k = std::mem::size_of::<T>();
    validate_mem_ratio(k, arr.footprint(), arr.len());

    assert_eq!(refv.len(), arr.len());
    assert_eq!(arr.len(), arr.root.len());

    for (off, val) in refv.iter().enumerate() {
        assert_eq!(arr.get(off).unwrap(), val, "off-{}", off);
    }

    assert!(arr.get(arr.len()).is_err());
}

#[cfg(any(feature = "fuzzing", test))]
pub fn validate_mem_ratio(k: usize, mem: usize, n: usize) {
    match n {
        0 => assert!(mem < 1000, "n:{} footp:{}", n, mem),
        n if n < 200 => {
            let cap = k * n * 3 + 1000;
            assert!(mem < cap, "n:{} footp:{}", n, mem)
        }
        n => {
            let k = k as f64;
            let ratio = ((((mem as f64) / (n as f64)) - k) / k) * 100.0;
            assert!(
                (ratio < 120.0) || (n < 100),
                "n:{} footp:{} ratio:{}",
                n,
                mem,
                ratio,
            );
        }
    }
}

#[cfg(test)]
#[path = "ppar_test.rs"]
mod ppar_test;
