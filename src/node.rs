//! VSRead node implementation

use std::{
    fmt::{self, Debug, Formatter},
    sync::Arc,
};
use types::*;

/// One VSRead element
pub struct Node<T> {
    pub value: T,
    pub next: WrappedNode<T>,
}

impl<T> Node<T> {
    /// Creates node from raw value
    pub fn arc_node(value: T) -> ArcNode<T> {
        trace!("Create ArcNode");
        Arc::new(VoluntaryServitude::new(Node {
            value,
            next: VoluntaryServitude::new(None),
        }))
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        trace!("Drop Node");
        let mut next = unsafe { (*self.next.cell.get()).take() };
        while let Some(node) = next.take() {
            if Arc::strong_count(&node) > 1 {
                continue;
            }
            next = unsafe { (*(*node.cell.get()).next.cell.get()).take() };
            debug!("Last reference to next node, dropping it too iteratively");
        }
        trace!("Leaving Drop");
    }
}

/// Default Debug is recursive and causes a stackoverflow easily
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let next = if let Some(ref next) = unsafe { &*self.next.cell.get() } {
            let this = unsafe { &*next.cell.get() };
            let next = unsafe { (*this.next.cell.get()).is_some() };
            let next = if next { "Some" } else { "None" };
            format!("Some(Node {{ value: {:?}, next: {} }})", this.value, next)
        } else {
            "None".to_owned()
        };
        write!(f, "Node {{ value: {:?}, next: {} }}", self.value, &next)
    }
}
