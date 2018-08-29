//! Contains all types used by crate

pub use node::Node;
use std::{
    cell::UnsafeCell,
    fmt::{self, Debug, Formatter},
    sync::{Arc, Weak},
};
pub use vsread::VSRead;

/// Abstracts mutable thread-safe node
pub type ArcNode<T> = Arc<VoluntaryServitude<Node<T>>>;
/// Weak reference to ArcNode
pub type WeakNode<T> = Weak<VoluntaryServitude<Node<T>>>;

/// Abstracts mutable thread-safe next node
pub type WrappedNode<T> = VoluntaryServitude<Option<ArcNode<T>>>;
/// Weak reference to WrappedNode
pub type WrappedWeak<T> = VoluntaryServitude<Option<WeakNode<T>>>;

/// Wraps UnsafeCell to make it Sync (not actually thread-safe, must be used with care)
pub struct VoluntaryServitude<T> {
    pub cell: UnsafeCell<T>,
}
unsafe impl<T> Sync for VoluntaryServitude<T> {}

impl<T> VoluntaryServitude<T> {
    /// Creates a mutable multi-thread reference to raw value
    pub fn new(value: T) -> VoluntaryServitude<T> {
        trace!("New VoluntaryServitude");
        VoluntaryServitude {
            cell: UnsafeCell::new(value),
        }
    }

    /// Extracts Node from unsafe abstraction
    #[inline(always)]
    pub unsafe fn cell<'a>(&self) -> &'a mut T {
        &mut *self.cell.get()
    }
}

/// Recursively debugs UnsafeCell value
impl<T: Debug> Debug for VoluntaryServitude<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ {:?} }} }}",
            unsafe { self.cell() }
        )
    }
}
