use iter::VSReadIter;
use std::{fmt::{self, Debug, Formatter},
          sync::{atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT},
                 Arc,
                 Mutex}};
use types::*;

pub struct VSRead<T: Debug> {
    writing: Arc<Mutex<()>>,
    size: AtomicUsize,
    last_element: WrappedWeak<T>,
    node: WrappedNode<T>,
}

impl<T: Debug> Default for VSRead<T> {
    fn default() -> Self {
        VSRead {
            writing: Arc::new(Mutex::new(())),
            size: ATOMIC_USIZE_INIT,
            last_element: VoluntaryServitude::new(None),
            node: VoluntaryServitude::new(None),
        }
    }
}

impl<T: Debug> VSRead<T> {
    pub fn iter<'a>(&self) -> VSReadIter<'a, T> {
        debug!("Converting VSRead to VSReadIter: {:?}", self);
        let node = unsafe { &*self.node.cell.get() };
        VSReadIter::new(node, &self.size)
    }

    pub fn append(&self, value: T) {
        debug!(
            "Append element to VSRead (size: {}): {:?}",
            self.size.load(Ordering::Relaxed),
            value
        );

        trace!("Waiting for writing lock");
        let _lock = self.writing.lock().unwrap();
        trace!("Holding lock");

        let last_element = self.last_element.cell.get();
        let element = if let Some(ref weak_node) = unsafe { &*last_element } {
            if let Some(node) = weak_node.upgrade() {
                let next = Node::arc_node(value);
                let weak = Arc::downgrade(&next);
                let next = VoluntaryServitude::new(Some(next));
                trace!("Upgraded weak");
                unsafe {
                    (*node.cell.get()).next = next;
                    info!("New node (unsafe): {:?}", (*node.cell.get()).next);
                }
                Some(weak)
            } else {
                crit!("Weak was unable to upgrade, but it should: {:?}", self);
                self.update_last_element();
                warn!("Calling itself again to be properly ran");
                info!("Releasing lock on early return");
                return self.append(value);
            }
        } else {
            let next = Node::arc_node(value);
            let weak = Arc::downgrade(&next);
            unsafe {
                *self.node.cell.get() = Some(next);
                info!("First node (unsafe): {:?}", *self.node.cell.get());
            }
            Some(weak)
        };

        unsafe {
            *last_element = element;
        }

        self.size.fetch_add(1, Ordering::Relaxed);
        trace!("Increased size to: {}", self.size.load(Ordering::Relaxed));
        trace!("Releasing lock: {:?}", self);
    }

    fn update_last_element(&self) {
        debug!("Forcefully update self.last_element - O(n)");
        let last_element = self.last_element.cell.get();
        let mut size = 0;
        let mut last = unsafe { &*self.node.cell.get() };
        let mut next_last = last;

        while let Some(ref next) = next_last {
            size += 1;
            last = next_last;
            next_last = unsafe { &*(*next.cell.get()).next.cell.get() };
        }
        let old_size = self.size.swap(size, Ordering::Relaxed);
        warn!("Old size: {}, actual size: {}", old_size, size);

        unsafe {
            if let Some(last) = last {
                warn!("Obtained last_element again: {:?}", last);
                *last_element = Some(Arc::downgrade(&last));
            } else {
                warn!("No element in list");
                *last_element = None;
            }
        }
    }
}

impl<T: Debug> Debug for VSRead<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        debug!("Debug VSRead");
        let last_element = unsafe { &*self.last_element.cell.get() };
        let last_element = last_element.as_ref().cloned().take().map(|w| w.upgrade());
        write!(
            f,
            "VSRead {{ writing: {:?}, size: {:?}, last_element: {:?}, node: {:?}",
            self.writing, self.size, last_element, self.node
        )
    }
}
