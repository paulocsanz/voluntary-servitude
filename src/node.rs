//! [`VoluntaryServitude`] node implementation
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::{cell::UnsafeCell, ptr::null_mut, ptr::NonNull, mem::drop, ptr::swap_nonoverlapping};

/// One [`VoluntaryServitude`] element
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub struct Node<T> {
    /// Inner value
    value: T,
    /// Next node in chain
    next: UnsafeCell<*mut Node<T>>,
}

impl<T> Node<T> {
    /// Returns reference to inner value
    #[inline]
    pub fn value(&self) -> &T {
        trace!("value() = {:p}", &self.value);
        &self.value
    }

    /// Creates new node with inner value
    #[inline]
    pub fn new(value: T) -> Self {
        trace!("new()");
        let next = UnsafeCell::new(null_mut());
        Self { value, next }
    }

    /// Gets next pointer (caller must be careful with data races)
    #[inline]
    pub unsafe fn next(&self) -> Option<NonNull<Self>> {
        trace!("next()");
        NonNull::new(*self.next.get())
    }

    /// Inserts next as if there was None (caller must be careful with data races)
    #[inline]
    pub unsafe fn set_next(&self, node: *mut Self) {
        trace!("set_next({:p})", node);
        *self.next.get() = node;
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        info!("Drop chained nodes");
        let mut node = null_mut();
        unsafe { swap_nonoverlapping(&mut node, self.next.get(), 1) };
        while let Some(nn) = NonNull::new(node) {
            let mut next = null_mut();
            unsafe { swap_nonoverlapping(&mut next, nn.as_ref().next.get(), 1) };
            unsafe { drop(Box::from_raw(nn.as_ptr())) };
            node = next;
        }
        debug!("Dropped all chained nodes");
    }
}

impl<T: Debug> Debug for Node<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.debug_struct("Node")
            .field("value", &self.value)
            .field("next", unsafe { &(*self.next.get()) })
            .finish()
    }
}
