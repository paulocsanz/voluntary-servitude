//! Atomic `Box<T>`
//!
//! It can't provide a reference to the current value since it may be dropped at any time
//!
//! You must swap the element to access it
//!
//! [`FillOnceAtomicOption`] provides a API that enables access to the reference, but only enables `try_store` to write to it
//!
//! [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html

use std::fmt::{self, Debug, Formatter, Pointer};
use std::ptr::{null_mut, NonNull};
use std::{marker::PhantomData, mem::drop, sync::atomic::AtomicPtr, sync::atomic::Ordering};
use IntoPtr;

/// Atomic `Box<T>`
///
/// It can't provide a reference to the current value since it may be dropped at any time
///
/// You must swap the element to access it
///
/// [`FillOnceAtomicOption`] provides a API that enables access to the reference, but only enables `try_store` to write to it
///
/// [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html
pub struct Atomic<T>(AtomicPtr<T>, PhantomData<Box<T>>);

impl<T: Debug> Debug for Atomic<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("Atomic").field(&self.0).finish()
    }
}

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
    /// let filled = Atomic::new(10);
    /// assert_eq!(*filled.into_inner(), 10);
    /// ```
    #[inline]
    pub fn new<V>(data: V) -> Self
    where
        V: Into<Box<T>>
    {
        Self::from(data.into())
    }

    /// Stores value into `Atomic` and drops old one
    ///
    /// [`Atomic`]: ./struct.Atomic.html
    ///
    /// ```rust
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let filled = Atomic::from(10);
    /// filled.store(5, Ordering::SeqCst);
    /// assert_eq!(*filled.into_inner(), 5);
    /// ```
    #[inline]
    pub fn store<V>(&self, new: V, order: Ordering)
    where
        V: Into<Box<T>>
    {
        drop(self.swap(new, order))
    }

    /// Stores value into `Atomic` returning old value
    ///
    /// ```rust
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let option = Atomic::from(10);
    /// assert_eq!(*option.swap(4, Ordering::SeqCst), 10);
    /// assert_eq!(*option.into_inner(), 4);
    /// ```
    #[inline]
    pub fn swap<V>(&self, new: V, order: Ordering) -> Box<T>
    where
        V: Into<Box<T>>
    {
        unsafe { self.inner_swap(new.into().into_ptr(), order) }
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
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::ptr::null_mut;
    /// let empty: Option<Atomic<()>> = unsafe { Atomic::from_raw(null_mut()) };
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
        debug_assert!(!ptr.is_null());
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
    /// # use voluntary_servitude::Atomic;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::{sync::atomic::Ordering, ptr::null_mut};
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.get_raw(Ordering::SeqCst), f)
    }
}

impl<T> Drop for Atomic<T> {
    #[inline]
    fn drop(&mut self) {
        info!("Drop");
        NonNull::new(self.0.swap(null_mut(), Ordering::SeqCst))
            .map_or((), |nn| drop(unsafe { Box::from_raw(nn.as_ptr()) }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Atomic<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<Atomic<()>>();
    }
}
