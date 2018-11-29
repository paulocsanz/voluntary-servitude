//! Atomic abstractions and thread-safe appendable list with lock-free iterators
//!
//! # Features
//!  - [`Atomic abstractions (Atomic, AtomicOption, FillOnceAtomicOption, FillOnceAtomicArc)`]
//!  - [`Thread-safe appendable list with a lock-free iterator (VoluntaryServitude - also called VS)`]
//!  - [`Serde serialization/deserialization ("serde-traits" feature)`]
//!  - [`par_extend, from_par_iter rayon implementation ("rayon-traits" feature)`]
//!  - [`Use VoluntaryServitude from C (FFI) ("ffi" feature)`] (also in **./examples**)
//!  - [`Logging ("logs" feature)`]
//!
//! # Atomic abstractions
//!  - [`Atomic`] -> atomic `Box<T>`
//!  - [`AtomicOption`] -> atomic `Option<Box<T>>`
//!  - [`FillOnceAtomicOption`] -> atomic `Option<Box<T>>` that can give references (ideal for iterators)
//!  - [`FillOnceAtomicArc`] -> atomic `Option<Arc<T>>` with a limited Api (like [`FillOnceAtomicOption`])
//!
//! With [`Atomic`] and [`AtomicOption`] it's not safe to get a reference, you must replace the
//! value to access it
//!
//! To safely get a reference to T you must use [`FillOnceAtomicOption`] and accept the API limitations
//!
//! A safe `AtomicArc` is impossible without a GC, so you must use `ArcCell` from crossbeam (locks to clone) or [`FillOnceAtomicArc`]
//!
//! # Thread-safe appendable list that can create a lock-free iterator
//!  - [`VoluntaryServitude`] (also called [`VS`])
//!
//! # Api of `VS` Iterator
//! - [`Iter`]
//!
//! [`Atomic`]: ./struct.Atomic.html
//! [`AtomicOption`]: ./struct.AtomicOption.html
//! [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html
//! [`FillOnceAtomicArc`]: ./struct.FillOnceAtomicArc.html
//! [`Atomic abstractions (Atomic, AtomicOption, FillOnceAtomicOption, FillOnceAtomicArc)`]: #atomic-abstractions
//! [`Thread-safe appendable list with a lock-free iterator (VoluntaryServitude - also called VS)`]: ./struct.VoluntaryServitude.html
//! [`Serde serialization/deserialization ("serde-traits" feature)`]: ./struct.VoluntaryServitude.html#impl-Serialize
//! [`par_extend, from_par_iter rayon implementation ("rayon-traits" feature)`]: ./struct.VoluntaryServitude.html#impl-1
//! [`Use VoluntaryServitude from C (FFI)`]: ./ffi/index.html
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html
//! [`Iter`]: ./struct.Iter.html
//! [`Logging ("logs" feature)`]: #functions

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
#![doc(html_root_url = "https://docs.rs/voluntary_servitude/3.0.7/voluntary-servitude")]

#[cfg(feature = "rayon-traits")]
extern crate rayon as rayon_lib;

#[cfg(feature = "serde-traits")]
extern crate serde as serde_lib;

extern crate parking_lot;

#[macro_use]
#[cfg(feature = "logs")]
extern crate log;
#[cfg(feature = "logs")]
extern crate env_logger;

/// Setup logger according to `RUST_LOG` env var
///
/// *During tests log to stdout to supress output on passes*
///
/// # Enable the feature:
///
/// **Cargo.toml**
/// ```toml
/// [dependencies]
/// voluntary_servitude = { version = "3", features = "logs" }
/// ```
///
/// # Set the `RUST_LOG` env var:
/// ```bash
/// export RUST_LOG=voluntary_servitude=trace
/// export RUST_LOG=voluntary_servitude=debug
/// export RUST_LOG=voluntary_servitude=info
/// export RUST_LOG=voluntary_servitude=warn
/// export RUST_LOG=voluntary_servitude=error
/// ```
///
/// ```rust
/// // Must enable the `logs` feature and set the appropriate `RUST_LOG` env var
/// voluntary_servitude::setup_logger();
/// // Call code to be logged
/// // ...
/// ```
#[cfg(feature = "logs")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "logs")))]
#[inline]
pub fn setup_logger() {
    /// Ensures logger is only initialized once
    static STARTED: std::sync::Once = std::sync::ONCE_INIT;
    STARTED.call_once(env_logger::init);
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

#[macro_use]
mod macros;
mod atomic;
mod atomic_option;
#[cfg(feature = "ffi")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "ffi")))]
pub mod ffi;
mod fill_once_atomic_arc;
mod fill_once_atomic_option;
mod iterator;
mod node;
mod voluntary_servitude;

#[cfg(feature = "rayon-traits")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "rayon-traits")))]
mod rayon;

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "serde-traits")))]
#[cfg(feature = "serde-traits")]
mod serde;

pub use atomic::Atomic;
pub use atomic_option::{AtomicOption, NotEmpty};
pub use fill_once_atomic_arc::FillOnceAtomicArc;
pub use fill_once_atomic_option::FillOnceAtomicOption;
pub use iterator::Iter;
pub use voluntary_servitude::{VoluntaryServitude, VS};

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

/// Abstracts conditional execution
pub(crate) trait AlsoRun<T> {
    /// Runs closure conditionally
    fn also_run<P: FnMut(&T)>(self, func: P) -> Self;
}

impl<T> AlsoRun<T> for Option<T> {
    #[inline]
    fn also_run<P: FnMut(&T)>(self, mut func: P) -> Self {
        self.filter(|t| (func(t), true).1)
    }
}
