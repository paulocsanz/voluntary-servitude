//! Atomic `Box<T>`

use std::fmt::{Debug, Formatter, Pointer, Result as FmtResult};
use std::ptr::{null_mut, NonNull};
use std::{marker::PhantomData, mem::drop, sync::atomic::AtomicPtr, sync::atomic::Ordering};
use IntoPtr;

/// Atomic abstractions of a `Box<T>`
#[derive(Debug)]
pub struct Atomic<T>(AtomicPtr<T>, PhantomData<Box<T>>);

impl<T> Atomic<T> {
    /// Inner swap, helper to swap `Atomic` values
    #[inline]
    unsafe fn inner_swap(&self, new: *mut T, order: Ordering) -> Box<T> {
        Box::from_raw(self.0.swap(new, order))
    }

    /// Creates new `Atomic`
    ///
    /// ```rust
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let filled = Atomic::from(10);
    /// assert_eq!(*filled.into_inner(), 10);
    /// ```
    #[inline]
    pub fn new(data: Box<T>) -> Self {
        Self::from(data)
    }

    /// Stores value into `Atomic` and drops old one
    ///
    /// [`Atomic`]: ./struct.Atomic.html
    ///
    /// ```rust
    /// # use std::sync::atomic::Ordering;
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let filled = Atomic::from(10);
    /// filled.store(5.into(), Ordering::SeqCst);
    /// assert_eq!(*filled.into_inner(), 5);
    /// ```
    #[inline]
    pub fn store(&self, new: Box<T>, order: Ordering) {
        drop(self.swap(new, order))
    }

    /// Stores value into `Atomic` returning old value
    ///
    /// ```rust
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let option = Atomic::from(10);
    /// assert_eq!(*option.into_inner(), 10);
    /// ```
    #[inline]
    pub fn swap(&self, new: Box<T>, order: Ordering) -> Box<T> {
        unsafe { self.inner_swap(new.into_ptr(), order) }
    }

    /// Converts itself into a `Box<T>`
    ///
    /// ```rust
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let ten = Atomic::from(10);
    /// assert_eq!(*ten.into_inner(), 10);
    /// ```
    #[inline]
    pub fn into_inner(self) -> Box<T> {
        unsafe { self.inner_swap(null_mut(), Ordering::SeqCst) }
    }

    /// Creates new `Atomic` if pointer is not null (like `NonNull`)
    ///
    /// # Safety
    ///
    /// Unsafe because it uses a raw pointer, so it can't be sure of its origin (and ownership)
    ///
    /// You must own the pointer to call this
    ///
    /// ```rust
    /// # use std::ptr::null_mut;
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = unsafe { Atomic::<()>::from_raw(null_mut()) };
    /// assert!(empty.is_none());
    ///
    /// let filled = unsafe { Atomic::from_raw(Box::into_raw(Box::new(10))) };
    /// assert_eq!(filled.map(|a| *a.into_inner()), Some(10));
    /// ```
    #[inline]
    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        NonNull::new(ptr).map(|nn| Self::from_raw_unchecked(nn.as_ptr()))
    }

    /// Creates new `Atomic` based on raw pointer without checking for null pointer
    ///
    /// # Safety
    ///
    /// Unsafe because it trusts that the pointer is not null and because it can't be sure of the origin of `T` (and ownership)
    ///
    /// You must own the pointer to call this
    ///
    /// ```rust
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let filled = unsafe { Atomic::from_raw_unchecked(Box::into_raw(Box::new(10))) };
    /// assert_eq!(*filled.into_inner(), 10);
    ///
    /// // It's UB for `ptr` to be `null_mut()`
    /// // let empty = unsafe { Atomic::<()>::from_raw_unchecked(null_mut()) };
    /// ```
    #[inline]
    pub unsafe fn from_raw_unchecked(ptr: *mut T) -> Self {
        debug!("from_raw_unchecked({:p})", ptr);
        Atomic(AtomicPtr::new(ptr), PhantomData)
    }

    /// Atomically extracts the current stored pointer, this function should probably not be called
    ///
    /// # Safety
    ///
    /// It's almost never safe to deref this value, it could have been dropped from the moment you
    /// extracted it to the moment you deref/access it in any way, it will cause UB
    ///
    /// It exists to provide a way of implementing safe wrappers (like [`FillOnceAtomicOption`])
    ///
    /// [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html
    ///
    /// ```rust
    /// # use std::{sync::atomic::Ordering, ptr::null_mut};
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = unsafe { Atomic::<()>::from_raw_unchecked(null_mut()) };
    /// assert_eq!(empty.get_raw(Ordering::SeqCst), null_mut());
    ///
    /// let ptr = Box::into_raw(Box::new(10u8));
    /// let filled = unsafe { Atomic::from_raw(ptr) };
    /// assert_eq!(filled.map(|a| a.get_raw(Ordering::SeqCst)), Some(ptr));
    ///
    /// // You should probably never deref `ptr`
    /// // You should probably never use this method
    /// // UB will be everywhere, FillOnceAtomicOption is a safe an alternative
    /// ```
    #[inline]
    pub fn get_raw(&self, order: Ordering) -> *mut T {
        self.0.load(order)
    }

    /// Empties `Atomic`, this function should probably never be called
    ///
    /// You should probably use [`into_inner`]
    ///
    /// # Safety
    ///
    /// This is extremely unsafe, you don't want to call this unless you are implementing `Drop` for a chained `T`
    ///
    /// All reference will endup invalidated and any function call other than dropping will cause UB
    ///
    /// This is useful to obtain ownership of the inner value and implement a custom drop
    /// (like a linked list iteratively dropped - [`VS`])
    ///
    /// [`into_inner`]: #method.into_inner
    /// [`VS`]: ./type.VS.html
    #[inline]
    pub unsafe fn dangle(&mut self) -> Box<T> {
        info!("dangle()");
        self.inner_swap(null_mut(), Ordering::SeqCst)
    }
}

impl<T> From<T> for Atomic<T> {
    #[inline]
    fn from(into_ptr: T) -> Self {
        Self::from(Box::new(into_ptr))
    }
}

impl<T> From<Box<T>> for Atomic<T> {
    #[inline]
    fn from(into_ptr: Box<T>) -> Self {
        trace!("From<Box<T>>");
        Atomic(AtomicPtr::from(into_ptr.into_ptr()), PhantomData)
    }
}

impl<T> Pointer for Atomic<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Debug::fmt(&self.get_raw(Ordering::SeqCst), f)
    }
}

impl<T> Drop for Atomic<T> {
    #[inline]
    fn drop(&mut self) {
        info!("Drop");
        let _ = NonNull::new(self.0.swap(null_mut(), Ordering::SeqCst))
            .map(|nn| drop(unsafe { Box::from_raw(nn.as_ptr()) }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send() {
        fn assert_send<T>() {}
        assert_send::<Atomic<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Atomic<()>>();
    }
}
