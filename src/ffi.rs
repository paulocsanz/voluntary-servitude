//! Voluntary Servitude Foreign Function Interface (FFI)
//!
//! Allows using this rust library as a C library
//!
//! While `vs_t` ([`VoluntaryServitude`] in C) is thread-safe it's your responsibility to make sure it exists while pointers to it exist
//!
//! `vs_iter_t` ([`Iter`] in C) can outlive `vs_t` and isn't affected by `vs_clear`, it is **not** thread-safe.
//!
//! It can only be called by one thread (but multiple `vs_iter_t` of the same `vs_t` can exist at the same time)
//!
//! [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
//! [`Iter`]: ../struct.Iter.html
//!
//! # Examples
//!  - [`Single-thread C implementation`]
//!  - [`Multi-producer, multi-consumer C implementation`]
//!
//!  [`Single-thread C implementation`]: #single-thread-c-implementation
//!  [`Multi-producer, multi-consumer C implementation`]: #multi-producer-multi-consumer-c-implementation
//!
//! # Single-thread C implementation
//!
//! ```c
//! #include<assert.h>
//! #include<stdio.h>
//! #include "../include/voluntary_servitude.h"
//!
//! int main(int argc, char **argv) {
//!     // You are responsible for making sure 'vs' exists while accessed
//!     vs_t * vs = vs_new();
//!
//!     // Current vs_t length
//!     // Be careful with race-conditions since the value, when used, may not be true anymore
//!     assert(vs_len(vs) == 0);
//!
//!     const unsigned int data[2] = {12, 25};
//!     // Inserts void pointer to data to end of vs_t
//!     vs_append(vs, (void *) &data[0]);
//!     vs_append(vs, (void *) &data[1]);
//!
//!     // Creates a one-time lock-free iterator based on vs_t
//!     vs_iter_t * iter = vs_iter(vs);
//!
//!     // Clearing vs_t, doesn't change existing iterators
//!     vs_clear(vs);
//!     assert(vs_len(vs) == 0);
//!     assert(vs_iter_len(iter) == 2);
//!
//!     assert(*(unsigned int *) vs_iter_next(iter) == 12);
//!     // Index changes as you iter through vs_iter_t
//!     assert(vs_iter_index(iter) == 1);
//!     assert(*(unsigned int *) vs_iter_next(iter) == 25);
//!     assert(vs_iter_index(iter) == 2);
//!
//!     assert(vs_iter_next(iter) == NULL);
//!     assert(vs_iter_index(iter) == 2);
//!     // Index doesn't increase after it gets equal to 'len'
//!     // Length also is unable to increase after iterator is consumed
//!     assert(vs_iter_index(iter) == vs_iter_len(iter));
//!
//!     // Never forget to free vs_iter_t
//!     assert(vs_iter_destroy(iter) == 0);
//!
//!     // Create updated vs_iter_t
//!     vs_iter_t * iter2 = vs_iter(vs);
//!
//!     // Never forget to free vs_t
//!     assert(vs_destroy(vs) == 0);
//!
//!     // vs_iter_t keeps existing after the original vs_t is freed (or cleared)
//!     assert(vs_iter_len(iter2) == 0);
//!     assert(vs_iter_next(iter2) == NULL);
//!     assert(vs_iter_index(iter2) == 0);
//!
//!     assert(vs_iter_destroy(iter2) == 0);
//!
//!     printf("Single thread example ended without errors\n");
//!     (void) argc;
//!     (void) argv;
//!     return 0;
//! }
//! ```
//!
//! # Multi-producer, multi-consumer C implementation
//!
//! ```c
//! #include<pthread.h>
//! #include<assert.h>
//! #include<stdio.h>
//! #include "../include/voluntary_servitude.h"
//!
//! const unsigned int num_producers = 4;
//! const unsigned int num_consumers = 8;
//!
//! const unsigned int num_producer_values = 1000;
//! const unsigned int data[3] = {12, 25, 89};
//! const size_t last_index = sizeof(data) / sizeof(data[0]) - 1;
//!
//! void * producer();
//! void * consumer();
//!
//! int main(int argc, char** argv) {
//!     // You are responsible for making sure 'vs' exists
//!     // while the producers and consumers do
//!     vs_t * const vs = vs_new();
//!     unsigned int current_thread = 0;
//!     pthread_attr_t attr;
//!     pthread_t consumers[num_consumers],
//!               producers[num_producers];
//!
//!     if (pthread_attr_init(&attr) != 0) {
//!         fprintf(stderr, "Failed to initialize pthread arguments.\n");
//!         exit(-1);
//!     }
//!
//!     // Creates producer threads
//!     for (current_thread = 0; current_thread < num_producers; ++current_thread) {
//!         if (pthread_create(&producers[current_thread], &attr, &producer, (void *) vs) != 0) {
//!             fprintf(stderr, "Failed to create producer thread %d.\n", current_thread);
//!             exit(-2);
//!         }
//!
//!     }
//!
//!     // Creates consumers threads
//!     for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
//!         if (pthread_create(&consumers[current_thread], &attr, &consumer, (void *) vs) != 0) {
//!             fprintf(stderr, "Failed to create consumer thread %d.\n", current_thread);
//!             exit(-3);
//!         }
//!     }
//!
//!     // Join all threads, ensuring vs_t* is not used anymore
//!     for (current_thread = 0; current_thread < num_producers; ++current_thread) {
//!         pthread_join(producers[current_thread], NULL);
//!     }
//!     for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
//!         pthread_join(consumers[current_thread], NULL);
//!     }
//!
//!     // Never forget to free the memory allocated through the lib
//!     assert(vs_destroy(vs) == 0);
//!
//!     printf("Multi thread example ended without errors\n");
//!     (void) argc;
//!     (void) argv;
//!     return 0;
//! }
//!
//!
//! void * producer(void * const vs){
//!     unsigned int index;
//!     for (index = 0; index < num_producer_values; ++index) {
//!         assert(vs_append(vs, (void *) &data[index % last_index]) == 0);
//!     }
//!     return NULL;
//! }
//!
//! void * consumer(void * const vs) {
//!     const unsigned int total_values = num_producers * num_producer_values;
//!     unsigned int values;
//!
//!     while (values < total_values) {
//!         unsigned int sum = (values = 0);
//!         vs_iter_t * const iter = vs_iter(vs);
//!         const void * value;
//!
//!         while ((value = vs_iter_next(iter)) != NULL) {
//!             ++values;
//!             sum += *(unsigned int *) value;
//!         }
//!         printf("Consumer counts %d elements summing %d.\n", values, sum);
//!
//!         // Never forget to free the memory allocated through the lib
//!         assert(vs_iter_destroy(iter) == 0);
//!     }
//!     return NULL;
//! }
//! ```

use std::{ptr::drop_in_place, os::raw::c_void, ptr::null, ptr::NonNull};
use {IntoPtr, Iter, VS};

/// [`VoluntaryServitude`]'s representation in C
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
#[allow(non_camel_case_types)]
pub type vs_t = VS<*const c_void>;

/// [`Iter`]'s representation in C
///
/// [`Iter`]: ./struct.Iter.html
#[allow(non_camel_case_types)]
pub type vs_iter_t<'a> = Iter<'a, *const c_void>;

/// Initializes logger according to `RUST_LOG` env var (exists behind the `logs` feature)
///
/// ```bash
/// export RUST_LOG=vs=trace
/// export RUST_LOG=vs=debug
/// export RUST_LOG=vs=info
/// export RUST_LOG=vs=warn
/// export RUST_LOG=vs=error
/// ```
///
/// Feature to enable it:
///
/// ```bash
/// cargo build --features "logs"
/// ```
///
/// ```rust
/// use voluntary_servitude::ffi::*;
/// unsafe { initialize_logger() }
/// ```
#[no_mangle]
#[cfg(feature = "logs")]
pub unsafe extern "C" fn initialize_logger() {
    ::setup_logger();
}

/// Creates new empty [`VoluntaryServitude`]
///
/// [`vs_destroy`] should be called eventually for the [`VoluntaryServitude`] returned, otherwise memory will leak
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
/// [`vs_destroy`]: ./fn.vs_destroy.html
///
/// # Rust
///
/// ```rust
/// use voluntary_servitude::ffi::*;
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     assert_eq!(vs_len(vs), 0);
///     assert_eq!(vs_destroy(vs), 0);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     assert(vs_len(vs) == 0);
///     assert(vs_destroy(vs) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_new() -> *mut vs_t {
    vs![].into_ptr()
}

/// Makes lock-free iterator ([`Iter`]) based on [`VoluntaryServitude`]
///
/// [`vs_iter_destroy`] should be called eventually for the [`Iter`] returned, otherwise memory will leak
///
/// Returns `NULL`  if pointer to [`VoluntaryServitude`] is `NULL`
///
/// Warning: UB if pointer to [`VoluntaryServitude`] is invalid
///
/// For a more thorough example on `vs_iter` check [`vs_iter_next`] documentation (or [`Iter`] directly)
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
/// [`Iter`]: ../struct.Iter.html
/// [`vs_iter_destroy`]: ./fn.vs_iter_destroy.html
/// [`vs_iter_next`]: ./fn.vs_iter_next.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: i32 = 5;
///
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     let iter = vs_iter(vs);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_destroy(vs), 0);
///
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///     assert!(vs_iter_next(iter).is_null());
///     assert_eq!(vs_iter_destroy(iter), 0);
///
///     // Propagates NULL pointers
///     assert!(vs_iter(null_mut()).is_null());
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     const unsigned int data = 5;
///
///     assert(vs_append(vs, (void *) &data) == 0);
///     vs_iter_t * iter = vs_iter(vs);
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_destroy(vs) == 0);
///
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///     assert(vs_iter_next(iter) == NULL);
///
///     assert(vs_iter_destroy(iter) == 0);
///
///     // Propagates NULL pointers
///     assert(vs_iter(NULL) == NULL);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_iter<'a>(vs: *mut vs_t) -> *mut vs_iter_t<'a> {
    NonNull::new(vs).map(|nn| (&*nn.as_ptr()).iter()).into_ptr()
}

/// Atomically extracts current size of [`VoluntaryServitude`], be careful with race conditions when using it
///
/// Returns `0` if pointer to [`VoluntaryServitude`] is `NULL`
///
/// Warning: UB if pointer to [`VoluntaryServitude`] invalid
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     assert_eq!(vs_len(vs), 0);
///     let data: i32 = 5;
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_len(vs), 1);
///     assert_eq!(vs_clear(vs), 0);
///     assert_eq!(vs_len(vs), 0);
///     assert_eq!(vs_destroy(vs), 0);
///
///     // 0 length on NULL pointer
///     assert_eq!(vs_len(null_mut()), 0);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     assert(vs_len(vs) == 0);
///
///     const unsigned int data = 5;
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_len(vs) == 1);
///     assert(vs_clear(vs) == 0);
///     assert(vs_len(vs) == 0);
///     assert(vs_destroy(vs) == 0);
///
///     // 0 length on NULL pointer
///     assert(vs_len(NULL) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_len(vs: *const vs_t) -> usize {
    NonNull::new(vs as *mut vs_t).map_or(0, |nn| nn.as_ref().len())
}

/// Append element to [`VoluntaryServitude`]
///
/// Returns `1` if pointer to [`VoluntaryServitude`] or `c_void` is `NULL`
///
/// Returns `0` otherwise
///
/// Warning: UB if pointer to [`VoluntaryServitude`] is invalid
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: i32 = 5;
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_len(vs), 2);
///     assert_eq!(vs_destroy(vs), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vs_append(null_mut(), &data as *const i32 as *const c_void), 1);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     const unsigned int data = 5;
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_len(vs) == 2);
///     assert(vs_destroy(vs) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vs_append(NULL, (void *) &data) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_append(vs: *mut vs_t, element: *const c_void) -> u8 {
    NonNull::new(vs)
        .and_then(|vs| NonNull::new(element as *mut c_void).map(|el| (vs, el)))
        .map_or(1, |(vs, el)| (vs.as_ref().append(el.as_ptr()), 0).1)
}

/// Removes all elements from [`VoluntaryServitude`] (preserves existing [`Iter`])
///
/// Returns `1` if pointer to [`VoluntaryServitude`] is `NULL`
///
/// Returns `0` otherwise
///
/// Warning: UB if pointer to [`VoluntaryServitude`] is invalid
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
/// [`Iter`]: ../struct.Iter.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: i32 = 5;
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_len(vs), 1);
///     assert_eq!(vs_clear(vs), 0);
///     assert_eq!(vs_len(vs), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vs_clear(null_mut()), 1);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     const unsigned int data = 5;
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_len(vs) == 1);
///     assert(vs_clear(vs) == 0);
///     assert(vs_len(vs) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vs_clear(NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_clear(vs: *mut vs_t) -> u8 {
    NonNull::new(vs).map_or(1, |nn| (nn.as_ref().clear(), 0).1)
}

/// Free [`VoluntaryServitude`] (preserves existing [`Iter`])
///
/// Returns `1` if pointer to [`VoluntaryServitude`] is `NULL`
///
/// Returns `0` otherwise
///
/// Warning: UB if pointer to [`VoluntaryServitude`] is invalid
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
/// [`Iter`]: ../struct.Iter.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: i32 = 5;
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     let iter = vs_iter(vs);
///     assert_eq!(vs_destroy(vs), 0);
///
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///     assert_eq!(vs_iter_destroy(iter), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vs_destroy(null_mut()), 1);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     vs_iter_t * iter = vs_iter(vs);
///     const unsigned int data = 5;
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_destroy(vs) == 0);
///
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///     assert(vs_iter_destroy(vs) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vs_destroy(NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_destroy(list: *mut vs_t) -> u8 {
    NonNull::new(list).map_or(1, |nn| (drop_in_place(nn.as_ptr()), 0).1)
}

/// Obtains next element in [`Iter`], returns `NULL` if there are no more elements
///
/// Returns `NULL` if pointer to [`Iter`] is `NULL`
///
/// Warning: UB if pointer to [`Iter`] is invalid
///
/// [`Iter`]: ../struct.Iter.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: i32 = 5;
///
///     let iter = vs_iter(vs);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert!(vs_iter_next(iter).is_null());
///     assert_eq!(vs_iter_destroy(iter), 0);
///
///     let iter = vs_iter(vs);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 5);
///     assert!(vs_iter_next(iter).is_null());
///
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert!(vs_iter_next(iter).is_null());
///
///     assert_eq!(vs_iter_destroy(iter), 0);
///     assert_eq!(vs_destroy(vs), 0);
///
///     // Propagates NULL pointers
///     assert!(vs_iter_next(null_mut()).is_null());
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     vs_iter_t * iter = vs_iter(vs);
///     const unsigned int data = 5;
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_iter_next(iter) == NULL);
///     assert(vs_iter_destroy(iter) == 0);
///
///     iter = vs_iter(vs);
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(*(unsigned int *) vs_iter_next(iter) == 5);
///     assert(vs_iter_next(iter) == NULL);
///
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_iter_next(iter) == NULL);
///
///     assert(vs_iter_destroy(iter) == 0);
///     assert(vs_destroy(vs) == 0);
///
///     // Propagates NULL pointers
///     assert(vs_iter_next(NULL) == NULL);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_iter_next(iter: *mut vs_iter_t<'_>) -> *const c_void {
    NonNull::new(iter)
        .and_then(|mut nn| nn.as_mut().next().cloned())
        .unwrap_or(null())
}

/// Returns total size of [`Iter`], it may grow, but never decrease
///
/// Length won't increase after iterator is emptied (`vs_iter_next(iter) == NULL`)
///
/// Returns `0` if pointer to [`Iter`] is `NULL`
///
/// Warning: UB if pointer to [`Iter`] is invalid
///
/// [`Iter`]: ../struct.Iter.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: i32 = 5;
///     let iter = vs_iter(vs);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///
///     assert_eq!(vs_len(vs), 1);
///     assert_eq!(vs_iter_len(iter), 0);
///     assert_eq!(vs_iter_destroy(iter), 0);
///
///     let iter = vs_iter(vs);
///     assert_eq!(vs_iter_len(iter), 1);
///
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vs_len(vs), 4);
///     assert_eq!(vs_iter_len(iter), 4);
///
///     assert_eq!(vs_clear(vs), 0);
///     assert_eq!(vs_len(vs), 0);
///     assert_eq!(vs_iter_len(iter), 4);
///     assert_eq!(vs_iter_destroy(iter), 0);
///
///     let iter = vs_iter(vs);
///     assert_eq!(vs_iter_len(iter), 0);
///
///     assert_eq!(vs_iter_destroy(iter), 0);
///     assert_eq!(vs_destroy(vs), 0);
///
///     // 0 length on NULL pointer
///     assert_eq!(vs_iter_len(null_mut()), 0);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     const unsigned int data = 5;
///
///     vs_iter_t * iter = vs_iter(vs);
///     assert(vs_iter_len(iter) == 0);
///     assert(vs_append(vs, (void *) &data) == 0);
///
///     assert(vs_len(vs) == 1);
///     assert(vs_iter_len(iter) == 0);
///     assert(vs_iter_destroy(iter) == 0);
///
///     vs_iter_t * iter2 = vs_iter(vs);
///     assert(vs_len(vs) == 1);
///
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_append(vs, (void *) &data) == 0);
///     assert(vs_len(vs) == 4);
///     assert(vs_iter_len(iter2) == 4);
///
///     assert(vs_clear() == 0);
///     assert(vs_len(vs) == 0);
///     assert(vs_iter_len(iter2) == 4);
///     assert(vs_iter_destroy(iter2) == 0);
///
///     vs_iter_t * iter3 = vs_iter(vs);
///     assert(vs_iter_len(iter3) == 0);
///
///     assert(vs_iter_destroy(iter3) == 0);
///     assert(vs_destroy(vs) == 0);
///
///     // 0 length on NULL pointer
///     assert(vs_iter_len(NULL) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_iter_len(iter: *const vs_iter_t<'_>) -> usize {
    NonNull::new(iter as *mut vs_iter_t<'_>).map_or(0, |nn| nn.as_ref().len())
}

/// Returns current [`Iter`] index
///
/// Returns `0` if pointer to [`Iter`] is `NULL`
///
/// Warning: UB if pointer to [`Iter`] is invalid
///
/// [`Iter`]: ../struct.Iter.html
///
/// # Rust
///
/// ```rust
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let data: [i32; 3] = [4, 9, 8];
///     assert_eq!(vs_append(vs, &data[0] as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data[1] as *const i32 as *const c_void), 0);
///     assert_eq!(vs_append(vs, &data[2] as *const i32 as *const c_void), 0);
///
///     let iter = vs_iter(vs);
///     assert_eq!(vs_iter_index(iter), 0);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 4);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 9);
///     assert_eq!(*(vs_iter_next(iter) as *const i32), 8);
///     assert_eq!(vs_iter_index(iter), 3);
///
///     assert!(vs_iter_next(iter).is_null());
///     assert_eq!(vs_iter_index(iter), 3);
///     assert_eq!(vs_iter_index(iter), vs_iter_len(iter));
///
///     assert_eq!(vs_iter_destroy(iter), 0);
///     assert_eq!(vs_destroy(vs), 0);
///
///     // 0 index on NULL pointer
///     assert_eq!(vs_iter_index(null_mut()), 0);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     const unsigned int data[3] = { 4, 9, 8 };
///     assert(vs_append(vs, (void *) data) == 0);
///     assert(vs_append(vs, (void *) (data + 1)) == 0);
///     assert(vs_append(vs, (void *) (data + 2)) == 0);
///
///     vs_iter_t * iter = vs_iter(vs);
///     assert(vs_iter_index(iter) == 0);
///     assert(*(unsigned int *) vs_iter_next(iter) == 4);
///     assert(*(unsigned int *) vs_iter_next(iter) == 9);
///     assert(*(unsigned int *) vs_iter_next(iter) == 8);
///     assert(vs_iter_index(iter) == 3);
///
///     assert(vs_iter_next(iter) == NULL);
///     assert(vs_iter_index(iter) == 3);
///     assert(vs_iter_index(iter) == vs_iter_len(iter));
///
///     assert(vs_iter_destroy(iter) == 0);
///     assert(vs_destroy(vs) == 0);
///
///     // 0 index on NULL pointer
///     assert(vs_iter_index(NULL) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_iter_index(iter: *const vs_iter_t<'_>) -> usize {
    NonNull::new(iter as *mut vs_iter_t<'_>).map_or(0, |nn| nn.as_ref().index())
}

/// Free [`Iter`] (can happen after [`VoluntaryServitude`]'s free)
///
/// Returns `1` if pointer to [`VoluntaryServitude`] is `NULL`
///
/// Returns `0` otherwise
///
/// Warning: UB if pointer to [`Iter`] is invalid
///
/// [`VoluntaryServitude`]: ../struct.VoluntaryServitude.html
/// [`Iter`]: ../struct.Iter.html
///
/// # Rust
///
/// ```rust
/// use std::ptr::null_mut;
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vs = vs_new();
///     let iter = vs_iter(vs);
///     assert_eq!(vs_destroy(vs), 0);
///     assert_eq!(vs_iter_destroy(iter), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vs_iter_destroy(null_mut()), 1);
/// }
/// ```
///
/// # C
///
/// ```c
/// #include<assert.h>
/// #include "../include/voluntary_servitude.h"
///
/// int main(int argc, char **argv) {
///     vs_t * vs = vs_new();
///     vs_iter_t * iter = vs_iter(vs);
///     assert(vs_destroy(vs) == 0);
///     assert(vs_iter_destry(iter) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vs_iter_destroy(NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vs_iter_destroy(iter: *mut vs_iter_t<'_>) -> u8 {
    NonNull::new(iter).map_or(1, |nn| (drop_in_place(nn.as_ptr()), 0).1)
}
