//! `VoluntaryServitude` node implementation

use std::sync::atomic::{AtomicPtr, Ordering};
use std::{fmt, fmt::Debug, fmt::Formatter, ptr::null_mut, ptr::NonNull};

/// One `VoluntaryServitude` element
pub struct Node<T> {
    /// Inner value
    value: T,
    /// Next node in chain
    next: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    /// Returns reference to inner value
    #[inline]
    pub fn value(&self) -> &T {
        trace!("Node value");
        &self.value
    }

    /// Creates new node with inner value
    #[inline]
    pub fn new(value: T) -> Self {
        trace!("New node");
        Self {
            value,
            next: AtomicPtr::new(null_mut()),
        }
    }

    /// Returns consumable reference to Self
    #[inline]
    pub fn next(&self) -> Option<NonNull<Self>> {
        trace!("Next node");
        NonNull::new(self.next.load(Ordering::SeqCst))
    }

    /// Replaces `self.next` with pointer, returns old pointer
    #[inline]
    pub fn swap_next(&self, ptr: *mut Self) -> Option<NonNull<Self>> {
        trace!("Swap next node");
        NonNull::new(self.next.swap(ptr, Ordering::SeqCst))
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        info!("Drop chained nodes");
        let mut node = self.next.swap(null_mut(), Ordering::SeqCst);
        while !node.is_null() {
            unsafe {
                let next = (*node).next.swap(null_mut(), Ordering::SeqCst);
                drop(Box::from_raw(node));
                node = next;
            }
        }
        trace!("Dropped all chained nodes");
    }
}

/// Default Debug is recursive and causes a stackoverflow easily
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug Node");
        write!(f, "Node {{ value: {:?}, next: ", self.value)?;
        let next = self.next.load(Ordering::SeqCst);
        if next.is_null() {
            write!(f, "None }}")?;
        } else {
            unsafe {
                let next_next = (*next).next.load(Ordering::SeqCst);
                let has_next = if next_next.is_null() { "Some" } else { "None" };
                write!(
                    f,
                    "Some(Node {{ value: {:?}, next: {} }}) }}",
                    (*next).value,
                    has_next
                )?;
            }
        }
        Ok(())
    }
}
