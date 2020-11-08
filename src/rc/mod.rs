//! Module implement persistent array, faster but not thread safe.

use std::rc::Rc;

type NodeRef<T> = Rc<Node<T>>;

impl<T> Vector<T>
where
    T: Clone,
{
    /// Return whether this instance is thread-safe.
    pub fn is_thread_safe(&self) -> bool {
        false
    }
}

#[cfg(feature = "fuzzy")]
fn strong_count<T: Clone>(node: &NodeRef<T>) -> usize {
    Rc::strong_count(node)
}

#[cfg(feature = "fuzzy")]
fn as_ptr<T: Clone>(node: &NodeRef<T>) -> *const u8 {
    Rc::as_ptr(node) as *const u8
}

include!("../ppar.rs");
