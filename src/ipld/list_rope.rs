//! Module implement a variant of rope data structure.
//!
//! Expected to be used as list type in data-model.

// Calling this as [rope data-structure] might be grossly wrong, for
// there is neither a concat-op, nor a split-op. But it is largely
// inspired from rope.
//
// Fundamentally, it can be viewed as a binary-tree of array-blocks, where
// each leaf-node is a block of contiguous item of type T, while intermediate
// nodes only hold references to the child nodes, left and right.
// To be more precise, intermediate nodes in the tree are organised similar
// to rope structure, as a tuple of (weight, left, right) where weight is
// the sum of all items present in the leaf-nodes under the left-branch.

use std::{mem, rc::Rc};

use crate::{Error, Result};

const LEAF_CAP: usize = 1024; // in bytes.

pub struct Rope<T>
where
    T: Sized + Clone,
{
    len: usize,
    root: Rc<Node<T>>,
}

impl<T> Rope<T>
where
    T: Sized + Clone,
{
    pub fn new() -> Rope<T> {
        let root = Node::Z {
            data: Vec::default(),
        };
        Rope {
            len: 0,
            root: Rc::new(root),
        }
    }

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

    pub fn insert(&self, off: usize, value: T) -> Result<Rope<T>> {
        let (root, max_depth) = if off <= self.len {
            self.root.insert(off, value, 0 /*depth*/)?
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        let root = Self::try_rebalance(root, max_depth);
        let len = self.len + 1;

        Ok(Rope { root, len })
    }

    pub fn set(&self, off: usize, value: T) -> Result<Rope<T>> {
        let (root, max_depth) = if off < self.len {
            self.root.set(off, value, 0 /*depth*/)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        let root = Self::try_rebalance(root, max_depth);
        let len = self.len;

        Ok(Rope { root, len })
    }

    pub fn delete(&self, off: usize) -> Result<Rope<T>> {
        let (root, max_depth) = if off < self.len {
            self.root.delete(off, 0 /*depth*/)
        } else {
            err_at!(IndexFail, msg: "offset {} out of bounds", off)?
        };

        let root = Self::try_rebalance(root, max_depth);
        let len = self.len - 1;

        Ok(Rope { root, len })
    }

    pub fn calculate_len(&self) -> usize {
        self.root.len()
    }

    fn try_rebalance(root: Rc<Node<T>>, _max_depth: usize) -> Rc<Node<T>> {
        root // TODO
    }
}

enum Node<T>
where
    T: Sized + Clone,
{
    M {
        weight: usize,
        left: Rc<Node<T>>,
        right: Rc<Node<T>>,
    },
    Z {
        data: Vec<T>,
    },
}

impl<T> Node<T>
where
    T: Sized + Clone,
{
    fn newm(left: Rc<Node<T>>, right: Rc<Node<T>>, weight: usize) -> Rc<Node<T>> {
        Rc::new(Node::M {
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
            Node::Z { data } => (data.capacity() * mem::size_of::<T>()),
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
    fn insert(&self, off: usize, val: T, depth: usize) -> Result<(Rc<Node<T>>, usize)> {
        let depth = depth + 1;

        let (node, max_depth) = match self {
            Node::M {
                weight,
                left,
                right,
            } => {
                let weight = *weight;
                //println!(
                //    "{} {} lenl:{} lenr:{}",
                //    weight,
                //    off,
                //    left.len(),
                //    right.len()
                //);
                let (weight, left, right, max_depth) = if off < weight {
                    let (left, max_depth) = left.insert(off, val, depth)?;
                    (weight + 1, left, Rc::clone(right), max_depth)
                } else {
                    let off = off - weight;
                    let (right, max_depth) = right.insert(off, val, depth)?;
                    (weight, Rc::clone(left), right, max_depth)
                };
                (Node::newm(left, right, weight), max_depth)
            }
            Node::Z { data } if data.len() < leaf_size::<T>(LEAF_CAP) => {
                let mut data = data.to_vec();
                data.insert(off, val);
                (Rc::new(Node::Z { data }), depth)
            }
            Node::Z { data } => (Self::split_insert(data, off, val), depth),
        };

        Ok((node, max_depth))
    }

    fn set(&self, off: usize, value: T, depth: usize) -> (Rc<Node<T>>, usize) {
        let depth = depth + 1;
        match self {
            Node::M {
                weight,
                left,
                right,
            } if off < *weight => {
                let (left, max_depth) = left.set(off, value, depth);
                (Node::newm(left, Rc::clone(right), *weight), max_depth)
            }
            Node::M {
                weight,
                left,
                right,
            } => {
                let (right, max_depth) = right.set(off - *weight, value, depth);
                (Node::newm(Rc::clone(left), right, *weight), max_depth)
            }
            Node::Z { data } => {
                let mut data = data.to_vec();
                data[off] = value;
                (Rc::new(Node::Z { data }), depth)
            }
        }
    }

    fn delete(&self, off: usize, depth: usize) -> (Rc<Node<T>>, usize) {
        let depth = depth + 1;
        match self {
            Node::M {
                weight,
                left,
                right,
            } if off < *weight => {
                let (left, max_depth) = left.delete(off, depth);
                (Node::newm(left, Rc::clone(right), *weight), max_depth)
            }
            Node::M {
                weight,
                left,
                right,
            } => {
                let (right, max_depth) = right.delete(off - *weight, depth);
                (Node::newm(Rc::clone(left), right, *weight), max_depth)
            }
            Node::Z { data } => {
                let mut data = data.to_vec();
                data.remove(off);
                (Rc::new(Node::Z { data }), depth)
            }
        }
    }

    fn split_insert(data: &[T], off: usize, val: T) -> Rc<Node<T>> {
        let n = leaf_size::<T>(LEAF_CAP);
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
        let left = Rc::new(Node::Z { data: ld });
        let right = Rc::new(Node::Z { data: rd });
        Rc::new(Node::M {
            weight,
            left,
            right,
        })
    }
}

fn leaf_size<T>(cap: usize) -> usize {
    let s = mem::size_of::<T>();
    (cap / s) + 1
}

#[cfg(test)]
#[path = "rope_test.rs"]
mod rope_test;
