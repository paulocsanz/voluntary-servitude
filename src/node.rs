//! [`VoluntaryServitude`] node implementation
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html

use crate::prelude::*;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::Ordering;

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
        trace!("value() = {:p}", &self.value);
        &self.value
    }

    /// Creates new node with inner value
    #[inline]
    pub fn new(value: T) -> Self {
        trace!("new()");
        let next = FillOnceAtomicOption::default();
        Self { value, next }
    }

    /// Gets next pointer
    #[inline]
    pub fn next(&self) -> Option<&Self> {
        trace!("next()");
        self.next.get_ref(Ordering::SeqCst)
    }

    /// Inserts next as if there was None
    #[inline]
    pub fn try_store_next(&self, node: Box<Self>) -> Result<(), NotEmpty> {
        trace!("try_store_next({:p})", node);
        self.next.try_store(node, Ordering::SeqCst)
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    #[inline]
    fn drop(&mut self) {
        debug!("Drop nodes");
        let mut node = self.next.take(Ordering::SeqCst);
        while let Some(mut n) = node {
            node = n.next.take(Ordering::SeqCst);
        }
    }
}

impl<T: Debug> Debug for Node<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Node")
            .field("value", &self.value)
            .field("next", &self.next.get_ref(Ordering::SeqCst))
            .finish()
    }
}
