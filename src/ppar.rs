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

#[cfg(any(feature = "arbitrary", feature = "fuzzing"))]
impl<T: arbitrary::Arbitrary> arbitrary::Arbitrary for Vector<T> {
    fn arbitrary(u: &mut Unstructured) -> Result<Self> {
        let k = std::mem::size_of::<T>();
        let leaf_cap = u.choose(&[k, k * 2, k * 100, k * 1000, k * 10000]).clone();
        let auto_reb = u.choose(&[true, false]).clone(); // auto_rebalance

        let mut arr = {
            let arr: Vec<T> = u.arbitrary();

            Vector::from_slice(&arr, Some(leaf_cap));
            arr.set_auto_rebalance(auto_reb);
            arr
        };

        Ok(arr)
    }
}

impl<T> Vector<T>
where
    T: Sized,
{
    /// Create a new empty Vector.
    pub fn new() -> Vector<T> {
        let root = Node::Z {
            data: Vec::default(),
        };
        Vector {
            len: 0,
            root: NodeRef::new(root),
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
        let val = if index < self.len {
            self.root.get(index)
        } else {
            err_at!(IndexFail, msg: "index {} out of bounds", index)?
        };

        Ok(val)
    }

    /// Insert an element at `off` position within the vector, or `IndexFail`
    /// error if out of bounds.
    pub fn insert(&mut self, off: usize, value: T) -> Result<()>
    where
        T: Clone,
    {
        let rn = Rebalance::new(self);
        let (root, _) = if off <= self.len {
            self.root.insert(off, value, &rn)?
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        self.root = root;
        self.len += 1;

        Ok(())
    }

    /// Update the element at `off` position within the vector, or `IndexFail`
    /// error if out of bounds.
    pub fn set(&mut self, off: usize, value: T) -> Result<T>
    where
        T: Clone,
    {
        let (root, val) = if off < self.len {
            self.root.set(off, value)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        self.root = root;
        Ok(val)
    }

    /// Remove and return the element at `off` position within the vector,
    /// or `IndexFail` error if out of bounds.
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
    #[cfg(feature = "fuzzy")]
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

    fn len(&self) -> usize {
        match self {
            Node::M { weight, right, .. } => weight + right.len(),
            Node::Z { data } => data.len(),
        }
    }

    fn footprint(&self) -> usize {
        let n = mem::size_of_val(self);
        n + match self {
            Node::Z { data } => {
                // println!("fp-leaf {} {}", data.len(), data.capacity());
                data.capacity() * mem::size_of::<T>()
            }
            Node::M { left, right, .. } => {
                // println!("fp-intr");
                left.footprint() + right.footprint()
            }
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

    fn set(&self, off: usize, value: T) -> (NodeRef<T>, T)
    where
        T: Clone,
    {
        match self {
            Node::M {
                weight,
                left,
                right,
            } if off < *weight => {
                let (left, old) = left.set(off, value);
                (Node::newm(left, NodeRef::clone(right), *weight), old)
            }
            Node::M {
                weight,
                left,
                right,
            } => {
                let (right, old) = right.set(off - *weight, value);
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
                let zs = Self::collect_zs(&node);
                let depth = (zs.len() as f64).log2().ceil() as usize;
                let mut iter = zs.into_iter();
                let item = iter.next();
                let (nroot, _) = Node::build_bottoms_up(depth, item, &mut iter);
                assert!(iter.next().is_none());
                Ok((nroot, depth))
            }
        }
    }

    fn collect_zs(root: &NodeRef<T>) -> Vec<NodeRef<T>> {
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

    // only used with src/bin/fuzzy program
    #[cfg(feature = "fuzzy")]
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

fn max_leaf_items<T>(cap: usize) -> usize {
    let s = mem::size_of::<T>();
    (cap / s) + 1
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

#[cfg(feature = "fuzzy")]
enum Op {
    //
}

#[cfg(test)]
#[path = "ppar_test.rs"]
mod ppar_test;
