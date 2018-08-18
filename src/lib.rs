//! Lockfree data-structures
//!
//! Currently only implements a thread-safe appendable list with a lock-free iterator
//!
//! # Single thread
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! # extern crate env_logger;
//! use voluntary_servitude::VSRead;
//!
//! # fn main() {
//! # ::std::env::set_var("RUST_LOG", "trace");
//! # env_logger::Builder::from_default_env()
//! #       .default_format_module_path(false)
//! #       .default_format_timestamp(false)
//! #       .init();
//! let list = vsread![]; // or VSRead::default();
//! assert_eq!((0..10000).map(|i| list.append(i)).count(), 10000);
//! let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
//! assert_eq!(count, 10000);
//! assert_eq!((0..10000).map(|i| list.append(i)).count(), 10000);
//! let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&(i % 10000), el)).count();
//! assert_eq!(count, 20000);
//! # }
//! ```
//!
//! # Single producer, single consumer
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! # extern crate env_logger;
//! use std::{thread::spawn, sync::Arc};
//! use voluntary_servitude::VSRead;
//!
//! # fn main() {
//! # ::std::env::set_var("RUST_LOG", "trace");
//! # env_logger::Builder::from_default_env()
//! #       .default_format_module_path(false)
//! #       .default_format_timestamp(false)
//! #       .init();
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
//! # }
//! ```
//!
//! # Multi producer, multi consumer
//! ```
//! # #[macro_use] extern crate voluntary_servitude;
//! # extern crate env_logger;
//! use std::{thread::spawn, sync::Arc};
//! use voluntary_servitude::VSRead;
//!
//! const consumers: usize = 8;
//! const producers: usize = 4;
//!
//! # fn main() {
//! # ::std::env::set_var("RUST_LOG", "trace");
//! # env_logger::Builder::from_default_env()
//! #       .default_format_module_path(false)
//! #       .default_format_timestamp(false)
//! #       .init();
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
//! # }
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
#[cfg(debug_assertions)]
extern crate log;

#[cfg(test)]
extern crate env_logger;

#[cfg(not(debug_assertions))]
macro_rules! trace {
    ($($x:expr),*) => {};
}
#[cfg(not(debug_assertions))]
macro_rules! debug {
    ($($x:expr),*) => {};
}
#[cfg(not(debug_assertions))]
macro_rules! info {
    ($($x:expr),*) => {};
}

#[cfg(test)]
fn setup_logger() {
    use std::{
        env::set_var,
        sync::{Once, ONCE_INIT},
    };
    static STARTED: Once = ONCE_INIT;
    STARTED.call_once(|| {
        set_var("RUST_LOG", "trace");

        env_logger::Builder::from_default_env()
            .default_format_module_path(false)
            .default_format_timestamp(false)
            .init();
    })
}

#[macro_use]
mod macros;
mod iter;
mod node;
mod types;
mod vsread;

pub use types::VSRead;
