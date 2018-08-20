extern crate voluntary_servitude;
use std::{
    os::raw::c_void,
    ptr::{null, null_mut},
};
use voluntary_servitude::ffi::{
    vsread_append, vsread_clear, vsread_destroy, vsread_iter,
    vsread_iter_destroy, vsread_iter_index, vsread_iter_len, vsread_iter_next, vsread_len,
    vsread_new,
};

fn initialize_logger() {
    #[cfg(feature = "logs")] voluntary_servitude::ffi::initialize_logger();
}

#[test]
fn mutability() {
    initialize_logger();
    let vsread = vsread_new();
    let mut data: i32 = 1;
    vsread_append(vsread, &data as *const i32 as *const c_void);
    let iter = vsread_iter(vsread);
    let iter2 = vsread_iter(vsread);
    assert!(!iter.is_null());
    assert!(!iter2.is_null());

    unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), 1) }
    data = 4;
    let _ = data;
    unsafe { assert_eq!(*(vsread_iter_next(iter2) as *const i32), 4) }
    vsread_destroy(vsread);
    vsread_iter_destroy(iter);
    vsread_iter_destroy(iter2);
}

#[test]
fn null_ptr() {
    initialize_logger();
    let vsread = vsread_new();
    assert!(!vsread.is_null());

    let data: i32 = 1;
    vsread_append(null(), &data as *const i32 as *const c_void);
    vsread_append(vsread, &data as *const i32 as *const c_void);
    assert_eq!(vsread_len(null()), 0);
    assert_eq!(vsread_len(vsread), 1);

    assert_eq!(vsread_iter(null()), null_mut());
    let iter = vsread_iter(vsread);
    assert!(!iter.is_null());
    assert_eq!(vsread_iter_index(iter), 0);
    assert_eq!(vsread_iter_len(iter), 1);
    unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), data) }
    assert_eq!(vsread_iter_index(iter), 1);
    assert_eq!(vsread_iter_next(iter), null());
    vsread_iter_destroy(iter);

    vsread_clear(null());
    vsread_clear(vsread);
    assert!(!vsread.is_null());
    assert_eq!(vsread_len(vsread), 0);
    let iter = vsread_iter(vsread);
    assert_eq!(vsread_iter_index(iter), 0);
    assert_eq!(vsread_iter_len(iter), 0);
    assert!(vsread_iter_next(iter).is_null());

    vsread_destroy(vsread);
    vsread_iter_destroy(iter);
}

#[test]
fn new() {
    initialize_logger();
    vsread_destroy(vsread_new());
}

#[test]
fn iter() {
    initialize_logger();
    let new = vsread_new();
    let iter = vsread_iter(new);
    assert_eq!(vsread_iter_index(iter), 0);
    assert!(vsread_iter_next(iter).is_null());
    assert_eq!(vsread_iter_index(iter), 0);
    assert_eq!(vsread_len(new), 0);
    assert_eq!(vsread_iter_len(iter), 0);

    let data: i32 = 32;
    assert_eq!(vsread_iter_len(iter), 0);
    vsread_append(new, &data as *const i32 as *const c_void);
    assert_eq!(vsread_iter_len(iter), 0);
    assert_eq!(vsread_len(new), 1);

    vsread_iter_destroy(iter);
    let iter = vsread_iter(new);
    vsread_append(new, &data as *const i32 as *const c_void);
    assert_eq!(vsread_iter_len(iter), 1);
    assert_eq!(vsread_iter_index(iter), 0);
    assert_eq!(vsread_len(new), 2);
    unsafe { assert_eq!(*(vsread_iter_next(iter) as *const i32), data) }
    assert_eq!(vsread_iter_index(iter), 1);
    assert!(vsread_iter_next(iter).is_null());
    assert_eq!(vsread_iter_index(iter), 1);
    vsread_iter_destroy(iter);

    let data2: i32 = 10;
    let iter = vsread_iter(new);
    assert_eq!(vsread_iter_len(iter), 2);
    vsread_append(new, &data2 as *const i32 as *const c_void);
    assert_eq!(vsread_len(new), 3);
    vsread_append(new, &data as *const i32 as *const c_void);
    assert_eq!(vsread_len(new), 4);
    vsread_append(new, &data2 as *const i32 as *const c_void);
    assert_eq!(vsread_iter_len(iter), 2);
    assert_eq!(vsread_len(new), 5);
    unsafe {
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data);
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data);
    }
    assert!(vsread_iter_next(iter).is_null());

    let iter = vsread_iter(new);
    vsread_clear(new);
    assert_eq!(vsread_len(new), 0);

    assert_eq!(vsread_iter_len(iter), 5);
    unsafe {
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data);
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data);
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data2);
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data);
        assert_eq!(*(vsread_iter_next(iter) as *const i32), data2);
    }
    assert!(vsread_iter_next(iter).is_null());

    let iter = vsread_iter(new);
    assert!(vsread_iter_next(iter).is_null());
    assert_eq!(vsread_iter_len(iter), 0);

    vsread_destroy(null_mut());
    vsread_destroy(new);
    vsread_iter_destroy(null_mut());
    vsread_iter_destroy(iter);
}
