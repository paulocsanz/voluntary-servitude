pub use node::Node;
use std::{
    cell::UnsafeCell,
    fmt::{self, Debug, Formatter},
    sync::{Arc, Weak},
};
pub use vsread::VSRead;

pub type ArcNode<T> = Arc<VoluntaryServitude<Node<T>>>;
pub type WeakNode<T> = Weak<VoluntaryServitude<Node<T>>>;
pub type WrappedNode<T> = VoluntaryServitude<Option<ArcNode<T>>>;
pub type WrappedWeak<T> = VoluntaryServitude<Option<WeakNode<T>>>;

pub struct VoluntaryServitude<T> {
    pub cell: UnsafeCell<T>,
}
unsafe impl<T> Sync for VoluntaryServitude<T> {}

impl<T: Debug> VoluntaryServitude<T> {
    pub fn new(value: T) -> VoluntaryServitude<T> {
        trace!("New VoluntaryServitude based on {:?}", value);
        VoluntaryServitude {
            cell: UnsafeCell::new(value),
        }
    }
}

impl<T: Debug> Debug for VoluntaryServitude<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug VoluntaryServitude<T>");
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ {:?} }} }}",
            unsafe { &*self.cell.get() }
        )
    }
}
