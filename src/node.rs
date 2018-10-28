//! [`VoluntaryServitude`] node implementation
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::atomic::Ordering;
use {FillOnceAtomicOption, NotEmpty};

/// One [`VoluntaryServitude`] element
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub struct Node<T> {
    /// Inner value
    value: T,
    /// Next node in chain
    next: FillOnceAtomicOption<Node<T>>,
}

impl<T> Node<T> {
    /// Returns reference to inner value
    #[inline]
    pub fn value(&self) -> &T {
        trace!("value() = {:p}", &self.value as *const T);
        &self.value
    }

    /// Creates new node with inner value
    #[inline]
    pub fn new(value: T) -> Self {
        trace!("new()");
        let next = FillOnceAtomicOption::default();
        Self { value, next }
    }

    /// Atomically extracts ref to next Node
    #[inline]
    pub fn next(&self) -> Option<&Self> {
        trace!("next()");
        self.next.get_ref(Ordering::SeqCst)
    }

    /// If [`FillOnceAtomicOption`] was empty it will fill it and return None, otherwise return the [`NotEmpty`] error
    ///
    /// [`FillOnceAtomicOption`]: ./struct.FillOnceAtomicOption.html
    /// [`NotEmpty`]: ./struct.NotEmpty.html
    #[inline]
    pub fn try_set_next(&self, node: Box<Self>) -> Result<(), NotEmpty> {
        trace!("try_set_next({:p})", node);
        self.next.try_store(node, Ordering::SeqCst)
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        info!("Drop chained nodes");
        let mut node = unsafe { self.next.dangle() };
        while let Some(n) = node.take() {
            let next = unsafe { n.next.dangle() };
            drop(n);
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
            .field("next", &self.next.get_raw(Ordering::SeqCst))
            .finish()
    }
}
