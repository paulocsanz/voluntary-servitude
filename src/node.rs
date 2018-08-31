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

    /// Extracts next ArcNode from Node
    #[inline(always)]
    #[allow(unknown_lints)]
    #[allow(mut_from_ref)]
    pub unsafe fn next(&self) -> &mut Option<ArcNode<T>> {
        self.next.cell()
    }
}

/// Default Drop is recursive and causes a stackoverflow easily
impl<T> Drop for Node<T> {
    fn drop(&mut self) {
        trace!("Drop Node");
        let mut next = unsafe { self.next().take() };
        while let Some(node) = next.take() {
            if Arc::strong_count(&node) > 1 {
                continue;
            }
            next = unsafe { node.cell().next().take() };
            debug!("Last reference to next node, dropping it too iteratively");
        }
        trace!("Leaving Drop");
    }
}

/// Default Debug is recursive and causes a stackoverflow easily
impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let next = if let Some(ref next) = unsafe { self.next() } {
            let this = unsafe { next.cell() };
            let next = unsafe { this.next().is_some() };
            let next = if next { "Some" } else { "None" };
            format!("Some(Node {{ value: {:?}, next: {} }})", this.value, next)
        } else {
            "None".to_owned()
        };
        write!(f, "Node {{ value: {:?}, next: {} }}", self.value, &next)
    }
}
