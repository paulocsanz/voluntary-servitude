use iter::VSReadIter;
use std::{
    fmt::{self, Debug, Formatter},
    mem::drop,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use types::*;

/// Appendable list that can become a lockfree iterator (one append blocks others - lock write only)
///
/// Parallel examples in main lib docs
///
/// ```
/// # #[macro_use] extern crate voluntary_servitude;
/// # use voluntary_servitude::VSRead;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// let list: VSRead<()> = vsread![]; // or VSRead::default();
/// assert_eq!(list.iter().count(), 0);
///
/// let list = vsread![3, 2];
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
///
/// list.clear();
/// assert_eq!(list.iter().count(), 0);
///
/// list.append(2);
/// list.append(3);
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&2, &3]);
///
/// list.append(3);
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&2, &3, &3]);
/// let list = vsread![3; 3];
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &3, &3]);
/// for el in list.iter() {
///     assert_eq!(el, &3);
/// }
///
/// let mut iter = list.iter();
/// let _ = iter.next();
/// let _ = iter.next();
/// let _ = iter.next();
/// assert_eq!(iter.next(), None);
/// ```
pub struct VSRead<T> {
    writing: Arc<Mutex<()>>,
    size: Arc<AtomicUsize>,
    last_element: WrappedWeak<T>,
    node: WrappedNode<T>,
}

impl<T> Default for VSRead<T> {
    fn default() -> Self {
        trace!("Default VSRead");
        VSRead {
            writing: Arc::new(Mutex::new(())),
            size: Arc::new(AtomicUsize::new(0)),
            last_element: VoluntaryServitude::new(None),
            node: VoluntaryServitude::new(None),
        }
    }
}

impl<T> VSRead<T> {
    /// Atomically extracts current size, be careful with data-races when using it
    ///
    /// ```
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # use voluntary_servitude::VSRead;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vsread![3, 2];
    /// assert_eq!(list.len(), 2);
    /// list.append(5);
    /// assert_eq!(list.len(), 3);
    /// list.clear();
    /// assert_eq!(list.len(), 0);
    /// ```
    pub fn len(&self) -> usize {
        trace!("Len VSRead");
        self.size.load(Ordering::Relaxed)
    }

    /// Atomically checks if VSRead is empty, be careful with data-races when using it
    ///
    /// ```
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # use voluntary_servitude::VSRead;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vsread![3, 2];
    /// assert!(!list.is_empty());
    /// list.clear();
    /// assert!(list.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        trace!("Is Empty VSRead");
        self.len() == 0
    }

    /// Makes lock-free iterator based on VSRead
    ///
    /// ```
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # use voluntary_servitude::VSRead;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vsread![3, 2];
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// assert_eq!(list.iter().count(), 2);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// ```
    pub fn iter<'a>(&self) -> VSReadIter<'a, T> {
        trace!("Iter VSRead");
        VSReadIter::new(unsafe { self.node.cell() }, &self.size)
    }

    /// Remove all elements from list (locks writing)
    ///
    /// ```
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # use voluntary_servitude::VSRead;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vsread![3, 2];
    /// assert_eq!(list.iter().count(), 2);
    ///
    /// list.clear();
    /// assert_eq!(list.iter().count(), 0);
    /// ```
    pub fn clear(&self) {
        trace!("Clear VSRead");

        debug!("Clear: Waiting for writing lock");
        let _lock = self.writing.lock().expect("Clear: Mutex Poisoned");
        debug!("Clear: Holding lock");

        self.size.store(0, Ordering::Relaxed);
        unsafe {
            *self.last_element.cell() = None;
            *self.node.cell() = None;
        }
    }

    /// Insert element in Option and update last_element
    fn fill_node(&self, node: *mut Option<ArcNode<T>>, value: T) {
        trace!("Fill Node VSRead");
        let next = Node::arc_node(value);
        let last = Some(Arc::downgrade(&next));
        unsafe {
            *node = Some(next);
            *self.last_element.cell() = last;
        }
        let _size = self.size.fetch_add(1, Ordering::Relaxed);
        trace!("Fill: Increased size to: {}", _size + 1);
    }

    /// Insert element after last node (locks write)
    ///
    /// ```
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # use voluntary_servitude::VSRead;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vsread![]; // or VSRead::default()
    /// assert_eq!(list.iter().count(), 0);
    ///
    /// list.append(3);
    /// list.append(2);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// list.append(8);
    /// list.append(9);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2, &8, &9]);
    /// ```
    pub fn append(&self, value: T) {
        trace!("Append VSRead to {}", self.len());

        debug!("Append: Waiting for writing lock");
        let _lock = self.writing.lock().expect("Append: Mutex Poisoned");
        debug!("Append: Holding lock");

        if self.is_empty() {
            info!("Append: VSRead is empty, inserting first node");
            self.fill_node(self.node.cell.get(), value);
        } else {
            info!("Append: Insert new node to VSRead");
            let last_element = unsafe { self.last_element.cell().take() };
            let last_element = last_element.and_then(|el| el.upgrade());
            if let Some(ref last_next) = last_element.map(|el| unsafe { &el.cell().next }) {
                self.fill_node(last_next.cell.get(), value);
            } else {
                debug_assert!(false, "last_element is None or upgrade failed");
                self.update_last_element();
                info!("Append: Releasing lock to call itself again after fix");
                drop(_lock);
                return self.append(value);
            }
        };
        trace!("Append: Releasing lock");
    }

    /// Re-obtain last element by iterating over list while locked - O(n)
    ///
    /// This should never be executed, but it's here to ensure things don't break in prod
    ///
    /// Won't be called in debug
    fn update_last_element(&self) {
        warn!("Update: Forcefully update self.last_element - O(n)");
        let mut node = unsafe { self.node.cell().as_ref().cloned() };
        let mut last_node = None;
        let mut size = 0;
        while node.is_some() {
            last_node = node.clone();
            size += 1;
            node = node.and_then(|node| unsafe {
                node.cell()
                    .next()
                    .as_ref()
                    .cloned()
                    .or_else(|| None)
            });
        }
        let node = last_node.as_ref().map(|arc| Arc::downgrade(arc));
        unsafe { *self.last_element.cell() = node }
        let _old_size = self.size.swap(size, Ordering::Relaxed);
        debug!("Update: Old size: {}, actual size: {}", _old_size, size);
    }
}

/// Upgrade from weak reference (last_element)
impl<T: Debug> Debug for VSRead<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug VSRead");
        let last_element = unsafe { self.last_element.cell() };
        let last_element = last_element.as_ref().cloned().take().map(|w| w.upgrade());
        write!(
            f,
            "VSRead {{ writing: {:?}, size: {:?}, last_element: {:?}, node: {:?}",
            self.writing, self.size, last_element, self.node
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_logger() {
        #[cfg(feature = "logs")]
        ::setup_logger();
    }

    #[test]
    fn vsread_len() {
        setup_logger();
        let list = vsread![1, 2, 3];
        assert_eq!(list.len(), 3);
        list.append(4);
        assert_eq!(list.len(), 4);
        list.clear();
        assert_eq!(list.len(), 0);
        list.append(4);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn update_last_element() {
        let list = vsread![2, 3];
        unsafe {
            *list.last_element.cell() = None;
            list.update_last_element();
            assert!(list.last_element.cell().is_some());
            let _ = list.last_element.cell()
                .take()
                .and_then(|el| el.upgrade())
                .map(|el| &*el.cell.get())
                .map(|el| assert_eq!(el.value, 3));
        }

        let list: VSRead<()> = vsread![];
        list.update_last_element();
        unsafe {
            assert!(list.last_element.cell().is_none());
        }
    }

    #[test]
    fn fill_node() {
        setup_logger();
        let list = vsread![3, 2];
        let node = unsafe { list.node.cell() };
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);

        list.fill_node(node, 9);
        let _ = list.size.swap(1, Ordering::Relaxed);
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&9]);
        list.fill_node(unsafe { node.clone().unwrap().cell().next() }, 8);
        assert_eq!(list.iter().collect::<Vec<_>>(), vec![&9, &8]);
        assert_eq!(list.size.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VSRead<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VSRead<()>>();
    }
}
