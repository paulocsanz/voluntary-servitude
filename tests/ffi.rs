#[cfg(feature = "ffi")]
extern crate voluntary_servitude;
#[cfg(feature = "ffi")]
use std::{os::raw::c_void, ptr::drop_in_place, ptr::null_mut};
#[cfg(feature = "ffi")]
use voluntary_servitude::ffi::*;

#[cfg(feature = "ffi")]
fn initialize_logger() {
    #[cfg(feature = "logs")]
    unsafe {
        voluntary_servitude::ffi::initialize_logger()
    }
}

#[cfg(feature = "ffi")]
unsafe extern "C" fn free(ptr: *mut c_void) {
    drop_in_place(ptr);
}

#[test]
#[cfg(feature = "ffi")]
fn drop_elements() {
    unsafe {
        initialize_logger();
        let vs = vs_new(Some(free));
        assert_eq!(vs_append(vs, Box::into_raw(10.into()) as *mut c_void), 0);
        assert_eq!(vs_append(vs, Box::into_raw(14.into()) as *mut c_void), 0);
        assert_eq!(vs_append(vs, Box::into_raw(8.into()) as *mut c_void), 0);
        assert_eq!(vs_append(vs, Box::into_raw(1.into()) as *mut c_void), 0);
        let iter = vs_iter(vs);
        assert_eq!(vs_iter_destroy(iter), 0);
        assert_eq!(vs_destroy(vs), 0);

        let vs = vs_new(Some(free));
        assert_eq!(vs_append(vs, Box::into_raw(10.into()) as *mut c_void), 0);
        assert_eq!(vs_append(vs, Box::into_raw(14.into()) as *mut c_void), 0);
        assert_eq!(vs_append(vs, Box::into_raw(8.into()) as *mut c_void), 0);
        assert_eq!(vs_append(vs, Box::into_raw(1.into()) as *mut c_void), 0);
        let iter = vs_iter(vs);
        assert_eq!(vs_destroy(vs), 0);
        assert_eq!(vs_iter_destroy(iter), 0);
    }
}

#[test]
#[cfg(feature = "ffi")]
fn mutability() {
    unsafe {
        initialize_logger();
        let vs = vs_new(Some(free));
        let mut data: i32 = 1;
        assert_eq!(vs_append(vs, &mut data as *mut i32 as *mut c_void), 0);
        let iter = vs_iter(vs);
        let iter2 = vs_iter(vs);
        assert!(!iter.is_null());
        assert!(!iter2.is_null());

        assert_eq!(*(vs_iter_next(iter) as *mut i32), 1);
        data = 4;
        assert_eq!(*(vs_iter_next(iter2) as *mut i32), 4);
        assert_eq!(vs_destroy(vs), 0);
        assert_eq!(vs_iter_destroy(iter), 0);
        assert_eq!(vs_iter_destroy(iter2), 0);
        let _ = data;
    }
}

#[test]
#[cfg(feature = "ffi")]
fn null_ptr() {
    unsafe {
        initialize_logger();
        let vs = vs_new(Some(free));
        assert!(!vs.is_null());

        let data = Box::into_raw(Box::new(1)) as *mut c_void;
        assert_eq!(vs_append(null_mut(), data), 1);
        assert_eq!(vs_append(vs, data), 0);
        assert_eq!(vs_len(null_mut()), 0);
        assert_eq!(vs_len(vs), 1);

        assert_eq!(vs_iter(null_mut()), null_mut());
        let iter = vs_iter(vs);
        assert!(!iter.is_null());
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_iter_len(iter), 1);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 1);
        assert_eq!(vs_iter_index(iter), 1);
        assert_eq!(vs_iter_next(iter), null_mut());
        assert_eq!(vs_iter_destroy(iter), 0);

        assert_eq!(vs_clear(null_mut()), 1);
        assert_eq!(vs_clear(vs), 0);
        assert!(!vs.is_null());
        assert_eq!(vs_len(vs), 0);
        let iter = vs_iter(vs);
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_iter_len(iter), 0);
        assert!(vs_iter_next(iter).is_null());

        assert_eq!(vs_destroy(vs), 0);
        assert_eq!(vs_iter_destroy(iter), 0);
    }
}

#[test]
#[cfg(feature = "ffi")]
fn new() {
    unsafe {
        initialize_logger();
        vs_destroy(vs_new(None));
        vs_destroy(vs_new(Some(free)));
    }
}

#[test]
#[cfg(feature = "ffi")]
fn iter() {
    unsafe {
        initialize_logger();
        let new = vs_new(Some(free));
        let iter = vs_iter(new);
        assert_eq!(vs_iter_index(iter), 0);
        assert!(vs_iter_next(iter).is_null());
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_len(new), 0);
        assert_eq!(vs_iter_len(iter), 0);

        let data = Box::into_raw(Box::new(32)) as *mut c_void;
        assert_eq!(vs_iter_len(iter), 0);
        assert_eq!(vs_append(new, data), 0);
        assert_eq!(vs_iter_len(iter), 0);
        assert_eq!(vs_len(new), 1);

        vs_iter_destroy(iter);
        let iter = vs_iter(new);
        assert_eq!(vs_append(new, data), 0);
        assert_eq!(vs_iter_len(iter), 2);
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_len(new), 2);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(vs_iter_index(iter), 1);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert!(vs_iter_next(iter).is_null());
        assert_eq!(vs_iter_index(iter), 2);
        assert_eq!(vs_iter_destroy(iter), 0);

        let data2 = Box::into_raw(Box::new(10)) as *mut c_void;
        let iter = vs_iter(new);
        assert_eq!(vs_iter_len(iter), 2);
        assert_eq!(vs_append(new, data2), 0);
        assert_eq!(vs_append(new, data), 0);
        assert_eq!(vs_append(new, data2), 0);
        assert_eq!(vs_iter_len(iter), 5);
        assert_eq!(vs_len(new), 5);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 10);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 10);
        assert!(vs_iter_next(iter).is_null());

        assert_eq!(vs_iter_destroy(iter), 0);
        let iter = vs_iter(new);
        assert_eq!(vs_clear(new), 0);
        assert_eq!(vs_len(new), 0);

        assert_eq!(vs_iter_len(iter), 5);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 10);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 32);
        assert_eq!(*(vs_iter_next(iter) as *mut i32), 10);
        assert!(vs_iter_next(iter).is_null());

        assert_eq!(vs_iter_destroy(iter), 0);
        let iter = vs_iter(new);
        assert!(vs_iter_next(iter).is_null());
        assert_eq!(vs_iter_len(iter), 0);

        assert_eq!(vs_destroy(null_mut()), 1);
        assert_eq!(vs_destroy(new), 0);
        assert_eq!(vs_iter_destroy(null_mut()), 1);
        assert_eq!(vs_iter_destroy(iter), 0);
    }
}
