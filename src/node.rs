use std::{
    fmt::{self, Debug, Formatter}, mem, sync::Arc,
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
        trace!("Drop");
        info!("Drop Node<T>: {:?}", self);
        let mut node = self;
        loop {
            let next = node.next.cell.get();
            let count = if let Some(ref next) = unsafe { &*next } {
                let (weak, strong) = (Arc::weak_count(next), Arc::strong_count(next));
                trace!("Next strong count: {}, weak count: {}", strong, weak);
                strong
            } else {
                debug!("No next node: {:?}", node);
                mem::drop(node);
                break;
            };

            if count > 1 {
                unsafe { *next = None };
                debug!("Strong count bigger than 1, leave");
                mem::drop(node);
                break;
            }

            if let Some(next) = unsafe { (*next).take().map(|n| &mut *n.cell.get()) } {
                mem::drop(node);
                trace!("Next: {:?}", next);
                node = next;
            }
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
