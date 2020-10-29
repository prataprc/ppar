//! Module implement persistent array using a variant of rope data structure.
//!
//! Fundamentally, it can be viewed as a binary-tree of array-blocks, where
//! each leaf-node is a block of contiguous item of type T, while intermediate
//! nodes only hold references to the child nodes, left and right.
//! To be more precise, intermediate nodes in the tree are organised similar
//! to rope structure, as a tuple of (weight, left, right) where weight is
//! the sum of all items present in the leaf-nodes under the left-branch.
//!
//! **Alternates libraries**:
//!
//! im: https://github.com/bodil/im-rs
//! rpds: https://github.com/orium/rpds

#[allow(unused_imports)]
use log::debug;

#[cfg(feature = "ppar-rc")]
use std::rc::Rc;
#[cfg(not(feature = "ppar-rc"))]
use std::sync::Arc;
use std::{borrow::Borrow, mem};

use crate::{Error, Result};

/// Leaf not shall not exceed this default size, refer
/// [Vector::set_leaf_size] for optimal configuration.
pub const LEAF_CAP: usize = 10 * 1024; // in bytes.

#[cfg(feature = "ppar-rc")]
type NodeRef<T> = Rc<Node<T>>;

#[cfg(not(feature = "ppar-rc"))]
type NodeRef<T> = Arc<Node<T>>;

/// Persistent array, that can also be used as mutable vector.
///
/// Use [mod@std::vec] when only single threaded mutable vector is needed.
pub struct Vector<T>
where
    T: Sized + Clone,
{
    len: usize,
    root: NodeRef<T>,
    auto_rebalance: bool,
    leaf_cap: usize,
}

impl<T: Clone> Clone for Vector<T> {
    fn clone(&self) -> Self {
        Vector {
            len: self.len,
            root: NodeRef::clone(&self.root),
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        }
    }
}

impl<T> Vector<T>
where
    T: Sized + Clone,
{
    pub fn new() -> Self {
        let root = Node::Z {
            data: Vec::default(),
        };
        Vector {
            len: 0,
            root: NodeRef::new(root),
            auto_rebalance: true,
            leaf_cap: LEAF_CAP,
        }
    }

    pub fn from_slice(slice: &[T], leaf_node_size: Option<usize>) -> Self {
        let n = leaf_size::<T>(leaf_node_size.unwrap_or(LEAF_CAP));

        let (root, _) = {
            let zs: Vec<NodeRef<T>> = slice
                .chunks(n)
                .map(|x| NodeRef::new(Node::Z { data: x.to_vec() }))
                .collect();
            let depth = ((zs.len() as f64).log2() as usize) + 1;
            let mut iter = zs.into_iter();
            let item = iter.next();
            Node::build_bottoms_up(depth, item, &mut iter)
        };
        Vector {
            len: slice.len(),
            root,
            auto_rebalance: true,
            leaf_cap: leaf_node_size.unwrap_or(LEAF_CAP),
        }
    }

    /// Size of the leaf node can be adjusted. Note that all leaf nodes
    /// shall be of equal size set by `leaf_size`. Setting a large value will
    /// make the tree shallow giving better read performance, at the expense
    /// of write performance. Leaf size must be specified in bytes.
    pub fn set_leaf_size(&mut self, leaf_size: usize) -> &mut Self {
        self.leaf_cap = leaf_size;
        self
    }

    /// Auto rebalance is enabled by default. This has some penalty for write
    /// heavy situations, since every write op will try to rebalance the tree
    /// if goes too much off-balance. Application can disable auto-rebalance
    /// to get maximum efficiency, and call [Self::rebalance] method as and
    /// when required.
    pub fn set_auto_rebalance(&mut self, rebalance: bool) -> &mut Self {
        self.auto_rebalance = rebalance;
        self
    }
}

impl<T> Vector<T>
where
    T: Sized + Clone,
{
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn footprint(&self) -> usize {
        mem::size_of_val(self) + self.root.footprint()
    }

    pub fn get(&self, index: usize) -> Result<&T> {
        let val = if index < self.len {
            self.root.get(index)
        } else {
            err_at!(IndexFail, msg: "index {} out of bounds", index)?
        };

        Ok(val)
    }

    pub fn insert(&self, off: usize, value: T) -> Result<Self> {
        let rn = Rebalance::new(self);
        let (root, _) = if off <= self.len {
            self.root.insert(off, value, &rn)?
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        Ok(Vector {
            root,
            len: self.len + 1,
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        })
    }

    pub fn set(&self, off: usize, value: T) -> Result<Self> {
        let root = if off < self.len {
            self.root.set(off, value)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        Ok(Vector {
            root,
            len: self.len,
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        })
    }

    pub fn delete(&self, off: usize) -> Result<Self> {
        let root = if off < self.len {
            self.root.delete(off)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        Ok(Vector {
            root,
            len: self.len - 1,
            auto_rebalance: self.auto_rebalance,
            leaf_cap: self.leaf_cap,
        })
    }

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
}

enum Node<T>
where
    T: Sized + Clone,
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

impl<T> Node<T>
where
    T: Sized + Clone,
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
    fn insert(&self, off: usize, val: T, rn: &Rebalance) -> Result<(NodeRef<T>, usize)> {
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
            Node::Z { data } if data.len() < leaf_size::<T>(rn.leaf_cap) => {
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

    fn set(&self, off: usize, value: T) -> NodeRef<T> {
        match self {
            Node::M {
                weight,
                left,
                right,
            } if off < *weight => {
                let left = left.set(off, value);
                Node::newm(left, NodeRef::clone(right), *weight)
            }
            Node::M {
                weight,
                left,
                right,
            } => {
                let right = right.set(off - *weight, value);
                Node::newm(NodeRef::clone(left), right, *weight)
            }
            Node::Z { data } => {
                let mut data = data.to_vec();
                data[off] = value;
                NodeRef::new(Node::Z { data })
            }
        }
    }

    fn delete(&self, off: usize) -> NodeRef<T> {
        match self {
            Node::M {
                weight,
                left,
                right,
            } => {
                //println!(
                //    "{} {} lenl:{} lenr:{}",
                //    weight,
                //    off,
                //    left.len(),
                //    right.len()
                //);
                let weight = *weight;
                if off < weight {
                    let left = left.delete(off);
                    Node::newm(left, NodeRef::clone(right), weight - 1)
                } else {
                    let right = right.delete(off - weight);
                    Node::newm(NodeRef::clone(left), right, weight)
                }
            }
            Node::Z { data } => {
                let mut ndata = data[..off].to_vec();
                ndata.extend_from_slice(&data[(off + 1)..]);
                NodeRef::new(Node::Z { data: ndata })
            }
        }
    }

    fn split_insert(data: &[T], off: usize, val: T) -> NodeRef<T> {
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

                debug!(
                    target: "ppar",
                    "rebalanced {} leaf nodes, depth:{:?}",
                    zs.len(),
                    depth
                );

                let depth = ((zs.len() as f64).log2() as usize) + 1;
                let (nroot, _) = {
                    let mut iter = zs.into_iter();
                    let item = iter.next();
                    Node::build_bottoms_up(depth, item, &mut iter)
                };

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
        match (depth, item) {
            (1, Some(l)) => {
                let weight = l.len();
                let (n, left, right) = match ziter.next() {
                    Some(r) => (l.len() + r.len(), l, r),
                    None => (l.len(), l, NodeRef::new(Node::Z { data: vec![] })),
                };
                let node = Node::M {
                    weight,
                    left,
                    right,
                };
                (NodeRef::new(node), n)
            }
            (1, None) => (NodeRef::new(Node::Z { data: vec![] }), 0),
            (_, None) => (NodeRef::new(Node::Z { data: vec![] }), 0),
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
        }
    }
}

fn leaf_size<T>(cap: usize) -> usize {
    let s = mem::size_of::<T>();
    (cap / s) + 1
}

struct Rebalance {
    n_leafs: f64,
    auto_rebalance: bool,
    leaf_cap: usize,
}

impl Rebalance {
    fn new<T: Sized + Clone>(r: &Vector<T>) -> Self {
        let n_leafs = r.len / leaf_size::<T>(r.leaf_cap);
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

#[cfg(test)]
#[path = "ppar_test.rs"]
mod ppar_test;
