extern crate voluntary_servitude;
use std::ptr::null_mut;
use voluntary_servitude::ffi::*;

fn initialize_logger() {
    #[cfg(feature = "logs")]
    unsafe {
        voluntary_servitude::ffi::initialize_logger()
    }
}

#[test]
fn mutability() {
    unsafe {
        initialize_logger();
        let vs = vs_new();
        let mut data: i32 = 1;
        vs_append(vs, &data as *const i32 as *const c_void);
        let iter = vs_iter(vs);
        let iter2 = vs_iter(vs);
        assert!(!iter.is_null());
        assert!(!iter2.is_null());

        assert_eq!(*(vs_iter_next(iter) as *const i32), 1);
        data = 4;
        let _ = data;
        assert_eq!(*(vs_iter_next(iter2) as *const i32), 4);
        vs_destroy(vs);
        vs_iter_destroy(iter);
        vs_iter_destroy(iter2);
    }
}

#[test]
fn null_ptr() {
    unsafe {
        initialize_logger();
        let vs = vs_new();
        assert!(!vs.is_null());

        let data: i32 = 1;
        vs_append(null_mut(), &data as *const i32 as *const c_void);
        vs_append(vs, &data as *const i32 as *const c_void);
        assert_eq!(vs_len(null_mut()), 0);
        assert_eq!(vs_len(vs), 1);

        assert_eq!(vs_iter(null_mut()), null_mut());
        let iter = vs_iter(vs);
        assert!(!iter.is_null());
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_iter_len(iter), 1);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(vs_iter_index(iter), 1);
        assert_eq!(vs_iter_next(iter), null_mut());
        vs_iter_destroy(iter);

        vs_clear(null_mut());
        vs_clear(vs);
        assert!(!vs.is_null());
        assert_eq!(vs_len(vs), 0);
        let iter = vs_iter(vs);
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_iter_len(iter), 0);
        assert!(vs_iter_next(iter).is_null());

        vs_destroy(vs);
        vs_iter_destroy(iter);
    }
}

#[test]
fn new() {
    unsafe {
        initialize_logger();
        vs_destroy(vs_new());
    }
}

#[test]
fn iter() {
    unsafe {
        initialize_logger();
        let new = vs_new();
        let iter = vs_iter(new);
        assert_eq!(vs_iter_index(iter), 0);
        assert!(vs_iter_next(iter).is_null());
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_len(new), 0);
        assert_eq!(vs_iter_len(iter), 0);

        let data: i32 = 32;
        assert_eq!(vs_iter_len(iter), 0);
        vs_append(new, &data as *const i32 as *const c_void);
        assert_eq!(vs_iter_len(iter), 0);
        assert_eq!(vs_len(new), 1);

        vs_iter_destroy(iter);
        let iter = vs_iter(new);
        vs_append(new, &data as *const i32 as *const c_void);
        assert_eq!(vs_iter_len(iter), 2);
        assert_eq!(vs_iter_index(iter), 0);
        assert_eq!(vs_len(new), 2);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(vs_iter_index(iter), 1);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert!(vs_iter_next(iter).is_null());
        assert_eq!(vs_iter_index(iter), 2);
        vs_iter_destroy(iter);

        let data2: i32 = 10;
        let iter = vs_iter(new);
        assert_eq!(vs_iter_len(iter), 2);
        vs_append(new, &data2 as *const i32 as *const c_void);
        vs_append(new, &data as *const i32 as *const c_void);
        vs_append(new, &data2 as *const i32 as *const c_void);
        assert_eq!(vs_iter_len(iter), 5);
        assert_eq!(vs_len(new), 5);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data2);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data2);
        assert!(vs_iter_next(iter).is_null());

        let iter = vs_iter(new);
        vs_clear(new);
        assert_eq!(vs_len(new), 0);

        assert_eq!(vs_iter_len(iter), 5);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data2);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data);
        assert_eq!(*(vs_iter_next(iter) as *const i32), data2);
        assert!(vs_iter_next(iter).is_null());

        let iter = vs_iter(new);
        assert!(vs_iter_next(iter).is_null());
        assert_eq!(vs_iter_len(iter), 0);

        vs_destroy(null_mut());
        vs_destroy(new);
        vs_iter_destroy(null_mut());
        vs_iter_destroy(iter);
    }
}
