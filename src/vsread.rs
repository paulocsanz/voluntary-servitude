use iter::VSReadIter;
use std::{
    fmt::{self, Debug, Formatter}, mem,
    sync::{
        atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT}, Arc, Mutex,
    },
};
use types::*;

pub struct VSRead<T: Debug> {
    writing: Arc<Mutex<()>>,
    size: AtomicUsize,
    last_element: WrappedWeak<T>,
    node: WrappedNode<T>,
}

impl<T: Debug> Default for VSRead<T> {
    fn default() -> Self {
        trace!("Default VSRead");
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
        trace!("Converting VSRead to VSReadIter: {:?}", self);
        let node = unsafe { &*self.node.cell.get() };
        VSReadIter::new(node, &self.size)
    }

    fn append_to(&self, node: *mut Option<ArcNode<T>>, value: T) {
        debug!("Append {}: {:?}", self.size.load(Ordering::Relaxed), value);
        let next = Node::arc_node(value);
        let weak = Some(Arc::downgrade(&next));
        unsafe {
            *node = Some(next);
            *self.last_element.cell.get() = weak;
        }
    }

    pub fn append(&self, value: T) {
        debug!("Append {}: {:?}", self.size.load(Ordering::Relaxed), value);

        trace!("Waiting for writing lock");
        let _lock = self.writing.lock().unwrap();
        trace!("Holding lock");

        if self.size.load(Ordering::Relaxed) == 0 {
            self.append_to(self.node.cell.get(), value);
        } else {
            let last_element = unsafe { (*self.last_element.cell.get()).take() };
            let last_element = last_element.and_then(|el| el.upgrade());
            if let Some(ref last_next) = last_element.map(|el| unsafe { &(*el.cell.get()).next }) {
                self.append_to(last_next.cell.get(), value);
            } else {
                crit!("last_element is None or failed to upgrade: {:?}", self);
                self.update_last_element();
                trace!("Releasing lock to call itself again after fix: {:?}", self);
                mem::drop(_lock);
                return self.append(value);
            }
        };

        self.size.fetch_add(1, Ordering::Relaxed);
        trace!("Increased size to: {}", self.size.load(Ordering::Relaxed));
        trace!("Releasing lock: {:?}", self);
    }

    fn update_last_element(&self) {
        warn!("Forcefully update self.last_element - O(n)");
        let mut node = unsafe { (*self.node.cell.get()).as_ref().cloned() };
        let mut size = 0;
        while node.is_some() {
            size += 1;
            node = node.and_then(|node| unsafe {
                (*(*node.cell.get()).next.cell.get())
                    .as_ref()
                    .cloned()
                    .or_else(|| Some(node))
            });
        }
        unsafe {
            *self.last_element.cell.get() = node.as_ref().map(|arc| Arc::downgrade(arc));
        }
        let old_size = self.size.swap(size, Ordering::Relaxed);
        warn!("Old size: {}, actual size: {}", old_size, size);
        info!("self.last_element now is {:?}", self.last_element);
    }
}

impl<T: Debug> Debug for VSRead<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug VSRead");
        let last_element = unsafe { &*self.last_element.cell.get() };
        let last_element = last_element.as_ref().cloned().take().map(|w| w.upgrade());
        write!(
            f,
            "VSRead {{ writing: {:?}, size: {:?}, last_element: {:?}, node: {:?}",
            self.writing, self.size, last_element, self.node
        )
    }
}
