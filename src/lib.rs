//! Atomic abstractions and thread-safe appendable list with lock-free iterators
//!
//! # Features
//!  - [`Atomic abstractions (Atomic, AtomicOption, FillOnceAtomicOption, FillOnceAtomicArc)`]
//!  - [`Thread-safe appendable list with a lock-free iterator (VoluntaryServitude - also called VS)`]
//!  - [`Serde serialization/deserialization ("serde-traits" feature)`]
//!  - [`par_extend, from_par_iter rayon implementation ("rayon-traits" feature)`]
//!  - [`Logging ("logs" feature)`]
//!
//!     You probably only need this if you are debugging this crate
//!
//! # Atomic abstractions
//!  - [`Atomic`] -> atomic `Box<T>`
//!  - [`AtomicOption`] -> atomic `Option<Box<T>>`
//!  - [`FillOnceAtomicOption`] -> atomic `Option<Box<T>>` that can give references (ideal for iterators)
//!  - [`FillOnceAtomicArc`] -> atomic `Option<Arc<T>>` with a limited API (like [`FillOnceAtomicOption`])
//!
//! With [`Atomic`] and [`AtomicOption`] it's not safe to get a reference, you must replace the value to access it.
//!
//! To safely get a reference of T you must use [`FillOnceAtomicOption`] and accept the API limitations (initially `None` but can be filled once).
//!
//! For a safe `AtomicArc` you must use some data-structure from `arc-swap`, `RwLock/Mutex` from `parking_lot` (or `std`, which is slower but the standard) or [`FillOnceAtomicArc`] and accept the limited API (2018).
//!
//! # Thread-safe appendable list that can create a lock-free iterator
//!  - [`VoluntaryServitude`] (also called [`VS`])
//!
//! # API of `VS` Iterator
//! - [`Iter`]
//!
//! # Logging
//!
//! *Setup logger according to `RUST_LOG` env var and `logs` feature*
//!
//! ## Enable the feature:
//!
//! **Cargo.toml**
//! ```toml
//! [dependencies]
//! voluntary_servitude = { version = "4", features = "logs" }
//! ```
//!
//! ## Set the `RUST_LOG` env var:
//!
//! ```bash
//! export RUST_LOG=voluntary_servitude=trace
//! export RUST_LOG=voluntary_servitude=debug
//! export RUST_LOG=voluntary_servitude=info
//! export RUST_LOG=voluntary_servitude=warn
//! export RUST_LOG=voluntary_servitude=error
//! ```
//!
//! ## Enable the logger using some setup (like env_logger)
//!
//! ```rust
//! env_logger::init();
//! // Call code to be logged
//! // ...
//! ```
//!
//! [`Atomic`]: ./atomics/struct.Atomic.html
//! [`AtomicOption`]: ./atomics/struct.AtomicOption.html
//! [`FillOnceAtomicOption`]: ./atomics/struct.FillOnceAtomicOption.html
//! [`FillOnceAtomicArc`]: ./atomics/struct.FillOnceAtomicArc.html
//! [`Atomic abstractions (Atomic, AtomicOption, FillOnceAtomicOption, FillOnceAtomicArc)`]: #atomic-abstractions
//! [`Thread-safe appendable list with a lock-free iterator (VoluntaryServitude - also called VS)`]: ./struct.VoluntaryServitude.html
//! [`Serde serialization/deserialization ("serde-traits" feature)`]: ./struct.VoluntaryServitude.html#impl-Serialize
//! [`&VS`]: ./struct.VoluntaryServitude.html#impl-Insertable<Tab>
//! [`&Iter`]: ./struct.Iter.html#impl-Insertable<Tab>
//! [`par_extend, from_par_iter rayon implementation ("rayon-traits" feature)`]: ./struct.VoluntaryServitude.html#impl-FromParallelIterator<T>
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html
//! [`Iter`]: ./struct.Iter.html
//! [`Logging ("logs" feature)`]: #logging

#![deny(
    missing_docs,
    missing_debug_implementations,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results,
    bad_style,
    const_err,
    dead_code,
    improper_ctypes,
    legacy_directory_ownership,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    plugin_as_library,
    private_in_public,
    safe_extern_statics,
    unconditional_recursion,
    unions_with_drop_fields,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
#![doc(html_root_url = "https://docs.rs/voluntary_servitude/4.0.4/voluntary-servitude")]
#![cfg_attr(docs_rs_workaround, feature(doc_cfg))]

/// Alias for [`voluntary_servitude`] macro
///
/// [`voluntary_servitude`]: ./macro.voluntary_servitude.html
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # env_logger::init();
/// use voluntary_servitude::VS;
/// let vs: VS<()> = vs![];
/// assert!(vs.is_empty());
///
/// let vs = vs![1, 2, 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
///
/// let vs = vs![1; 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1; 3]);
/// # let _ = vs![1, 2, 3,];
/// ```
#[macro_export]
macro_rules! vs {
    () => (voluntary_servitude![]);
    ($elem: expr; $n: expr) => (voluntary_servitude![$elem; $n]);
    ($($x: expr),+) => (voluntary_servitude![$($x),+]);
    ($($x: expr,)+) => (voluntary_servitude![$($x,)+]);
}

/// Creates new [`VS`] with specified elements as in the `vec!` macro
///
/// [`VS`]: ./type.VS.html
///
/// ```
/// # env_logger::init();
/// # #[macro_use] extern crate voluntary_servitude;
/// use voluntary_servitude::VS;
/// let vs: VS<()> = voluntary_servitude![];
/// assert!(vs.is_empty());
///
/// let vs = voluntary_servitude![1, 2, 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
///
/// let vs = voluntary_servitude![1; 3];
/// assert_eq!(vs.iter().collect::<Vec<_>>(), vec![&1; 3]);
/// # let _ = voluntary_servitude![1, 2, 3,];
/// ```
#[macro_export]
macro_rules! voluntary_servitude {
    () => ($crate::VS::default());
    ($elem: expr; $n: expr) => {{
        let vs = $crate::VS::default();
        for _ in 0..$n {
            vs.append($elem);
        }
        vs
    }};
    ($($x: expr),+) => (voluntary_servitude![$($x,)+]);
    ($($x: expr,)+) => {{
        let vs = $crate::VS::default();
        $(vs.append($x);)+
        vs
    }};
}

/// Remove logging macros when they are disabled (at compile time)
#[macro_use]
#[cfg(not(feature = "logs"))]
#[allow(unused)]
mod mock {
    macro_rules! trace(($($x:tt)*) => ());
    macro_rules! debug(($($x:tt)*) => ());
    macro_rules! info(($($x:tt)*) => ());
    macro_rules! warn(($($x:tt)*) => ());
    macro_rules! error(($($x:tt)*) => ());
}

pub mod atomics;
mod iterator;
mod node;
mod traits;
mod voluntary_servitude;

/// Simplify internal imports
#[allow(unused)]
mod prelude {
    pub(crate) use crate::atomics::{Atomic, AtomicOption, FillOnceAtomicOption};
    pub(crate) use crate::{IntoPtr, NotEmpty};
    pub(crate) use crate::{Iter, VoluntaryServitude, VS};
    #[cfg(feature = "logs")]
    pub use log::{debug, error, info, trace, warn};
}

use std::{error::Error, fmt, fmt::Debug, fmt::Display, fmt::Formatter};

/// Happens when you call `try_store` in a already filled [`AtomicOption`]/[`FillOnceAtomicOption`]/[`FillOnceAtomicArc`]
///
/// [`AtomicOption`]: ./atomics/struct.AtomicOption.html#method.try_store
/// [`FillOnceAtomicOption`]: ./atomics/struct.FillOnceAtomicOption.html#method.try_store
/// [`FillOnceAtomicArc`]: ./atomics/struct.FillOnceAtomicArc.html#method.try_store
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct NotEmpty;

impl Debug for NotEmpty {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "NotEmpty")
    }
}

impl Display for NotEmpty {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "not empty")
    }
}

impl Error for NotEmpty {}

pub use crate::iterator::Iter;
pub use crate::voluntary_servitude::{VoluntaryServitude, VS};

use std::ptr::null_mut;

/// Trait made to simplify conversion between smart pointers and raw pointers
pub(crate) trait IntoPtr<T> {
    /// Converts itself into a mutable pointer to it (leak or unwrap things)
    fn into_ptr(self) -> *mut T;
}

impl<T> IntoPtr<T> for T {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut Self {
        Box::into_raw(Box::new(self))
    }
}

impl<T> IntoPtr<T> for Option<T> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        self.map(Box::new).into_ptr()
    }
}

impl<T> IntoPtr<T> for Box<T> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        Self::into_raw(self)
    }
}

impl<T> IntoPtr<T> for Option<Box<T>> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        self.map_or(null_mut(), Box::into_raw)
    }
}

#[cfg(test)]
pub fn setup_logger() {
    use std::sync::Once;
    #[allow(unused)]
    static INITIALIZE: Once = Once::new();
    #[cfg(feature = "logs")]
    INITIALIZE.call_once(env_logger::init);
}
