//! Lock-free data-structures
//!
//! Implements a lock-free thread-safe appendable list with a lock-free iterator
//!  - [`VoluntaryServitude`] (also called [`VS`])
//!
//! # Features
//!  - [`Lock-free thread-safe appendable list`]
//!  - [`Serde serialization ('serde-traits' feature)`]
//!  - [`Call this code from C (FFI)`] (also in **./examples**)
//!  - System Allocator ('system-alloc' feature)
//!  - [`Logging ('logs' feature)`]
//!
//! [`Lock-free thread-safe appendable list`]: #multi-producer-multi-consumer
//! [`Serde serialization ('serde-traits' feature)`]: ./serde/index.html
//! [`Call this code from C (FFI)`]: ./ffi/index.html
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html
//! [`Logging ('logs' feature)`]: ./fn.setup_logger.html
//!
//! # Single thread
//! ```rust
//! # #[macro_use] extern crate voluntary_servitude;
//! # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//!
//! let (a, b, c) = (0usize, 1usize, 2usize);
//! // VS alias to VoluntaryServitude
//! // vs! alias to voluntary_servitude! (and operate like vec!)
//! let list = vs![a, b, c];
//! assert_eq!(list.iter().collect::<Vec<_>>(), vec![&a, &b, &c]);
//!
//! // Current VS's length
//! // Be careful with data-races since the value, when used, may not be true anymore
//! assert_eq!(list.len(), 3);
//!
//! // The 'iter' method makes a one-time lock-free iterator (VSIter)
//! for (index, element) in list.iter().enumerate() {
//!     assert_eq!(index, *element);
//! }
//!
//! // You can get the current iteration index
//! // iter.next() == iter.len() means iteration ended (iter.next() == None)
//! let mut iter = list.iter();
//! assert_eq!(iter.index(), 0);
//! assert_eq!(iter.next(), Some(&0));
//! assert_eq!(iter.index(), 1);
//!
//! // List can also be cleared (but current iterators are not affected)
//! list.clear();
//!
//! assert_eq!(iter.len(), 3);
//! assert_eq!(list.len(), 0);
//! assert_eq!(list.iter().len(), 0);
//! assert_eq!(list.iter().next(), None);
//!
//! println!("Single thread example ended without errors");
//! ```
//!
//! # Multi producer, multi consumer
//! ```rust
//! #[macro_use] extern crate voluntary_servitude;
//! use std::{sync::Arc, thread::spawn};
//!
//! const CONSUMERS: usize = 8;
//! const PRODUCERS: usize = 4;
//! const ELEMENTS: usize = 10000;
//!
//! let list = Arc::new(vs![]);
//! let mut handlers = vec![];
//!
//! // Creates producer threads to insert 10k elements
//! for _ in 0..PRODUCERS {
//!     let l = Arc::clone(&list);
//!     handlers.push(spawn(move || { let _ = (0..ELEMENTS).map(|i| l.append(i)).count(); }));
//! }
//!
//! // Creates consumer threads to print number of elements until all of them are inserted
//! for _ in 0..CONSUMERS {
//!     let consumer = Arc::clone(&list);
//!     handlers.push(spawn(move || {
//!         loop {
//!             let count = consumer.iter().count();
//!             println!("{} elements", count);
//!             if count == PRODUCERS * ELEMENTS { break; }
//!         }
//!     }));
//! }
//!
//! // Join threads
//! for handler in handlers.into_iter() {
//!     handler.join().expect("Failed to join thread");
//! }
//!
//! println!("Multi thread example ended without errors");
//! ```

#![cfg_attr(docs_rs_workaround, feature(global_allocator, doc_cfg))]
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

#[cfg(feature = "system-alloc")]
use std::alloc::System;

#[cfg(feature = "system-alloc")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "system-alloc")))]
#[global_allocator]
/// Represents the use of the system's allocator instead of rust's default
///
/// By default is disabled, but can be enabled with the 'system-alloc' feature
/// It's intended to be used by the FFI, but you can use it in rust by setting in Cargo.toml
///
/// ```bash
/// cargo build --release --features "system-alloc"
/// ```
///
/// *./dist/libvoluntary_servitude.so (FFI) is compiled with system allocator*
pub static GLOBAL_ALLOC: System = System;

#[macro_use]
#[cfg(feature = "logs")]
extern crate log;
#[cfg(feature = "logs")]
extern crate env_logger;

/// Setup logger according to `RUST_LOG` env var (must enable "logs" feature)
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
/// // Must enable the "logs" feature and set the appropriate `RUST_LOG` env var
/// voluntary_servitude::setup_logger();
/// // Call code to be logged
/// // ...
/// ```
#[cfg(feature = "logs")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "logs")))]
#[inline]
pub fn setup_logger() {
    static STARTED: std::sync::Once = std::sync::ONCE_INIT;
    #[cfg(not(test))]
    STARTED.call_once(|| env_logger::init());
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

/// Logging is not enabled, it's available behind "logs" feature flag
///
/// When "logs" is set the function `setup_logger` will be available to start logging execution
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
/// ```rust
/// // Must enable the "logs" feature and set the appropriate `RUST_LOG` env var
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
pub mod ffi;
mod iterator;
mod node;
#[cfg(feature = "serde-traits")]
#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "serde-traits")))]
pub mod serde;
mod voluntary_servitude;

#[cfg(not(feature = "serde-traits"))]
pub mod serde {
    //! Serde is not enabled, it's available behind "serde-traits" feature flag
    //!
    //! This feature provides access to serde's Serialize/Deserialize implemnetation for [`VSIter`] and [`VoluntaryServitude`]
    //!
    //! # Serialize
    //!  - [`VoluntaryServitude`]
    //!  - [`VSIter`]
    //!
    //! # Deserialize
    //!  - [`VoluntaryServitude`]
    //!
    //! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
    //! [`VSIter`]: ../struct.VSIter.html
    //!
    //! # Enable the feature:
    //!
    //! **Cargo.toml**
    //! ```toml
    //! [dependencies]
    //! voluntary_servitude = { version = "3", features = "logs" }
    //! ```
    //!
    //! # See full docs:
    //! ```bash
    //! cargo doc --all-features --open
    //! ```
    //!
    //! # To test integration with serde 'serde-tests' must also be enabled
    //! ```bash
    //! cargo test --features "serde-traits serde-tests"
    //! ```
}

pub use iterator::VSIter;
pub use voluntary_servitude::{VoluntaryServitude, VS};
