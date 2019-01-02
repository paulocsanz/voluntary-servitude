//! Atomic `Option<Arc<T>>` that can only be filled once
//!
//! Since `FillOnceAtomicArc` can only be filled once it's safe to provide access to the inner `Option<Arc<T>>` and `Option<&T>`

use crate::prelude::*;
use std::fmt::{self, Debug, Formatter, Pointer};
use std::{sync::atomic::Ordering, sync::Arc};

/// Atomic abstraction of a `Option<Arc<T>>` that can provide access to a cloned `Option<Arc<T>>` and a `Option<&T>`
pub struct FillOnceAtomicArc<T>(FillOnceAtomicOption<Arc<T>>);

impl<T> FillOnceAtomicArc<T> {
    /// Creates new `FillOnceAtomicArc`
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// use std::sync::{Arc, atomic::Ordering};
    /// let empty: FillOnceAtomicArc<()> = FillOnceAtomicArc::new(None);
    /// assert_eq!(empty.get_ref(Ordering::SeqCst), None);
    ///
    /// let filled = FillOnceAtomicArc::new(Arc::new(10));
    /// assert_eq!(filled.get_ref(Ordering::SeqCst), Some(&10));
    /// ```
    #[inline]
    pub fn new<V>(data: V) -> Self
    where
        V: Into<Option<Arc<T>>>,
    {
        trace!("new()");
        Self::from(data.into())
    }

    /// Stores new `Arc<T>` if `FillOnceAtomicArc` currently contains a `None`
    ///
    /// This operation is implemented as a single atomic `compare_and_swap`.
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// use std::sync::atomic::Ordering;
    /// let option = FillOnceAtomicArc::default();
    /// let old = option.try_store(5, Ordering::SeqCst);
    /// assert!(old.is_ok());
    /// assert_eq!(option.get_ref(Ordering::SeqCst), Some(&5));
    ///
    /// let failed_to_store = option.try_store(10, Ordering::SeqCst);
    /// assert!(failed_to_store.is_err());
    /// assert_eq!(option.get_ref(Ordering::SeqCst), Some(&5));
    /// ```
    #[inline]
    pub fn try_store<V>(&self, data: V, order: Ordering) -> Result<(), NotEmpty>
    where
        V: Into<Arc<T>>,
    {
        self.0.try_store(data.into(), order)
    }

    /// Atomically retrieves a cloned `Option<Arc<T>>`
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// use std::sync::atomic::Ordering;
    /// let empty: FillOnceAtomicArc<()> = FillOnceAtomicArc::new(None);
    /// assert_eq!(empty.load(Ordering::SeqCst), None);
    ///
    /// let filled = FillOnceAtomicArc::from(10);
    /// assert_eq!(filled.load(Ordering::SeqCst).map(|a| *a), Some(10));
    /// ```
    #[inline]
    pub fn load(&self, order: Ordering) -> Option<Arc<T>> {
        self.0.get_ref(order).cloned()
    }

    /// Atomically extracts a reference to the element stored
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// use std::sync::atomic::Ordering;
    /// let empty: FillOnceAtomicArc<()> = FillOnceAtomicArc::new(None);
    /// assert_eq!(empty.get_ref(Ordering::SeqCst), None);
    ///
    /// let filled = FillOnceAtomicArc::from(10);
    /// assert_eq!(filled.get_ref(Ordering::SeqCst), Some(&10));
    /// ```
    #[inline]
    pub fn get_ref(&self, order: Ordering) -> Option<&T> {
        self.0.get_ref(order).map(|arc| &**arc)
    }

    /// Converts itself into a `Option<Arc<T>>`
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// let ten = FillOnceAtomicArc::from(10);
    /// assert_eq!(ten.into_inner().map(|a| *a), Some(10));
    /// ```
    #[inline]
    pub fn into_inner(self) -> Option<Arc<T>> {
        self.0.into_inner().map(|a| Arc::clone(&*a))
    }

    /// Creates new `FillOnceAtomicArc` based on a raw pointer
    ///
    /// # Safety
    ///
    /// Unsafe because it uses a raw pointer, so it can't be sure of its origin (and ownership)
    ///
    /// You must own the pointer to call this
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// use std::{sync::Arc, sync::atomic::Ordering, ptr::null_mut};
    /// let empty = unsafe { FillOnceAtomicArc::<()>::from_raw(null_mut()) };
    /// assert_eq!(empty.get_ref(Ordering::SeqCst), None);
    ///
    /// let ptr = Box::into_raw(Box::new(Arc::new(10)));
    /// let filled = unsafe { FillOnceAtomicArc::from_raw(ptr) };
    /// assert_eq!(filled.get_ref(Ordering::SeqCst), Some(&10));
    /// ```
    #[inline]
    pub unsafe fn from_raw(ptr: *mut Arc<T>) -> Self {
        FillOnceAtomicArc(FillOnceAtomicOption::from_raw(ptr))
    }

    /// Atomically extracts the stored pointer
    ///
    /// If pointer returned is not null it's safe to deref as long as you don't drop the `FillOnceAtomicArc`
    ///
    /// # Safety
    ///
    /// To deref it you must ensure that it's not `null`, the `FillOnceAtomicArc` wasn't dropped
    ///
    /// Returns `null` if `FillOnceAtomicArc` is empty (was not initialized or dropped)
    ///
    ///
    /// ```rust
    /// # use voluntary_servitude::atomics::FillOnceAtomicArc;
    /// # env_logger::init();
    /// use std::{sync::atomic::Ordering, ptr::null_mut, ops::Deref};
    /// let empty: FillOnceAtomicArc<()> = FillOnceAtomicArc::new(None);
    /// assert_eq!(empty.get_raw(Ordering::SeqCst), null_mut());
    ///
    /// let filled = FillOnceAtomicArc::from(10);
    /// assert_eq!(unsafe { (&*filled.get_raw(Ordering::SeqCst)).deref().deref() }, &10);
    /// ```
    #[inline]
    pub fn get_raw(&self, order: Ordering) -> *mut Arc<T> {
        self.0.get_raw(order)
    }
}

impl<T> Default for FillOnceAtomicArc<T> {
    #[inline]
    fn default() -> Self {
        Self::from(None)
    }
}

impl<T> From<T> for FillOnceAtomicArc<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self::from(Arc::new(value))
    }
}

impl<T> From<Arc<T>> for FillOnceAtomicArc<T> {
    #[inline]
    fn from(into_ptr: Arc<T>) -> Self {
        Self::from(Some(into_ptr))
    }
}

impl<T> From<Option<Arc<T>>> for FillOnceAtomicArc<T> {
    #[inline]
    fn from(arc: Option<Arc<T>>) -> Self {
        trace!("From Option<Arc<T>>");
        FillOnceAtomicArc(FillOnceAtomicOption::from(arc.map(Box::new)))
    }
}

impl<T> Pointer for FillOnceAtomicArc<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        Debug::fmt(&self.get_raw(Ordering::SeqCst), f)
    }
}

impl<T: Debug> Debug for FillOnceAtomicArc<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("FillOnceAtomicArc")
            .field(&self.load(Ordering::SeqCst))
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<FillOnceAtomicArc<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<FillOnceAtomicArc<()>>();
    }
}
