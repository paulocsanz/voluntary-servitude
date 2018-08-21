//! Lockfree data-structures
//!
//! Currently only implements a thread-safe appendable list with a lock-free iterator
//!
//! Contains FFI implementation, see C examples in **./examples**
//!
//! To enable logging set the feature 'logs' (and the appropriate config in env var)
//!
//! *Uses system allocator by default, jemmaloc can be enabled with the 'jemmaloc' feature*
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
//!
//! unsafe {
//!     # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//!     // Create VSRead with 3 elements
//!     // vsread![] makes an empty VSRead
//!     // vsread![1; 3] makes a VSRead with 3 elements with 1 as value
//!     let list = vsread![0, 1, 2];
//!     assert_eq!(list.len(), 3);
//!
//!     // The 'iter' method makes a one-time lock-free iterator (VSReadIter) based on VSRead
//!     assert_eq!(list.iter().len(), 3);
//!
//!     // You can get the current iteration index (can be compared with the length 'len')
//!     assert_eq!(list.iter().index(), 0);
//!
//!     // Appends 9997 elements to it
//!     assert_eq!((3..10000).map(|i| list.append(i)).count(), 9997);
//!
//!     // Iterates through all elements to ensure it's what we inserted
//!     let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
//!     assert_eq!(count, 10000);
//!
//!     // List can also be cleared
//!     list.clear();
//!     assert_eq!(list.len(), 0);
//! }
//! ```
//!
//! # Multi producer, multi consumer
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! use std::{thread::spawn, sync::Arc};
//!
//! unsafe {
//!     # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
//!
//!     const CONSUMERS: usize = 8;
//!     const PRODUCERS: usize = 4;
//!
//!     let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
//!     let mut handlers = vec![];
//!
//!     // Creates producer threads to insert 10k elements each
//!     for _ in 0..PRODUCERS {
//!         let l = Arc::clone(&list);
//!         handlers.push(spawn(move || { let _ = (0..10000).map(|i| l.append(i)).count(); }));
//!     }
//!
//!     // Creates consumer threads to print number of elements until all elements are inserted
//!     for _ in 0..CONSUMERS {
//!         let consumer = Arc::clone(&list);
//!         handlers.push(spawn(move || {
//!             loop {
//!                 let count = consumer.iter().count();
//!                 println!("{} elements", count);
//!                 if count == PRODUCERS * 10000 { break; }
//!             }
//!         }));
//!     }
//!
//!     // Join threads
//!     for handler in handlers.into_iter() {
//!         handler.join().expect("Failed to join thread");
//!     }
//! }
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
