//! Module implement thread-safe persistent array.

use std::sync::Arc as Ref;

#[path = "./ppar.rs"]
mod ppar;

/// Persistent array, thread-safe version.
pub use self::ppar::{IntoIter, Iter, Vector};
#[cfg(test)]
pub use ppar::validate;

impl<T> Vector<T>
where
    T: Clone,
{
    /// Return whether this instance is thread-safe.
    pub fn is_thread_safe(&self) -> bool {
        true
    }

    #[cfg(test)]
    pub fn is_rc_type() -> bool {
        false
    }
}
