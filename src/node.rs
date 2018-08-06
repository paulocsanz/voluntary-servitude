use std::{
    fmt::{self, Debug, Formatter},
    sync::Arc,
};
use types::*;

pub struct Node<T: Debug> {
    pub value: T,
    pub next: WrappedNode<T>,
}

impl<T: Debug> Node<T> {
    pub fn arc_node(value: T) -> ArcNode<T> {
        trace!("New ArcNode Based on {:?}", value);
        Arc::new(VoluntaryServitude::new(Node {
            value,
            next: VoluntaryServitude::new(None),
        }))
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T: Debug> Drop for Node<T> {
    fn drop(&mut self) {
        info!("Drop Node<T>: {:?}", self);
        let mut next = unsafe { (*self.next.cell.get()).take() };
        while let Some(node) = next.take() {
            if Arc::strong_count(&node) > 1 {
                continue;
            }
            next = unsafe { (*(*node.cell.get()).next.cell.get()).take() };
            debug!("Dropping node: {:?}", next);
        }
        debug!("Leaving Drop");
    }
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug Node");
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
