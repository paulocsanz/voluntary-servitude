//! Atomic `Box<T>`

use atomic_option::AtomicOption;
use std::fmt::{Debug, Formatter, Pointer, Result as FmtResult};
use std::{marker::PhantomData, ptr::NonNull, sync::atomic::Ordering};

/// Atomic abstractions of a `Box<T>`
#[derive(Debug)]
pub struct Atomic<T>(AtomicOption<T>, PhantomData<Box<T>>);

impl<T> Atomic<T> {
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
        self.0.store(Some(new), order)
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
        unwrap_option!(self.0.swap(Some(new), order); "Atomic<T> was null (swap)")
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
        unwrap_option!(self.0.swap(None, Ordering::SeqCst); "Atomic<T> was null (into_inner)")
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
    /// // Will cause UB if you use the value in empty (other than storing to it)
    /// // let empty = unsafe { Atomic::<()>::from_raw_unchecked(null_mut()) };
    /// ```
    #[inline]
    pub unsafe fn from_raw_unchecked(ptr: *mut T) -> Self {
        debug!("from_raw_unchecked({:p})", ptr);
        Atomic(AtomicOption::from_raw(ptr), PhantomData)
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
        self.0.get_raw(order)
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
        Atomic(AtomicOption::from(into_ptr), PhantomData)
    }
}

impl<T> Pointer for Atomic<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Debug::fmt(&self.get_raw(Ordering::SeqCst), f)
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
