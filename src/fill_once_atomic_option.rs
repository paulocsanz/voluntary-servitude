//! Atomic `Option<Box<T>>` that can give references (but only be filled once)
//!
//! Since `FillOnceAtomicOption` can only be filled once it's safe to provide access to `Option<&T>`
//!
//! This is ideal for a iterator or some consumer that doesn't actually consume the data

use std::fmt::{Debug, Formatter, Pointer, Result as FmtResult};
use std::{ptr::NonNull, sync::atomic::Ordering};
use {Atomic, AtomicOption, NotEmpty};

/// Atomic abstraction of a `Option<Box<T>>` that can provide access to a `Option<&T>`
///
/// This is ideal for a iterator or some consumer that doesn't actually consume the data
pub struct FillOnceAtomicOption<T>(AtomicOption<T>);

impl<T> FillOnceAtomicOption<T> {
    /// Creates new `FillOnceAtomicOption`
    ///
    /// ```rust
    /// # use std::sync::atomic::Ordering;
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = FillOnceAtomicOption::<()>::default();
    /// assert_eq!(empty.get_ref(Ordering::SeqCst), None);
    ///
    /// let filled = FillOnceAtomicOption::new(Some(10.into()));
    /// assert_eq!(filled.get_ref(Ordering::SeqCst), Some(&10));
    /// ```
    #[inline]
    pub fn new(data: Option<Box<T>>) -> Self {
        Self::from(data)
    }

    /// Stores new value if `FillOnceAtomicOption` was not initialized (contains a `None`)
    ///
    /// This operation is implemented as a single atomic `compare_and_swap`.
    ///
    /// ```rust
    /// # use std::sync::atomic::Ordering;
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let option = FillOnceAtomicOption::default();
    /// let old = option.try_store(5.into(), Ordering::SeqCst);
    /// assert!(old.is_ok());
    /// assert_eq!(option.get_ref(Ordering::SeqCst), Some(&5));
    ///
    /// let failed_to_store = option.try_store(10.into(), Ordering::SeqCst);
    /// assert!(failed_to_store.is_err());
    /// assert_eq!(option.get_ref(Ordering::SeqCst), Some(&5));
    /// ```
    #[inline]
    pub fn try_store(&self, data: Box<T>, order: Ordering) -> Result<(), NotEmpty> {
        self.0.try_store(data, order)
    }
}

impl<T: Copy> FillOnceAtomicOption<T> {
    /// Returns a copy of the wrapped `T`
    ///
    /// ```rust
    /// # use std::sync::atomic::Ordering;
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = FillOnceAtomicOption::<()>::new(None);
    /// assert_eq!(empty.load(Ordering::SeqCst), None);
    ///
    /// let filled = FillOnceAtomicOption::from(10);
    /// assert_eq!(filled.load(Ordering::SeqCst), Some(10));
    /// ```
    #[inline]
    pub fn load(&self, order: Ordering) -> Option<T> {
        self.get_ref(order).cloned()
    }
}

impl<T> FillOnceAtomicOption<T> {
    /// Atomically extracts a reference to the element stored
    ///
    /// ```rust
    /// # use std::sync::atomic::Ordering;
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = FillOnceAtomicOption::<()>::new(None);
    /// assert_eq!(empty.get_ref(Ordering::SeqCst), None);
    ///
    /// let filled = FillOnceAtomicOption::from(10);
    /// assert_eq!(filled.get_ref(Ordering::SeqCst), Some(&10));
    /// ```
    #[inline]
    pub fn get_ref(&self, order: Ordering) -> Option<&T> {
        let raw = self.0.get_raw(order);
        debug!("FillOnceAtomicOption get_ref: {:p}", raw);
        NonNull::new(raw).map(|nn| unsafe { &*nn.as_ptr() })
    }

    /// Converts itself into a `Option<Box<T>>`
    ///
    /// ```rust
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let ten = FillOnceAtomicOption::from(10);
    /// assert_eq!(ten.into_inner().map(|a| *a), Some(10));
    /// ```
    #[inline]
    pub fn into_inner(self) -> Option<Box<T>> {
        self.0.into_inner()
    }

    /// Creates new `FillOnceAtomicOption` based on a raw pointer
    ///
    /// # Safety
    ///
    /// Unsafe because it uses a raw pointer, so it can't be sure of its origin (and ownership)
    ///
    /// You must own the pointer to call this
    ///
    /// ```rust
    /// # use std::{sync::atomic::Ordering, ptr::null_mut};
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = unsafe { FillOnceAtomicOption::<()>::from_raw(null_mut()) };
    /// assert_eq!(empty.get_ref(Ordering::SeqCst), None);
    ///
    /// let filled = unsafe { FillOnceAtomicOption::from_raw(Box::into_raw(Box::new(10))) };
    /// assert_eq!(filled.get_ref(Ordering::SeqCst), Some(&10));
    /// ```
    #[inline]
    pub unsafe fn from_raw(ptr: *mut T) -> Self {
        FillOnceAtomicOption(AtomicOption::from_raw(ptr))
    }

    /// Atomically extracts the stored pointer
    ///
    /// If pointer returned is not null it's safe to deref as long as you don't drop the `FillOnceAtomicOption` or call [`dangle`] in it
    ///
    /// # Safety
    ///
    /// To deref it you must ensure that it's not `null`, the `FillOnceAtomicOption` wasn't dropped and [`dangle`] was not called
    ///
    /// Returns `null` if `FillOnceAtomicOption` is empty (was not initialized or unsafely emptied with [`dangle`] and dropped)
    ///
    /// [`dangle`]: #method.dangle
    ///
    /// ```rust
    /// # use std::{sync::atomic::Ordering, ptr::null_mut, ops::Deref};
    /// # use voluntary_servitude::FillOnceAtomicOption;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let empty = FillOnceAtomicOption::<()>::new(None);
    /// assert_eq!(empty.get_raw(Ordering::SeqCst), null_mut());
    ///
    /// let filled = FillOnceAtomicOption::from(10);
    /// assert_eq!(unsafe { (&*filled.get_raw(Ordering::SeqCst)).deref() }, &10);
    /// ```
    #[inline]
    pub fn get_raw(&self, order: Ordering) -> *mut T {
        self.0.get_raw(order)
    }

    /// Empties `FillOnceAtomicOption`, this function should probably never be called
    ///
    /// You should probably use [`into_inner`]
    ///
    /// # Safety
    ///
    /// This is extremely unsafe, you don't want to call this unless you are implementing `Drop` for a chained `T`
    ///
    /// All reference will endup invalidated and any function call other than [`try_store`] (or dropping) will cause UB
    ///
    /// In a multi-thread environment it's very hard to ensure that this won't happen
    ///
    /// This is useful to obtain ownership of the inner value and implement a custom drop
    /// (like a linked list iteratively dropped - [`VS`])
    ///
    /// [`into_inner`]: #method.into_inner
    /// [`dangle`]: #method.dangle
    /// [`try_store`]: #method.try_store
    /// [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html
    /// [`AtomicOption`]: ./struct.AtomicOption.html
    /// [`VS`]: ./type.VS.html
    #[inline]
    pub unsafe fn dangle(&self) -> Option<Box<T>> {
        info!("dangle()");
        self.0.swap(None, Ordering::SeqCst)
    }
}

impl<T> Default for FillOnceAtomicOption<T> {
    #[inline]
    fn default() -> Self {
        Self::from(Option::<Box<T>>::default())
    }
}

impl<T> From<T> for FillOnceAtomicOption<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::from(Box::new(value))
    }
}

impl<T> From<Box<T>> for FillOnceAtomicOption<T> {
    #[inline]
    fn from(into_ptr: Box<T>) -> Self {
        Self::from(Some(into_ptr))
    }
}

impl<T> From<Option<T>> for FillOnceAtomicOption<T> {
    #[inline]
    fn from(into_ptr: Option<T>) -> Self {
        Self::from(into_ptr.map(Box::new))
    }
}

impl<T> From<Option<Box<T>>> for FillOnceAtomicOption<T> {
    #[inline]
    fn from(into_ptr: Option<Box<T>>) -> Self {
        trace!("From<Option<Box<T>>");
        FillOnceAtomicOption(AtomicOption::from(into_ptr))
    }
}

impl<T> From<AtomicOption<T>> for FillOnceAtomicOption<T> {
    #[inline]
    fn from(atomic: AtomicOption<T>) -> Self {
        trace!("From AtomicOption");
        Self::from(atomic.into_inner())
    }
}

impl<T> From<Atomic<T>> for FillOnceAtomicOption<T> {
    #[inline]
    fn from(atomic: Atomic<T>) -> Self {
        trace!("From Atomic");
        Self::from(atomic.into_inner())
    }
}

impl<T> Pointer for FillOnceAtomicOption<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        Debug::fmt(&self.get_raw(Ordering::SeqCst), f)
    }
}

impl<T: Debug> Debug for FillOnceAtomicOption<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.debug_tuple("FillOnceAtomicOption")
            .field(&self.get_ref(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_once_ref() {
        let atomic = FillOnceAtomicOption::from(Some(10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
        assert_eq!(atomic.get_ref(Ordering::SeqCst), Some(&10));
    }

    #[test]
    fn test_send() {
        fn assert_send<T>() {}
        assert_send::<FillOnceAtomicOption<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<FillOnceAtomicOption<()>>();
    }
}
