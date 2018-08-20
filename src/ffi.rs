//! Voluntary Servitude Foreign Function Interface (FFI)
//!
//! Allows using this rust library as a C library

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
/// initialize_logger();
/// ```
#[no_mangle]
#[cfg(feature = "logs")]
pub extern "C" fn initialize_logger() {
    ::setup_logger();
}

/// Creates new empty VSRead (thread-safe appendable list with lock-free iterator)
///
/// vsread_drop should be called eventually for VSRead returned, otherwise memory will leak
///
/// ```
/// use voluntary_servitude::ffi::*;
/// # #[cfg(feature = "logs")] initialize_logger();
///
/// let vsread = vsread_new();
/// assert_eq!(vsread_len(vsread), 0);
/// vsread_destroy(vsread);
/// ```
#[no_mangle]
pub extern "C" fn vsread_new() -> *mut VSRead<*const c_void> {
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
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
///
/// let iter = vsread_iter(vsread);
/// assert!(!iter.is_null());
/// assert!(vsread_iter_next(iter).is_null());
///
/// let data: i32 = 3;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// let iter = vsread_iter(vsread);
/// unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 3) }
/// assert!(vsread_iter_next(iter).is_null());
/// vsread_destroy(vsread);
/// vsread_iter_destroy(iter);
///
/// assert_eq!(vsread_iter(null_mut()), null_mut());
/// ```
#[no_mangle]
pub extern "C" fn vsread_iter<'a>(
    vsread: *const VSRead<*const c_void>,
) -> *mut VSReadIter<'a, *const c_void> {
    if vsread.is_null() {
        return null_mut();
    }
    let vsread = unsafe { &*vsread };
    Box::into_raw(Box::new(vsread.iter()))
}

/// Atomically extracts current size of VSRead, be careful with data-races when using it
///
/// Returns 0 if pointer to VSRead is NULL
///
/// Warning: UB if pointer to VSRead invalid
///
/// ```
/// use std::{ptr::null, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// assert_eq!(vsread_len(vsread), 0);
/// let data: i32 = 5;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// assert_eq!(vsread_len(vsread), 1);
/// vsread_destroy(vsread);
///
/// assert_eq!(vsread_len(null()), 0);
/// ```
#[no_mangle]
pub extern "C" fn vsread_len(list: *const VSRead<*const c_void>) -> usize {
    if list.is_null() {
        return 0;
    }
    let list = unsafe { &*list };
    list.len()
}

/// Append element to VSRead, locks other writes
///
/// Returns early if pointer to VSRead is NULL
///
/// Warning: UB if pointer to VSRead is invalid
///
/// ```
/// use std::{ptr::{null, null_mut}, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// let mut data: i32 = 5;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// assert_eq!(vsread_len(vsread), 1);
///
/// let iter = vsread_iter(vsread);
/// unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 5) }
/// vsread_iter_destroy(iter);
///
/// let iter = vsread_iter(vsread);
/// data = 2;
/// unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 2) }
/// vsread_iter_destroy(iter);
/// vsread_destroy(vsread);
///
/// // Does nothing
/// vsread_append(null_mut(), &data as *const i32 as *const c_void);
/// vsread_append(null_mut(), null());
/// ```
#[no_mangle]
pub extern "C" fn vsread_append(list: *const VSRead<*const c_void>, element: *const c_void) {
    if list.is_null() {
        return;
    }
    let list = unsafe { &*list };
    list.append(element);
}

/// Remove all elements from list, locks other writes
///
/// Returns early if pointer to VSRead is NULL
///
/// Warning: UB if pointer to VSRead is invalid
///
/// ```
/// use std::{ptr::null, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// let mut data: i32 = 5;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// assert_eq!(vsread_len(vsread), 1);
/// vsread_clear(vsread);
/// assert_eq!(vsread_len(vsread), 0);
///
/// // Does nothing
/// vsread_clear(null());
/// ```
#[no_mangle]
pub extern "C" fn vsread_clear(list: *const VSRead<*const c_void>) {
    if list.is_null() {
        return;
    }
    let list = unsafe { &*list };
    list.clear();
}

/// Free VSRead
///
/// Returns early if pointer to VSRead is NULL
///
/// Warning: UB if pointer to VSRead is invalid
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// let mut data: i32 = 5;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// assert_eq!(vsread_len(vsread), 1);
/// vsread_destroy(vsread);
///
/// // Does nothing
/// vsread_destroy(null_mut());
/// ```
#[no_mangle]
pub extern "C" fn vsread_destroy(list: *mut VSRead<*const c_void>) {
    if list.is_null() {
        return;
    }
    let list = unsafe { Box::from_raw(list) };
    drop(list);
}

/// Obtain next element in iter, returns NULL if there are no more elements
///
/// Returns NULL if pointer to VSReadIter is NULL
///
/// Warning: UB if pointer to VSReadIter is invalid
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// let mut data: i32 = 5;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
///
/// let iter = vsread_iter(vsread);
/// unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 5) }
/// assert!(vsread_iter_next(iter).is_null());
/// vsread_iter_destroy(iter);
///
/// let iter = vsread_iter(vsread);
/// data = 2;
/// unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 2) }
/// vsread_iter_destroy(iter);
/// vsread_destroy(vsread);
///
/// assert!(vsread_iter_next(null_mut()).is_null());
/// ```
#[no_mangle]
pub extern "C" fn vsread_iter_next(iter: *mut VSReadIter<'_, *const c_void>) -> *const c_void {
    if iter.is_null() {
        return null();
    }
    let iter = unsafe { &mut *iter };
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
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// assert_eq!(vsread_len(vsread), 0);
/// let iter = vsread_iter(vsread);
/// assert_eq!(vsread_iter_len(iter), 0);
///
/// let mut data: i32 = 5;
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// vsread_append(vsread, &data as *const i32 as *const c_void);
/// assert_eq!(vsread_len(vsread), 3);
/// assert_eq!(vsread_iter_len(iter), 0);
/// vsread_iter_destroy(iter);
///
/// let iter = vsread_iter(vsread);
/// assert_eq!(vsread_iter_len(iter), 3);
/// vsread_iter_destroy(iter);
/// vsread_destroy(vsread);
///
/// assert_eq!(vsread_iter_len(null_mut()), 0);
/// ```
#[no_mangle]
pub extern "C" fn vsread_iter_len(iter: *const VSReadIter<'_, *const c_void>) -> usize {
    if iter.is_null() {
        return 0;
    }
    let iter = unsafe { &*iter };
    iter.len()
}

/// Returns current iterator index
///
/// Returns 0 if pointer to VSReadIter is NULL
///
/// Warning: UB if pointer to VSReadIter is null or invalid
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// let data: [i32; 3] = [4, 9, 8];
/// vsread_append(vsread, &data[0] as *const i32 as *const c_void);
/// vsread_append(vsread, &data[1] as *const i32 as *const c_void);
/// vsread_append(vsread, &data[2] as *const i32 as *const c_void);
///
/// let iter = vsread_iter(vsread);
/// assert_eq!(vsread_iter_index(iter), 0);
/// unsafe {
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 4);
///     assert_eq!(vsread_iter_index(iter), 1);
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 9);
///     assert_eq!(vsread_iter_index(iter), 2);
///     assert_eq!(*(vsread_iter_next(iter) as *const i32), 8);
///     assert_eq!(vsread_iter_index(iter), 3);
/// }
/// assert!(vsread_iter_next(iter).is_null());
/// assert_eq!(vsread_iter_index(iter), 3);
/// assert_eq!(vsread_iter_index(iter), vsread_iter_len(iter));
/// vsread_iter_destroy(iter);
///
/// vsread_destroy(vsread);
///
/// assert_eq!(vsread_iter_index(null_mut()), 0);
/// ```
#[no_mangle]
pub extern "C" fn vsread_iter_index(iter: *const VSReadIter<'_, *const c_void>) -> usize {
    if iter.is_null() {
        return 0;
    }
    let iter = unsafe { &*iter };
    iter.index()
}

/// Free VSReadIter
///
/// Returns early if pointer to VSReadIter is NULL
///
/// Warning: UB if pointer to VSReadIter is invalid
///
/// ```
/// use std::{ptr::null_mut, os::raw::c_void};
/// use voluntary_servitude::ffi::*;
///
/// # #[cfg(feature = "logs")] initialize_logger();
/// let vsread = vsread_new();
/// let data: [i32; 3] = [4, 9, 8];
/// vsread_append(vsread, &data[0] as *const i32 as *const c_void);
/// vsread_append(vsread, &data[1] as *const i32 as *const c_void);
/// vsread_append(vsread, &data[2] as *const i32 as *const c_void);
///
/// let iter = vsread_iter(vsread);
/// assert_eq!(vsread_iter_len(iter), 3);
/// vsread_iter_destroy(iter);
///
/// vsread_destroy(vsread);
///
/// // Does nothing
/// vsread_iter_destroy(null_mut());
/// ```
#[no_mangle]
pub extern "C" fn vsread_iter_destroy(iter: *mut VSReadIter<'_, *const c_void>) {
    if iter.is_null() {
        return;
    }
    let iter = unsafe { Box::from_raw(iter) };
    drop(iter);
}
