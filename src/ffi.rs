//! Voluntary Servitude Foreign Function Interface (FFI)
//!
//! Allows using this rust library as a C library
//!
//! # Single-thread C implementation
//!
//! ```c
//! #include<assert.h>
//! #include "../include/voluntary_servitude.h"
//!
//! int main(int argc, char **argv) {
//!     // Rust allocates memory through malloc
//!     vsread_t * const vsread = vsread_new();
//!
//!     // Current vsread_t length
//!     // Be careful with data-races since the value, when used, may not be true anymore
//!     assert(vsread_len(vsread) == 0);
//!
//!     const unsigned int data[2] = {12, 25};
//!     // Inserts void pointer to data to end of vsread_t
//!     vsread_append(vsread, (void *) data);
//!     assert(vsread_len(vsread) == 1);
//!     vsread_append(vsread, (void *) (data + 1));
//!     assert(vsread_len(vsread) == 2);
//!
//!     // Creates a one-time lock-free iterator based on vsread_t
//!     vsread_iter_t * const iter = vsread_iter(vsread);
//!     // Index changes as you iter through vsread_iter_t
//!     assert(vsread_iter_index(iter) == 0);
//!
//!     // Clears vsread_t, doesn't change existing iterators
//!     vsread_clear(vsread);
//!     assert(vsread_len(vsread) == 0);
//!     assert(vsread_iter_len(iter) == 2);
//!
//!     assert(*(unsigned int *) vsread_iter_next(iter) == 12);
//!     assert(vsread_iter_index(iter) == 1);
//!
//!     assert(*(unsigned int *) vsread_iter_next(iter) == 25);
//!     assert(vsread_iter_index(iter) == 2);
//!
//!     // Next can be called after there are no more elements (NULL pointer returned), but nothing happens
//!     assert(vsread_iter_next(iter) == NULL);
//!     assert(vsread_iter_next(iter) == NULL);
//!     assert(vsread_iter_index(iter) == 2);
//!
//!     // Never forget to free vsread_iter_t
//!     vsread_iter_destroy(iter);
//!
//!     // Create updated vsread_iter_t
//!     vsread_iter_t * const iter2 = vsread_iter(vsread);
//!
//!     // Never forget to free vsread_t
//!     vsread_destroy(vsread);
//!
//!     assert(vsread_iter_len(iter2) == 0);
//!     assert(vsread_iter_next(iter2) == NULL);
//!
//!     vsread_iter_destroy(iter2);
//!
//!     // Unused arguments
//!     (void) argc;
//!     (void) argv;
//!     return 0;
//! }
//! ```
//!
//! # Multi-thread C implementation
//!
//! ```c
//! #include<pthread.h>
//! #include<stdio.h>
//! #include "../include/voluntary_servitude.h"
//!
//! const unsigned int num_producers = 4;
//! const unsigned int num_consumers = 8;
//!
//! const unsigned int num_producer_values = 1000;
//! const unsigned int data[3] = {12, 25, 89};
//!
//! void* producer();
//! void* consumer();
//!
//! int main(int argc, char** argv)
//! {
//!     // Rust allocates memory through malloc
//!     vsread_t * const vsread = vsread_new();
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
//!         if (pthread_create(&producers[current_thread], &attr, &producer, (void *) vsread) != 0) {
//!             fprintf(stderr, "Failed to create producer thread %d.\n", current_thread);
//!             exit(-2);
//!         }
//!
//!     }
//!
//!     // Creates consumers threads
//!     for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
//!         if (pthread_create(&consumers[current_thread], &attr, &consumer, (void *) vsread) != 0) {
//!             fprintf(stderr, "Failed to create consumer thread %d.\n", current_thread);
//!             exit(-3);
//!         }
//!     }
//!
//!     // Join all threads, ensuring vsread_t* is not used anymore
//!     for (current_thread = 0; current_thread < num_producers; ++current_thread) {
//!         pthread_join(producers[current_thread], NULL);
//!     }
//!     for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
//!         pthread_join(consumers[current_thread], NULL);
//!     }
//!
//!     // Never forget to free the memory allocated through rust
//!     vsread_destroy(vsread);
//!
//!     (void) argc;
//!     (void) argv;
//!     return 0;
//! }
//!
//! void * producer(void * const vsread){
//!     unsigned int index;
//!     for (index = 0; index < num_producer_values; ++index) {
//!         vsread_append(vsread, (void *) (data + (index % 2)));
//!     }
//!     return NULL;
//! }
//!
//! void * consumer(void * const vsread) {
//!     const unsigned int total_values = num_producers * num_producer_values;
//!     unsigned int values = 0;
//!     while (values < total_values) {
//!         unsigned int sum = (values = 0);
//!         vsread_iter_t * const iter = vsread_iter(vsread);
//!         const void * value;
//!         while ((value = vsread_iter_next(iter)) != NULL) {
//!             ++values;
//!             sum += *(unsigned int *) value;
//!         }
//!         printf("Consumer counts %d elements summing %d.\n", values, sum);
//!
//!         vsread_iter_destroy(iter);
//!     }
//!     return NULL;
//! }
//! ```

use iter::VSReadIter;
use std::{
    mem::drop,
    os::raw::c_void,
    ptr::{null, null_mut},
};
use vsread::VSRead;

/// Initialize logger according to RUST_LOG env var (only exists 'logs' feature)
///
/// Currently there is no warning and logging is stripped at compile time in release
///
/// ```bash
/// export RUST_LOG=voluntary_servitude=trace
/// export RUST_LOG=voluntary_servitude=debug
/// export RUST_LOG=voluntary_servitude=info
/// ```
///
/// Feature to enable it:
///
///```bash
/// cargo build --features "logs"
/// ```
///
/// ```
/// use voluntary_servitude::ffi::*;
/// unsafe { initialize_logger() }
/// ```
#[no_mangle]
#[cfg(feature = "logs")]
pub unsafe extern "C" fn initialize_logger() {
    ::setup_logger();
}

/// Creates new empty VSRead (thread-safe appendable list with lock-free iterator)
///
/// vsread_drop should be called eventually for VSRead returned, otherwise memory will leak
///
/// # Rust
///
/// ```
/// use voluntary_servitude::ffi::*;
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///
///     let vsread = vsread_new();
///     assert_eq!(vsread_len(vsread), 0);
///     assert_eq!(vsread_destroy(vsread), 0);
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
///     vsread_t * const vsread = vsread_new();
///     assert(vsread_len(vsread) == 0);
///     assert(vsread_destroy(vsread) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_new() -> *mut VSRead<*const c_void> {
    Box::into_raw(Box::new(vsread![]))
}

/// Makes lock-free iterator based on VSRead
///
/// vsread_iter_drop should be called eventually for VSReadIter returned, otherwise memory will leak
///
/// Returns NULL if pointer to VSRead is NULL
///
/// Warning: UB if pointer to VSRead is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///
///     let iter = vsread_iter(vsread);
///     assert!(!iter.is_null());
///     assert!(vsread_iter_next(iter).is_null());
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     let data: i32 = 3;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     let iter = vsread_iter(vsread);
///     unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 3) }
///     assert!(vsread_iter_next(iter).is_null());
///     assert_eq!(vsread_destroy(vsread), 0);
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     // Propagates NULL pointers
///     assert_eq!(vsread_iter(null_mut()), null_mut());
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
///     vsread_t * const vsread = vsread_new();
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     assert(iter != NULL);
///     assert(vsread_iter_next(iter) == NULL);
///     assert(vsread_iter_destroy(iter) == 0);
///
///     const unsigned int data = 3;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     vsread_iter_t * const iter2 = vsread_iter(vsread);
///     assert(*(unsigned int *) vsread_iter_next(iter2) == 3);
///     assert(vsread_iter_next(iter2) == NULL);
///
///     assert(vsread_destroy(iter) == 0);
///     assert(vsread_iter_destroy(iter2) == 0);
///
///     // Propagates NULL pointers
///     assert(vsread_iter(NULL) == NULL);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_iter<'a>(
    vsread: *const VSRead<*const c_void>,
) -> *mut VSReadIter<'a, *const c_void> {
    if vsread.is_null() {
        return null_mut();
    }
    let vsread = &*vsread;
    Box::into_raw(Box::new(vsread.iter()))
}

/// Atomically extracts current size of VSRead, be careful with data-races when using it
///
/// Returns 0 if pointer to VSRead is NULL
///
/// Warning: UB if pointer to VSRead invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     assert_eq!(vsread_len(vsread), 0);
///     let data: i32 = 5;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_len(vsread), 1);
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // 0 length on NULL pointer
///     assert_eq!(vsread_len(null()), 0);
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
///     vsread_t * const vsread = vsread_new();
///     assert(vsread_len(vsread) == 0);
///
///     const unsigned int data = 5;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_len(vsread) == 1);
///     assert(vsread_destroy(vsread) == 0);
///
///     // 0 length on NULL pointer
///     assert(vsread_len(NULL) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_len(list: *const VSRead<*const c_void>) -> usize {
    if list.is_null() {
        return 0;
    }
    let list = &*list;
    list.len()
}

/// Append element to VSRead, locks other writes
///
/// Returns 1 if pointer to VSRead is NULL
///
/// Returns 0 otherwise
///
/// Warning: UB if pointer to VSRead is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::{null, null_mut}, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     let mut data: i32 = 5;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_len(vsread), 1);
///
///     let iter = vsread_iter(vsread);
///     unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 5) }
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     let iter = vsread_iter(vsread);
///     data = 2;
///     unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 2) }
///     assert_eq!(vsread_iter_destroy(iter), 0);
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vsread_append(null_mut(), &data as *const i32 as *const c_void), 1);
///     assert_eq!(vsread_append(null_mut(), null()), 1);
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
///     vsread_t * const vsread = vsread_new();
///     unsigned int data = 5;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_len(vsread) == 1);
///
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     assert(*(unsigned int *) vsread_iter_next(iter) == 5);
///     assert(vsread_iter_destroy(iter) == 0);
///
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     data = 2;
///     assert(*(unsigned int *) vsread_iter_next(iter) == 2);
///     assert(vsread_iter_destroy(iter) == 0);
///     assert(vsread_destroy(vsread) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vsread_append(NULL, (void *) &data) == 1);
///     assert(vsread_append(NULL, NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_append(
    list: *const VSRead<*const c_void>,
    element: *const c_void,
) -> u8 {
    if list.is_null() {
        return 1;
    }
    let list = &*list;
    list.append(element);
    0
}

/// Remove all elements from list, locks other writes
///
/// Returns 1 if pointer to VSRead is NULL
///
/// Returns 0 otherwise
///
/// Warning: UB if pointer to VSRead is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     let mut data: i32 = 5;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_len(vsread), 1);
///     assert_eq!(vsread_clear(vsread), 0);
///     assert_eq!(vsread_len(vsread), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vsread_clear(null()), 1);
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
///     vsread_t * const vsread = vsread_new();
///     unsigned int data = 5;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_len(vsread) == 1);
///     assert(vsread_clear(vsread) == 0);
///     assert(vsread_len(vsread) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vsread_clear(NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_clear(list: *const VSRead<*const c_void>) -> u8 {
    if list.is_null() {
        return 1;
    }
    let list = &*list;
    list.clear();
    0
}

/// Free VSRead
///
/// Returns 1 if pointer to VSRead is NULL
///
/// Returns 0 otherwise
///
/// Warning: UB if pointer to VSRead is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     let mut data: i32 = 5;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_len(vsread), 1);
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vsread_destroy(null_mut()), 1);
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
///     vsread_t * const vsread = vsread_new();
///     unsigned int data = 5;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_len(vsread) == 1);
///     assert(vsread_destroy(vsread) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vsread_destroy(NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_destroy(list: *mut VSRead<*const c_void>) -> u8 {
    if list.is_null() {
        return 1;
    }
    let list = Box::from_raw(list);
    drop(list);
    0
}

/// Obtain next element in iter, returns NULL if there are no more elements
///
/// Returns NULL if pointer to VSReadIter is NULL
///
/// Warning: UB if pointer to VSReadIter is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     let mut data: i32 = 5;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///
///     let iter = vsread_iter(vsread);
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 5);
///     assert!(vsread_iter_next(iter).is_null());
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     let iter = vsread_iter(vsread);
///     data = 2;
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 2);
///     assert_eq!(vsread_iter_destroy(iter), 0);
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // Propagates NULL pointers
///     assert!(vsread_iter_next(null_mut()).is_null());
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
///     vsread_t * const vsread = vsread_new();
///     unsigned int data = 5;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     assert(*(unsigned int *) vsread_iter_next(iter) == 5);
///     assert(vsread_iter_next(iter) == NULL);
///     assert(vsread_iter_destroy(iter) == 0);
///
///     vsread_iter_t * const iter2 = vsread_iter(vsread);
///     data = 2;
///     assert(*(unsigned int *) vsread_iter_next(iter) == 2);
///     assert(vsread_iter_destroy(iter) == 0);
///     assert(vsread_destroy(vsread) == 0);
///
///     // Propagates NULL pointers
///     assert(vsread_iter_next(NULL) == NULL);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_iter_next(
    iter: *mut VSReadIter<'_, *const c_void>,
) -> *const c_void {
    if iter.is_null() {
        return null();
    }
    let iter = &mut *iter;
    match iter.next() {
        Some(pointer) => *pointer,
        None => null(),
    }
}

/// Returns total size of iterator, this never changes
///
/// Returns 0 if pointer to VSReadIter is NULL
///
/// Warning: UB if pointer to VSReadIter is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     assert_eq!(vsread_len(vsread), 0);
///     let iter = vsread_iter(vsread);
///     assert_eq!(vsread_iter_len(iter), 0);
///
///     let mut data: i32 = 5;
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_append(vsread, &data as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_len(vsread), 3);
///     assert_eq!(vsread_iter_len(iter), 0);
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     let iter = vsread_iter(vsread);
///     assert_eq!(vsread_iter_len(iter), 3);
///     assert_eq!(vsread_iter_destroy(iter), 0);
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // 0 length on NULL pointer
///     assert_eq!(vsread_iter_len(null_mut()), 0);
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
///     vsread_t * const vsread = vsread_new();
///     assert(vsread_len(vsread) == 0);
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     assert(vsread_iter_len(iter) == 0);
///
///     const unsigned int data = 5;
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_append(vsread, (void *) &data) == 0);
///     assert(vsread_len(vsread) == 3);
///     assert(vsread_iter_len(iter) == 0);
///     assert(vsread_iter_destroy(iter) == 0);
///
///     vsread_iter_t * const iter2 = vsread_iter(vsread);
///     assert(vsread_iter_len(iter2) == 3);
///     assert(vsread_iter_destroy(iter2) == 0);
///     assert(vsread_destroy(vsread) == 0);
///
///     // 0 length on NULL pointer
///     assert(vsread_iter_len(NULL) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_iter_len(iter: *const VSReadIter<'_, *const c_void>) -> usize {
    if iter.is_null() {
        return 0;
    }
    let iter = &*iter;
    iter.len()
}

/// Returns current iterator index
///
/// Returns 0 if pointer to VSReadIter is NULL
///
/// Warning: UB if pointer to VSReadIter is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     let data: [i32; 3] = [4, 9, 8];
///     assert_eq!(vsread_append(vsread, &data[0] as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_append(vsread, &data[1] as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_append(vsread, &data[2] as *const i32 as *const c_void), 0);
///
///     let iter = vsread_iter(vsread);
///     assert_eq!(vsread_iter_index(iter), 0);
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 4);
///     assert_eq!(vsread_iter_index(iter), 1);
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 9);
///     assert_eq!(vsread_iter_index(iter), 2);
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 8);
///     assert_eq!(vsread_iter_index(iter), 3);
///     assert!(vsread_iter_next(iter).is_null());
///     assert_eq!(vsread_iter_index(iter), 3);
///     assert_eq!(vsread_iter_index(iter), vsread_iter_len(iter));
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // 0 index on NULL pointer
///     assert_eq!(vsread_iter_index(null_mut()), 0);
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
///     vsread_t * const vsread = vsread_new();
///     unsigned int data[3] = { 4, 9, 8 };
///     assert(vsread_append(vsread, (void *) data) == 0);
///     assert(vsread_append(vsread, (void *) (data + 1)) == 0);
///     assert(vsread_append(vsread, (void *) (data + 2)) == 0);
///
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     assert(vsread_iter_index(iter) == 0);
///     assert(*(unsigned int *) vsread_iter_next(iter) == 4);
///     assert(vsread_iter_index(iter) == 1);
///     assert(*(unsigned int *) vsread_iter_next(iter) == 9);
///     assert(vsread_iter_index(iter) == 2);
///     assert(*(unsigned int *) vsread_iter_next(iter) == 8);
///     assert(vsread_iter_index(iter) == 3);
///
///     assert(vsread_iter_next(iter) == NULL);
///     assert(vsread_iter_index(iter) == 3);
///     assert(vsread_iter_index(iter) == vsread_iter_len(iter));
///     assert(vsread_iter_destroy(iter) == 0);
///
///     assert(vsread_destroy(vsread) == 0);
///
///     // 0 index on NULL pointer
///     assert(vsread_iter_index(NULL) == 0);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_iter_index(iter: *const VSReadIter<'_, *const c_void>) -> usize {
    if iter.is_null() {
        return 0;
    }
    let iter = &*iter;
    iter.index()
}

/// Free VSReadIter
///
/// Returns 1 if pointer to VSRead is NULL
///
/// Returns 0 otherwise
///
/// Warning: UB if pointer to VSReadIter is invalid
///
/// # Rust
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// unsafe {
///     # #[cfg(feature = "logs")] initialize_logger();
///     let vsread = vsread_new();
///     let data: [i32; 3] = [4, 9, 8];
///     assert_eq!(vsread_append(vsread, &data[0] as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_append(vsread, &data[1] as *const i32 as *const c_void), 0);
///     assert_eq!(vsread_append(vsread, &data[2] as *const i32 as *const c_void), 0);
///
///     let iter = vsread_iter(vsread);
///     assert_eq!(vsread_iter_len(iter), 3);
///     assert_eq!(vsread_iter_destroy(iter), 0);
///
///     assert_eq!(vsread_destroy(vsread), 0);
///
///     // Returns 1 on NULL pointer
///     assert_eq!(vsread_iter_destroy(null_mut()), 1);
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
///     vsread_t * const vsread = vsread_new();
///     unsigned int data[3] = { 4, 9, 8 };
///     assert(vsread_append(vsread, (void *) data) == 0);
///     assert(vsread_append(vsread, (void *) (data + 1)) == 0);
///     assert(vsread_append(vsread, (void *) (data + 2)) == 0);
///
///     vsread_iter_t * const iter = vsread_iter(vsread);
///     assert(vsread_iter_len(iter) == 3);
///     assert(vsread_iter_destry(iter) == 0);
///
///     assert(vsread_destroy(vsread) == 0);
///
///     // Returns 1 on NULL pointer
///     assert(vsread_iter_destroy(NULL) == 1);
///     return 0;
/// }
/// ```
#[no_mangle]
pub unsafe extern "C" fn vsread_iter_destroy(iter: *mut VSReadIter<'_, *const c_void>) -> u8 {
    if iter.is_null() {
        return 1;
    }
    let iter = Box::from_raw(iter);
    drop(iter);
    0
}
