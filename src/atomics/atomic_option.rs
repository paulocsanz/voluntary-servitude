//! Atomic `Option<Box<T>>`
//!
//! It can't provide a reference to the current value since it may be dropped at any time
//!
//! You must swap the element to access it
//!
//! [`FillOnceAtomicOption`] provides a API that enables access to the reference, but only enables `try_store` to write to it
//!
//! [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html

use std::fmt::{self, Debug, Formatter, Pointer};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::{marker::PhantomData, mem::drop, ptr::null_mut, ptr::NonNull};
use {atomics::Atomic, atomics::FillOnceAtomicOption, IntoPtr, NotEmpty};

/// Atomic `Option<Box<T>>`
///
/// It can't provide a reference to the current value since it may be dropped at any time
///
/// You must swap the element to access it
///
/// [`FillOnceAtomicOption`] provides a API that enables access to the reference, but only enables `try_store` to write to it
///
/// [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html#method.try_store
pub struct AtomicOption<T>(AtomicPtr<T>, PhantomData<Option<Box<T>>>);

impl<T: Debug> Debug for AtomicOption<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("AtomicOption").field(&self.0).finish()
    }
}

impl<T> AtomicOption<T> {
    /// Creates new `AtomicOption`
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty: AtomicOption<()> = AtomicOption::new(None);
    /// assert!(empty.into_inner().is_none());
    ///
    /// let filled = AtomicOption::new(Box::new(10));
    /// assert_eq!(filled.into_inner().map(|a| *a), Some(10));
    /// ```
    #[inline]
    pub fn new<V>(value: V) -> Self
    where
        V: Into<Option<Box<T>>>,
    {
        Self::from(value.into())
    }

    /// Stores new value if `AtomicOption` currently contains a `None`
    ///
    /// This operation is implemented as a single atomic `compare_and_swap`.
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let option = AtomicOption::default();
    /// let old = option.try_store(5, Ordering::SeqCst);
    /// assert!(old.is_ok());
    ///
    /// let failed_to_store = option.try_store(10, Ordering::SeqCst);
    /// assert!(failed_to_store.is_err());
    /// assert_eq!(option.into_inner().map(|a| *a), Some(5));
    /// ```
    #[inline]
    pub fn try_store<V>(&self, new: V, order: Ordering) -> Result<(), NotEmpty>
    where
        V: Into<Box<T>>,
    {
        let ptr = new.into().into_ptr();
        let old = NonNull::new(self.0.compare_and_swap(null_mut(), ptr, order));
        trace!("try_store({:p}) = {:?})", ptr, old);
        old.map_or(Ok(()), |_| Err(NotEmpty))
    }

    /// Stores value into `AtomicOption` and drops old one
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let option: AtomicOption<u8> = AtomicOption::new(None);
    /// option.store(Box::new(3), Ordering::SeqCst);
    /// assert_eq!(option.into_inner().map(|a| *a), Some(3));
    /// ```
    #[inline]
    pub fn store<V>(&self, new: V, order: Ordering)
    where
        V: Into<Option<Box<T>>>,
    {
        drop(self.swap(new, order));
    }

    /// Stores value into `AtomicOption` returning old value
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let option = AtomicOption::default();
    /// assert_eq!(option.swap(Box::new(5), Ordering::SeqCst), None);
    /// assert_eq!(option.swap(None, Ordering::SeqCst), Some(Box::new(5)));
    /// # assert_eq!(option.swap(Box::new(3), Ordering::SeqCst), None);
    /// ```
    #[inline]
    pub fn swap<V>(&self, new: V, order: Ordering) -> Option<Box<T>>
    where
        V: Into<Option<Box<T>>>,
    {
        let ptr = new.into().into_ptr();
        let old = NonNull::new(self.0.swap(ptr, order));
        trace!("swap({:p}) = {:?}", ptr, old);
        old.map(|nn| unsafe { Box::from_raw(nn.as_ptr()) })
    }

    /// Replaces `AtomicOption` value with `None` returning old value
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let option = AtomicOption::from(5);
    /// assert_eq!(option.take(Ordering::SeqCst), Some(Box::new(5)));
    /// assert_eq!(option.take(Ordering::SeqCst), None);
    /// # assert_eq!(option.take(Ordering::SeqCst), None);
    /// ```
    #[inline]
    pub fn take(&self, order: Ordering) -> Option<Box<T>> {
        self.swap(None, order)
    }

    /// Gives access to inner `AtomicPtr` (`AtomicOption` is an abstraction of it).
    ///
    /// # Safety
    ///
    /// This is heavily unsafe.
    ///
    /// It's unsafe because you are responsible for making sure `T` is not dropped nor replaced with a invalid pointer (or that will be invalidated while still stored).
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::sync::atomic::Ordering;
    /// let ten = AtomicOption::from(10);
    /// assert_eq!(unsafe { &*ten.atomic_ptr().load(Ordering::SeqCst) }, &10);
    /// ```
    #[inline]
    pub unsafe fn atomic_ptr(&self) -> &AtomicPtr<T> {
        debug!("atomic_ptr()");
        &self.0
    }

    /// Converts itself into a `Option<Box<T>>`
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let ten = AtomicOption::from(10);
    /// assert_eq!(ten.into_inner().map(|a| *a), Some(10));
    /// ```
    #[inline]
    pub fn into_inner(self) -> Option<Box<T>> {
        self.swap(None, Ordering::SeqCst)
    }

    /// Creates new `AtomicOption` based on raw pointer
    ///
    /// # Safety
    ///
    /// Unsafe because it uses a raw pointer, so it can't be sure of its origin (and ownership)
    ///
    /// You must own the pointer to call this
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::ptr::null_mut;
    /// let empty = unsafe { AtomicOption::<()>::from_raw(null_mut()) };
    /// assert!(empty.into_inner().is_none());
    ///
    /// let filled = unsafe { AtomicOption::from_raw(Box::into_raw(10.into())) };
    /// assert_eq!(filled.into_inner().map(|a| *a), Some(10));
    /// ```
    #[inline]
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        debug!("from_raw({:p})", ptr);
        AtomicOption(AtomicPtr::new(ptr), PhantomData)
    }

    /// Atomically extracts current pointer stored, this function should probably not be called
    ///
    /// # Safety
    ///
    /// It's almost never safe to deref this value, it could have been dropped from the moment you extracted it to the moment you deref/access it in any way, it WILL cause UB
    ///
    /// It exists to provide a way of implementing safe wrappers (like [`FillOnceAtomicOption`])
    ///
    /// [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::AtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// use std::{sync::atomic::Ordering, ptr::null_mut};
    /// let empty: AtomicOption<()> = AtomicOption::new(None);
    /// assert_eq!(empty.get_raw(Ordering::SeqCst), null_mut());
    ///
    /// let ptr = Box::into_raw(10u8.into());
    /// let filled = unsafe { AtomicOption::from_raw(ptr) };
    /// assert_eq!(filled.get_raw(Ordering::SeqCst), ptr);
    ///
    /// // You should probably never deref `ptr`
    /// // You should probably never use this method
    /// // UB will be everywhere, FillOnceAtomicOption is a safe an alternative
    /// ```
    #[inline]
    pub fn get_raw(&self, order: Ordering) -> *mut T {
        trace!("get_raw({:?})", order);
        self.0.load(order)
    }
}

impl<T> Default for AtomicOption<T> {
    #[inline]
    fn default() -> Self {
        Self::new(None)
    }
}

impl<T> From<T> for AtomicOption<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::from(Box::new(value))
    }
}

impl<T> From<Box<T>> for AtomicOption<T> {
    #[inline]
    fn from(boxed: Box<T>) -> Self {
        Self::from(Some(boxed))
    }
}

impl<T> From<Option<T>> for AtomicOption<T> {
    #[inline]
    fn from(into_ptr: Option<T>) -> Self {
        Self::from(into_ptr.map(Box::new))
    }
}

impl<T> From<Option<Box<T>>> for AtomicOption<T> {
    #[inline]
    fn from(into_ptr: Option<Box<T>>) -> Self {
        let ptr = into_ptr.into_ptr();
        trace!("From Option<Box<T>>: {:p}", ptr);
        AtomicOption(AtomicPtr::new(ptr), PhantomData)
    }
}

impl<T> From<FillOnceAtomicOption<T>> for AtomicOption<T> {
    #[inline]
    fn from(atomic: FillOnceAtomicOption<T>) -> Self {
        trace!("From FillOnceAtomicOption");
        Self::from(atomic.into_inner())
    }
}

impl<T> From<Atomic<T>> for AtomicOption<T> {
    #[inline]
    fn from(atomic: Atomic<T>) -> Self {
        trace!("From Atomic");
        Self::from(atomic.into_inner())
    }
}

impl<T> Pointer for AtomicOption<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.get_raw(Ordering::SeqCst), f)
    }
}

impl<T> Drop for AtomicOption<T> {
    #[inline]
    fn drop(&mut self) {
        info!("Drop");
        drop(self.take(Ordering::SeqCst))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<AtomicOption<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<AtomicOption<()>>();
    }
}
