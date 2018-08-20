//! Lockfree data-structures
//!
//! Currently only implements a thread-safe appendable list with a lock-free iterator
//!
//! Contains FFI implementation, see C examples in **./examples**
//!
//! To enable logging set the feature 'logs' (and the appropriate config in env var)
//!
//! ```bash
//! cargo build --features "logs"
//! ```
//!
//! Examples:
//! ```bash
//! export RUST_LOG=voluntary_servitude=trace
//! export RUST_LOG=voluntary_servitude=debug
//! export RUST_LOG=voluntary_servitude=info
//! ```
//!
//! # Single thread
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! use voluntary_servitude::VSRead;
//! # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//!
//! let list = vsread![]; // or VSRead::default();
//! assert_eq!((0..10000).map(|i| list.append(i)).count(), 10000);
//! let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
//! assert_eq!(count, 10000);
//! assert_eq!((0..10000).map(|i| list.append(i)).count(), 10000);
//! let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&(i % 10000), el)).count();
//! assert_eq!(count, 20000);
//! ```
//!
//! # Single producer, single consumer
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! use std::{thread::spawn, sync::Arc};
//! use voluntary_servitude::VSRead;
//!
//! # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//! let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
//! let producer = Arc::clone(&list);
//! let _handler = spawn(move || {
//!     let _ = (0..10000).map(|i| producer.append(i)).count();
//! });
//!
//! loop {
//!     let count = list.iter().count();
//!     println!("{} elements", count);
//!     if count == 10000 { break; }
//! }
//!
//! // List can also be cleared
//! list.clear();
//! assert_eq!(list.iter().count(), 0);
//! ```
//!
//! # Multi producer, multi consumer
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! use std::{thread::spawn, sync::Arc};
//! use voluntary_servitude::VSRead;
//!
//! # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//! const consumers: usize = 8;
//! const producers: usize = 4;
//!
//! let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
//! let mut handlers = vec![];
//!
//! for _ in (0..producers) {
//!     let l = Arc::clone(&list);
//!     handlers.push(spawn(move || { let _ = (0..10000).map(|i| l.append(i)).count(); }));
//! }
//!
//! for c in (0..consumers) {
//!     let consumer = Arc::clone(&list);
//!     handlers.push(spawn(move || {
//!         loop {
//!             let count = consumer.iter().count();
//!             println!("{} elements", count);
//!             if count == producers * 10000 { break; }
//!         }
//!     }));
//! }
//! ```

#![deny(
    missing_debug_implementations, missing_docs, trivial_casts, trivial_numeric_casts,
    unused_extern_crates, unused_import_braces, unused_qualifications, unused_results
)]
#![deny(
    bad_style, const_err, dead_code, improper_ctypes, legacy_directory_ownership,
    non_shorthand_field_patterns, no_mangle_generic_items, overflowing_literals, path_statements,
    patterns_in_fns_without_body, plugin_as_library, private_in_public, private_no_mangle_fns,
    private_no_mangle_statics, safe_extern_statics, unconditional_recursion,
    unions_with_drop_fields, unused, unused_allocation, unused_comparisons, unused_parens,
    while_true
)]

#[macro_use]
#[cfg(feature = "logs")]
extern crate log;
#[cfg(feature = "logs")]
extern crate env_logger;

#[cfg(not(feature = "logs"))]
macro_rules! trace {
    ($($x:expr),*) => {};
}
#[cfg(not(feature = "logs"))]
macro_rules! debug {
    ($($x:expr),*) => {};
}
#[cfg(not(feature = "logs"))]
macro_rules! info {
    ($($x:expr),*) => {};
}

/// Setup logger according to RUST_LOG env var (only exists in debug mode)
///
/// export RUST_LOG=voluntary_servitude=trace
/// export RUST_LOG=voluntary_servitude=debug
/// export RUST_LOG=voluntary_servitude=info
///
/// ```
/// use voluntary_servitude::setup_logger;
/// setup_logger();
/// // Call code that should be logged to terminal
/// ```
#[cfg(feature = "logs")]
pub fn setup_logger() {
    use std::sync::{Once, ONCE_INIT};
    static STARTED: Once = ONCE_INIT;
    STARTED.call_once(|| {
        env_logger::Builder::from_default_env()
            .default_format_module_path(false)
            .default_format_timestamp(false)
            .init();
    })
}

#[macro_use]
mod macros;
pub mod ffi;
mod iter;
mod node;
mod types;
mod vsread;

pub use types::VSRead;
