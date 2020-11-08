//! Module implement thread-safe persistent array.

use std::sync::Arc;

type NodeRef<T> = Arc<Node<T>>;

impl<T> Vector<T>
where
    T: Clone,
{
    /// Return whether this instance is thread-safe.
    pub fn is_thread_safe(&self) -> bool {
        true
    }
}

#[cfg(feature = "fuzzy")]
fn strong_count<T: Clone>(node: &NodeRef<T>) -> usize {
    Arc::strong_count(node)
}

#[cfg(feature = "fuzzy")]
fn as_ptr<T: Clone>(node: &NodeRef<T>) -> *const u8 {
    Arc::as_ptr(node) as *const u8
}

include!("./ppar.rs");
