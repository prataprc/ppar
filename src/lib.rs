//! Package implement persistent array using a variant of rope data structure.
//!
//! Fundamentally, it can be viewed as a binary-tree of array-blocks, where
//! each leaf-node is a block of contiguous item of type `T`, while intermediate
//! nodes only hold references to the child nodes, left and right.
//! To be more precise, intermediate nodes in the tree are organised similar
//! to rope structure, as a tuple of (*weight*, *left*, *right*) where weight
//! is the sum of all items present in the leaf-nodes under the left-branch.
//!
//! Here is a quick list of situation that might require using `ppar`.
//!
//! * When array size is too large with repeated insert and remove operation.
//! * When shared ownership is required.
//! * When shared ownership across concurrent threads.
//! * To support undo/redo operation for array modifications.
//! * When splitting up of array and/or joining arrays are frequently done.
//! * Lazy clone of array using copy-on-write.
//!
//! Ownership and Cloning
//! =====================
//!
//! Cloning `arc::Vector` and `rc::Vector` is cheap, it creates a shared ownership
//! of the underlying tree. This is great for applications requiring
//! shared-ownership, but at the cost of copy-on-write for every mutation in
//! the Vector, like insert, remove, delete. For applications requiring only
//! single-ownership insert_mut, remove_mut, delete_mut gives better
//! performance because the underlying tree is mutated in-place. To help decide
//! what method to use when, methods that perform in-place mutation are
//! suffixed with `_mut`.
//!
//! Thread Safety
//! =============
//!
//! `arc::Vector<T>` is thread safe through `Arc`. To trade-off
//! thread-safety for performance use `rc::Vector` type, which is same as
//! `arc::Vector` type except for using `std::rc::Rc` instead of
//! `std::sync::Arc` for shared ownership. That is, `Send` and `Sync`
//! traits are not available for `rc::Vector` type while it is available
//! for `arc::Vector` type.
//!
//! **Alternate libraries**:
//!
//! * _[im](https://github.com/bodil/im-rs)_
//! * _[rpds](https://github.com/orium/rpds)_

use std::{error, fmt, result};

/// Short form to compose Error values.
///
/// Here are few possible ways:
///
/// ```ignore
/// use crate::Error;
/// err_at!(Invalid, msg: format!("bad argument"));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(Invalid, std::io::read(buf));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(Invalid, std::fs::read(file_path), format!("read failed"));
/// ```
///
macro_rules! err_at {
    ($v:ident, msg: $($arg:expr),+) => {{
        let prefix = format!("{}:{}", file!(), line!());
        Err(Error::$v(prefix, format!($($arg),+)))
    }};
    ($v:ident, $e:expr) => {{
        match $e {
            Ok(val) => Ok(val),
            Err(err) => {
                let prefix = format!("{}:{}", file!(), line!());
                Err(Error::$v(prefix, format!("{}", err)))
            }
        }
    }};
    ($v:ident, $e:expr, $($arg:expr),+) => {{
        match $e {
            Ok(val) => Ok(val),
            Err(err) => {
                let prefix = format!("{}:{}", file!(), line!());
                let msg = format!($($arg),+);
                Err(Error::$v(prefix, format!("{} {}", err, msg)));
            }
        }
    }};
}

pub mod arc;
pub mod rc;

/// Leaf node shall not exceed this default size.
///
/// Refer `Vector::set_leaf_size` for optimal configuration.
pub const LEAF_CAP: usize = 10 * 1024; // in bytes.

/// Threshold on tree depth, beyond which auto-rebalance will kick in.
pub const REBALANCE_THRESHOLD: usize = 30;

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
