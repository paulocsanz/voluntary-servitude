//! Lockfree data-structures
//!
//! Currently only implements a thread-safe appendable list with a lock-free iterator
//!
//! Contains FFI implementation, see C examples in **./examples** or in 'ffi' module
//!
//! *Uses system allocator by default, jemmaloc can be enabled with the 'jemmaloc' feature*
//!
//! ```bash
//! cargo build --features "jemmaloc"
//! ```
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
//! export RUST_LOG=voluntary_servitude=warn
//! ```
//!
//! # Single thread
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//!
//! const ELEMENTS: usize = 10000;
//! // Creates VSRead with 3 elements
//! // vsread![] and VSRead::default() make an empty VSRead
//! // vsread![1; 3] makes a VSRead with 3 elements equal to 1
//! let list = vsread![0, 1, 2];
//!
//! // Current VSRead length
//! // Be careful with data-races since the value, when used, may not be true anymore
//! assert_eq!(list.len(), 3);
//!
//! // The 'iter' method makes a one-time lock-free iterator (VSReadIter) based on VSRead
//! assert_eq!(list.iter().len(), 3);
//!
//! // You can get the current iteration index
//! // (if iter.index() is equal to iter.len(), then the iteration ended - iter.next() is None)
//! let mut iter = list.iter();
//! assert_eq!(iter.index(), 0);
//! assert_eq!(iter.next(), Some(&0));
//! assert_eq!(iter.index(), 1);
//!
//! // Appends 9997 elements to it
//! assert_eq!((3..ELEMENTS).map(|i| list.append(i)).count(), ELEMENTS - 3);
//!
//! // Iterates through all elements to ensure it's what we inserted
//! let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
//! assert_eq!(count, ELEMENTS);
//!
//! let iter2 = list.iter();
//!
//! // List can also be cleared (but current iterators are not affected)
//! list.clear();
//!
//! assert_eq!(list.len(), 0);
//! assert_eq!(list.iter().len(), 0);
//! assert_eq!(list.iter().next(), None);
//! assert_eq!(iter2.len(), ELEMENTS);
//! let count = iter2.enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
//! assert_eq!(count, ELEMENTS);
//!
//! println!("Single thread example ended without errors");
//! ```
//!
//! # Multi producer, multi consumer
//! ```
//! #[macro_use] extern crate voluntary_servitude;
//! use std::{thread::spawn, sync::Arc};
//!
//! const CONSUMERS: usize = 8;
//! const PRODUCERS: usize = 4;
//! const ELEMENTS: usize = 10000;
//!
//! let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
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

#![deny(
    missing_debug_implementations, missing_docs, trivial_numeric_casts, unused_extern_crates,
    unused_import_braces, unused_qualifications, unused_results
)]
#![deny(
    bad_style, const_err, dead_code, improper_ctypes, legacy_directory_ownership,
    non_shorthand_field_patterns, no_mangle_generic_items, overflowing_literals, path_statements,
    patterns_in_fns_without_body, plugin_as_library, private_in_public, private_no_mangle_fns,
    private_no_mangle_statics, safe_extern_statics, unconditional_recursion,
    unions_with_drop_fields, unused, unused_allocation, unused_comparisons, unused_parens,
    while_true
)]

#![doc(html_root_url = "https://docs.rs/voluntary_servitude/1.0.4/voluntary-servitude")]

#[cfg(not(feature = "jemmaloc"))]
use std::alloc::System;

#[cfg(not(feature = "jemmaloc"))]
#[global_allocator]
static GLOBAL_ALLOC: System = System;

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
#[cfg(not(feature = "logs"))]
macro_rules! warn {
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
