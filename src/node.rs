//! [`VoluntaryServitude`] node implementation
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html

use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::Ordering;
use FillOnceAtomicOption;

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
    pub fn set_next(&self, node: Box<Self>) {
        trace!("set_next({:p})", node);
        #[allow(unused)]
        let ret = self.next.try_store(node, Ordering::SeqCst);
        debug_assert!(ret.is_ok());
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    #[inline]
    fn drop(&mut self) {
        debug!("Drop nodes");
        let mut node = unsafe { self.next.dangle() };
        while let Some(mut n) = node {
            node = unsafe { n.next.dangle() };
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
