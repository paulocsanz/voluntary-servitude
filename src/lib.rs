//! Atomic abstractions and thread-safe appendable list with lock-free iterators
//!
//! # Features
//!  - [`Atomic abstractions (Atomic, AtomicOption, FillOnceAtomicOption, FillOnceAtomicArc)`]
//!  - [`Thread-safe appendable list with a lock-free iterator (VoluntaryServitude - also called VS)`]
//!  - [`Serde serialization ("serde-traits" feature)`]
//!  - [`Call this code from C (FFI)`] (also in **./examples**)
//!  - [`System Allocator ("system-alloc" feature)`]
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
//! A safe `AtomicArc` is impossible, so you must use `ArcCell` from crossbeam (locks to clone) or [`FillOnceAtomicArc`]
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
//! [`Serde serialization ("serde-traits" feature)`]: ./serde/index.html
//! [`Call this code from C (FFI)`]: ./ffi/index.html
//! [`System Allocator ("system-alloc" feature)`]: ./static.GLOBAL_ALLOC.html
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html
//! [`Iter`]: ./struct.Iter.html
//! [`Logging ("logs" feature)`]: ./fn.setup_logger.html

#![cfg_attr(docs_rs_workaround, feature(allocator_api))]
#![cfg_attr(docs_rs_workaround, feature(global_allocator))]
#![cfg_attr(docs_rs_workaround, feature(doc_cfg))]
#![deny(
    missing_debug_implementations,
    missing_docs,
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
    private_no_mangle_fns,
    private_no_mangle_statics,
    safe_extern_statics,
    unconditional_recursion,
    unions_with_drop_fields,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
#![doc(test(attr(deny(warnings))))]
#![doc(html_root_url = "https://docs.rs/voluntary_servitude/3.0.0/voluntary-servitude")]

#[cfg(feature = "serde-traits")]
extern crate serde as serde_lib;

#[cfg(feature = "rayon-traits")]
extern crate rayon as rayon_lib;

#[cfg(feature = "system-alloc")]
use std::alloc::System;

/// Represents the use of the system's allocator instead of rust's default
///
/// By default is disabled, but can be enabled with the `system-alloc` feature
/// It's intended to be used by the FFI, but you can use it in rust by setting in Cargo.toml
///
/// ```bash
/// cargo build --release --features "system-alloc"
/// ```
///
/// *`./dist/libvoluntary_servitude.so` (FFI) is compiled with the system's allocator*
#[cfg(feature = "system-alloc")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "system-alloc")))]
#[global_allocator]
pub static GLOBAL_ALLOC: System = System;

#[cfg(not(feature = "system-alloc"))]
/// System allocator is not enabled, it's available behind the `system-alloc` feature flag
///
/// It's intended to be used by the FFI, but you can use it in rust by setting in Cargo.toml
///
/// ```bash
/// cargo build --release --features "system-alloc"
/// ```
///
/// *`./dist/libvoluntary_servitude.so` (FFI) is compiled with system allocator*
pub static GLOBAL_ALLOC: () = ();

extern crate crossbeam;

#[macro_use]
#[cfg(feature = "logs")]
extern crate log;
#[cfg(feature = "logs")]
extern crate env_logger;

/// Setup logger according to `RUST_LOG` env var (must enable `logs` feature)
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
    #[cfg(not(test))]
    STARTED.call_once(env_logger::init);
    #[cfg(test)]
    STARTED.call_once(|| {
        use env_logger::{Builder, Target};
        use std::env::var;
        let mut builder = Builder::new();
        let _ = builder.target(Target::Stdout);
        if var("RUST_LOG").is_ok() {
            let _ = builder.parse(&var("RUST_LOG").unwrap());
        }
        builder.init();
    });
}

/// Enum impossible to construct (hint that the code is unreachable)
#[cfg(not(feature = "logs"))]
#[doc(hidden)]
#[derive(Debug)]
pub enum ImpossibleToInstantiate {}

/// Logging is not enabled, it's available behind the `logs` feature flag
///
/// When "logs" is set the function `setup_logger` will be available to start logging the execution
///
/// # Enable the feature:
/// **Cargo.toml**
/// ```toml
/// [dependencies]
/// voluntary_servitude = { version = "3", features = "logs" }
/// ```
///
/// # See full docs:
/// ```bash
/// cargo doc --all-features --open
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
/// ```_rust
/// // Must enable the `logs` feature and set the appropriate `RUST_LOG` env var
/// voluntary_servitude::setup_logger();
/// // Call code to be logged
/// // ...
/// ```
#[cfg(not(feature = "logs"))]
#[inline]
pub fn setup_logger(_: ImpossibleToInstantiate) {}

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
pub mod ffi;
mod fill_once_atomic_arc;
mod fill_once_atomic_option;
mod iterator;
mod node;
mod voluntary_servitude;

#[cfg(feature = "rayon-traits")]
pub mod rayon;
#[cfg(feature = "serde-traits")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "serde-traits")))]
pub mod serde;

#[cfg(not(feature = "serde-traits"))]
pub mod serde {
    //! Serde integration is not enabled, it's available behind `serde-traits` feature flag
    //!
    //! This feature provides access to serde's `Serialize`/`Deserialize` trait implementation for [`VoluntaryServitude`]
    //!
    //! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html#implementations
    //!
    //! # Enable the feature:
    //!
    //! **Cargo.toml**
    //!
    //! ```toml
    //! [dependencies]
    //! voluntary_servitude = { version = "3", features = "logs" }
    //! ```
    //!
    //! # See full docs:
    //!
    //! ```bash
    //! cargo doc --all-features --open
    //! ```
    //!
    //! # To test integration with serde `serde-tests` must also be enabled
    //!
    //! ```bash
    //! cargo test --features "serde-traits serde-tests"
    //! ```
}

#[cfg(feature = "rayon-traits")]
pub use rayon::ParIter;

pub use atomic::Atomic;
pub use atomic_option::{AtomicOption, NotEmpty};
pub use fill_once_atomic_arc::FillOnceAtomicArc;
pub use fill_once_atomic_option::FillOnceAtomicOption;
pub use iterator::Iter;
pub use voluntary_servitude::{VoluntaryServitude, VS};

use std::ptr::{null_mut, NonNull};

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

impl<T> IntoPtr<T> for *mut T {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> Self {
        self
    }
}

impl<T> IntoPtr<T> for Option<*mut T> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        self.unwrap_or(null_mut())
    }
}

impl<T> IntoPtr<T> for Option<NonNull<T>> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        self.map_or(null_mut(), |nn| nn.as_ptr())
    }
}

impl<T> IntoPtr<T> for Box<T> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        Self::into_raw(self)
    }
}

impl<T> IntoPtr<T> for Option<T> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        self.map_or(null_mut(), |v| v.into_ptr())
    }
}

impl<T> IntoPtr<T> for Option<Box<T>> {
    #[inline]
    #[must_use]
    fn into_ptr(self) -> *mut T {
        self.map_or(null_mut(), Box::into_raw)
    }
}
