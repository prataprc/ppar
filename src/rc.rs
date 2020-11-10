//! Module implement persistent array, faster but not thread safe.

use std::rc::Rc as Ref;

#[path = "./ppar.rs"]
mod ppar;

pub use self::ppar::Vector;
pub use self::ppar::*;

impl<T> Vector<T>
where
    T: Clone,
{
    /// Return whether this instance is thread-safe.
    pub fn is_thread_safe(&self) -> bool {
        false
    }
}
