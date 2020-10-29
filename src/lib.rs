//! Package implement persistent array using a variant of rope data structure.
//!
//! Fundamentally, it can be viewed as a binary-tree of array-blocks, where
//! each leaf-node is a block of contiguous item of type `T`, while intermediate
//! nodes only hold references to the child nodes, left and right.
//! To be more precise, intermediate nodes in the tree are organised similar
//! to rope structure, as a tuple of (weight, left, right) where weight is
//! the sum of all items present in the leaf-nodes under the left-branch.
//!
//! Thread Safety
//! =============
//!
//! By default `Vector<T>` is thread safe through `Arc`. To get better
//! performance over thread-safety, compile with `ppar-rc` feature, which uses
//! `Rc` instead of `Arc`.
//!
//! **Alternate libraries**:
//!
//! * _[im](https://github.com/bodil/im-rs)_
//! * _[rpds](https://github.com/orium/rpds)_

use std::{error, fmt, result};

#[macro_use]
mod util;
mod ppar;

pub use ppar::Vector;

/// Type alias for Result return type, used by this package.
pub type Result<T> = result::Result<T, Error>;

/// Error variants that can be returned by this package's API.
///
/// Each variant carries a prefix, typically identifying the
/// error location.
pub enum Error {
    IndexFail(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        use Error::*;

        match self {
            IndexFail(p, msg) => write!(f, "{} IndexFail: {}", p, msg),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl error::Error for Error {}
