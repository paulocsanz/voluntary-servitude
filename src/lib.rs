#[macro_use]
extern crate log;

use std::{cell::UnsafeCell,
          fmt::{self, Debug, Formatter},
          mem,
          sync::{atomic::{AtomicUsize, Ordering, ATOMIC_USIZE_INIT},
                 Arc,
                 Mutex,
                 Weak}};

macro_rules! crit {
    ($($x: expr),*) => {{
        error!("CRITICAL ERROR");
        error!($($x),*);
        debug_assert!(false, "This should never happen but it did, something is broken and should be fixed");
    }}
}

struct VoluntaryServitude<T> {
    pub cell: UnsafeCell<T>,
}

impl<T: Debug> Debug for VoluntaryServitude<Option<ArcNode<T>>> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        debug!("Debug VoluntaryServitude<Option<ArcNode<T>>>");
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ {:?} }} }}",
            unsafe { &*self.cell.get() }
        )
    }
}
impl<T: Debug> Debug for VoluntaryServitude<Option<WeakNode<T>>> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        debug!("Debug VoluntaryServitude<Option<WeakNode<T>>>");
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ {:?} }} }}",
            unsafe { &*self.cell.get() }
        )
    }
}

impl<T: Debug> Debug for VoluntaryServitude<Option<Node<T>>> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        debug!("Debug VoluntaryServitude<Option<Node<T>>>");
        let node = unsafe { &*self.cell.get() };
        let opt = if let Some(ref node) = node {
            let is_next = unsafe { (*node.next.cell.get()).is_some() };
            let next_opt = if is_next { "Some" } else { "None" };
            format!("Some(Node {{ value: {:?}, next: {} }})", node, next_opt)
        } else {
            "None".to_owned()
        };
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ {} }} }}",
            &opt
        )
    }
}

impl<T: Debug> Debug for VoluntaryServitude<Node<T>> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        debug!("Debug VoluntaryServitude<Node<T>>");
        let node = unsafe { &*self.cell.get() };
        let opt = if unsafe { (*node.next.cell.get()).is_some() } {
            "Some"
        } else {
            "None"
        };
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ Node {{ value: {:?}, next: {} }} }} }}",
            node.value, opt
        )
    }
}

impl<T: Debug> VoluntaryServitude<T> {
    fn new(value: T) -> VoluntaryServitude<T> {
        debug!("New VoluntaryServitude based on {:?}", value);
        VoluntaryServitude {
            cell: UnsafeCell::new(value),
        }
    }
}

unsafe impl<T> Sync for VoluntaryServitude<T> {}

type ArcNode<T> = Arc<VoluntaryServitude<Node<T>>>;
type WeakNode<T> = Weak<VoluntaryServitude<Node<T>>>;
type WrappedNode<T> = VoluntaryServitude<Option<ArcNode<T>>>;
type WrappedWeak<T> = VoluntaryServitude<Option<WeakNode<T>>>;

#[derive(Debug)]
struct Node<T> {
    value: T,
    next: WrappedNode<T>,
}

impl<T: Debug> Node<T> {
    fn arc_node(value: T) -> ArcNode<T> {
        debug!("New ArcNode Based on {:?}", value);
        Arc::new(VoluntaryServitude::new(Node {
            value,
            next: VoluntaryServitude::new(None),
        }))
    }
}

#[derive(Debug)]
pub struct VSReadIter<'a, T: 'a + Debug> {
    dropping: Arc<Mutex<()>>,
    current: Option<ArcNode<T>>,
    current_index: usize,
    size: usize,
    data: Option<&'a T>,
}

impl<'a, T: 'a + Debug> Drop for VSReadIter<'a, T> {
    fn drop(&mut self) {
        debug!("Drop VSReadIter");
        info!("{:?}", self);

        let _data = self.data.take();

        let next = self.current.take();
        info!("Next node: {:?}", next);

        VSRead::drop_nodes(next, &self.dropping);
    }
}

impl<'a, T: 'a + Debug> Iterator for VSReadIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        debug!("Next element in {:?}", self);
        if self.current_index == self.size {
            info!("No more elements in VSReadIter");
            self.data = None;
        } else if self.current_index == 0 && self.current.is_some() {
            if let Some(ref current) = self.current {
                self.data = unsafe { Some(&(*current.cell.get()).value) };
                self.current_index += 1;
            } else {
                crit!("Expected first value but found none: {:?}", self);
                self.data = None;
            }
            info!(
                "First element in VSReadIter ({}): {:?}",
                self.current_index, self.current
            );
        } else if self.current.is_some() {
            trace!("Current is Some");
            self.current = if let Some(ref current) = self.current {
                let curr = current.cell.get();
                let node = match unsafe { &(*(*curr).next.cell.get()) } {
                    Some(ref next) => {
                        self.current_index += 1;
                        info!("Found next node ({}): {:?}", self.current_index, next);
                        Arc::clone(next)
                    }
                    None => {
                        crit!("Expected node, but found None");
                        self.size = self.current_index;
                        self.data = None;
                        return self.data;
                    }
                };

                unsafe {
                    self.data = Some(&(*node.cell.get()).value);
                }

                Some(node)
            } else {
                crit!("Current is None when it shouldn't: {:?}", self);
                None
            }
        } else {
            crit!("self.current is None but it shouldn't: {:?}", self);
            self.size = self.current_index;
            self.data = None;
        }

        trace!("Element: {:?}", self.data);
        self.data
    }
}

pub struct VSRead<T: Debug> {
    writing: Arc<Mutex<()>>,
    size: AtomicUsize,
    last_element: WrappedWeak<T>,
    node: WrappedNode<T>,
}

impl<T: Debug> Drop for VSRead<T> {
    fn drop(&mut self) {
        debug!("Drop VSRead");
        info!("{:?}", self);

        let last_element_weak = unsafe { (*self.last_element.cell.get()).take() };
        info!("self.last_element = {:?}", last_element_weak);

        let next = unsafe { (*self.node.cell.get()).take() };
        info!("Next node: {:?}", next);

        VSRead::drop_nodes(next, &self.writing);
    }
}

impl<T: Debug> Debug for VSRead<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        debug!("Debug VSRead");
        let last_element = if let Some(ref weak_node) = unsafe { (&*self.last_element.cell.get()) }
        {
            Some(
                (*weak_node)
                    .upgrade()
                    .expect("Failed to upgrade last_element's Weak"),
            )
        } else {
            None
        };
        write!(
            f,
            "VSRead {{ writing: {:?}, size: {:?}, last_element: {:?}, node: {:?}",
            self.writing, self.size, last_element, self.node
        )
    }
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
        if let Some(ref node) = unsafe { &*self.node.cell.get() } {
            trace!("VSReadIter start node: {:?}", node);
            VSReadIter {
                dropping: Arc::clone(&self.writing),
                current: Some(Arc::clone(node)),
                current_index: 0,
                size: self.size.load(Ordering::Relaxed),
                data: None,
            }
        } else {
            trace!("VSReadIter is empty");
            VSReadIter {
                dropping: Arc::clone(&self.writing),
                current: None,
                current_index: 0,
                size: 0,
                data: None,
            }
        }
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
                trace!("Upgraded weak");
                let raw_node = node.cell.get();
                let new_node = Node::arc_node(value);
                let weak = Arc::downgrade(&new_node);
                unsafe {
                    (*raw_node).next = VoluntaryServitude::new(Some(new_node));
                    info!(
                        "Inserted new node after last (unsafe): {:?}",
                        (*raw_node).next
                    );
                }
                Some(weak)
            } else {
                crit!("Weak was unable to upgrade, but it should: {:?}", self);
                let mut size = 0;
                let mut last = unsafe { &*self.node.cell.get() };
                let mut next_last = last;
                while let Some(ref next) = next_last {
                    size += 1;
                    last = next_last;
                    next_last = unsafe { &*(*next.cell.get()).next.cell.get() };
                }

                if let Some(last) = last {
                    warn!(
                        "Obtained last_element again O({}): {:?}",
                        self.size.load(Ordering::Relaxed),
                        last
                    );
                    unsafe {
                        *last_element = Some(Arc::downgrade(&last));
                    }
                    self.size.store(size, Ordering::Relaxed);
                    warn!("Calling itself again to be properly ran");
                    info!("Releasing lock on early return");
                    return self.append(value);
                } else {
                    warn!("No element in list");
                    self.size.store(0, Ordering::Relaxed);
                    unsafe {
                        *self.node.cell.get() = None;
                        *last_element = None;
                    }
                    warn!("Calling itself again to be properly ran");
                    info!("Releasing lock on early return");
                    return self.append(value);
                }
            }
        } else {
            info!("First element to be inserted");
            let node = Node::arc_node(value);
            let weak = Arc::downgrade(&node);
            unsafe {
                *self.node.cell.get() = Some(node);
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

    /// Iterate over nodes dropping them (avoid stackoverflow on default recursion)
    ///
    /// Locks to properly check Arc strong_counts
    fn drop_nodes(mut next: Option<ArcNode<T>>, mutex: &Arc<Mutex<()>>) {
        info!("Locking to drop");
        let _lock = mutex.lock().unwrap();
        let mut next = if let Some(next) = next.take() {
            if Arc::strong_count(&next) > 1 {
                mem::drop(next);
                info!("Dropped VSRead");
                return;
            }
            Some(next)
        } else {
            None
        };

        while let Some(node) = next.take() {
            let node = unsafe { &*node.cell.get() };
            info!("Dropping node: {:?}", next);
            mem::drop(next);
            info!("Dropped node");
            next = unsafe { (*node.next.cell.get()).take() };
            info!("Took next: {:?}", next);
        }
        info!("Dropped VSRead");
    }
}

#[cfg(test)]
mod tests {
    extern crate env_logger;

    use super::*;
    use std::{cmp::max,
              env::set_var,
              sync::{atomic::AtomicBool, Once, ONCE_INIT},
              thread::spawn};

    static STARTED: Once = ONCE_INIT;

    fn setup() {
        STARTED.call_once(|| {
            set_var("RUST_LOG", "warn");

            env_logger::Builder::from_default_env()
                .default_format_module_path(false)
                .default_format_timestamp(false)
                .init();
        })
    }

    #[test]
    fn single_thread() {
        setup();
        let list = VSRead::default();
        for i in 0..200000 {
            list.append(i);
        }

        for (i, el) in list.iter().enumerate() {
            assert_eq!(&i, el);
        }
    }

    #[test]
    fn single_producer_single_consumer() {
        setup();
        let count = 100000;
        let list = Arc::new(VSRead::default());
        let finished = Arc::new(AtomicBool::new(false));

        let list_clone = Arc::clone(&list);
        let finished_clone = Arc::clone(&finished);
        let _ = spawn(move || {
            for i in 0..count {
                list_clone.append(i + 1)
            }
            finished_clone.store(true, Ordering::Relaxed);
        });

        let mut total_max = 0;
        let mut last_len = 0;
        while !finished.load(Ordering::Relaxed) {
            let mut inner_max = 0;
            let mut len = 0;
            for (i, num) in list.iter().enumerate() {
                assert_eq!(i + 1, *num);
                inner_max = max(i + 1, inner_max);
                len = i;
            }
            assert!(
                inner_max > total_max || inner_max == count || len == last_len,
                "{} > {} || {} == {} || {} == {}",
                inner_max,
                total_max,
                inner_max,
                count,
                len,
                last_len
            );
            last_len = len;
            total_max = inner_max
        }
        assert_eq!(list.iter().count(), count);
    }

    #[test]
    fn multi_producers_single_consumer() {
        setup();
        let count = 100;
        let list = Arc::new(VSRead::default());
        let num_producers = 1000;
        let mut producers = vec![];
        let finished = Arc::new(AtomicUsize::new(0));

        for _ in 0..num_producers {
            let finished = Arc::clone(&finished);
            let list = Arc::clone(&list);
            producers.push(spawn(move || {
                for i in 0..count {
                    list.append(i);
                }
                finished.fetch_add(1, Ordering::Relaxed);
            }));
        }

        let mut last_len = 0;
        while finished.load(Ordering::Relaxed) < num_producers {
            let len = list.iter().count();
            assert!(len >= last_len);
            last_len = len;
        }
        let len = list.iter().count();
        assert_eq!(len, num_producers * count);
    }

    #[test]
    fn single_producer_multi_consumer() {
        setup();
        let count = 100000;
        let list = Arc::new(VSRead::default());
        let num_consumers = 1000;
        let mut consumers = vec![];
        let finished = Arc::new(AtomicBool::new(false));

        for _ in 0..num_consumers {
            let finished = Arc::clone(&finished);
            let list = Arc::clone(&list);
            consumers.push(spawn(move || {
                let mut len = 0;
                while !finished.load(Ordering::Relaxed) {
                    let inner_len = list.iter().count();
                    assert!(inner_len >= len);
                    len = inner_len;
                }
                len = list.iter().count();
                assert_eq!(len, list.iter().count());
                assert_eq!(len, count);
            }));
        }

        for i in 0..count {
            list.append(i);
        }
        finished.store(true, Ordering::Relaxed);

        for thread in consumers {
            thread.join().unwrap();
        }
    }

    #[test]
    fn multi_producer_multi_consumer() {
        setup();
        let count = 100;
        let list = Arc::new(VSRead::default());
        let num_producers = 1000;
        let mut producers = vec![];
        let finished = Arc::new(AtomicUsize::new(0));

        let num_consumers = 1000;
        let mut consumers = vec![];

        for _ in 0..num_producers {
            let finished = Arc::clone(&finished);
            let list = Arc::clone(&list);
            producers.push(spawn(move || {
                for i in 0..count {
                    list.append(i);
                }
                finished.fetch_add(1, Ordering::Relaxed);
            }));
        }

        for _ in 0..num_consumers {
            let finished = Arc::clone(&finished);
            let list = Arc::clone(&list);
            consumers.push(spawn(move || {
                let mut len = 0;
                while finished.load(Ordering::Relaxed) < num_producers {
                    let inner_len = list.iter().count();
                    assert!(inner_len >= len);
                    len = inner_len;
                }
                len = list.iter().count();
                assert_eq!(len, list.iter().count());
                assert_eq!(len, count * num_producers);
            }));
        }

        for (consumer, producer) in consumers.into_iter().zip(producers) {
            consumer.join().unwrap();
            producer.join().unwrap();
        }
    }

    fn elements_n(num: usize) {
        println!("{} users", num);
        setup();
        let list = VSRead::default();
        for i in 0..num {
            list.append(i);
        }
        assert_eq!(list.iter().count(), num);
        assert_eq!(list.iter().next(), Some(&0));
        for (i, el) in list.iter().enumerate() {
            assert_eq!(*el, i);
        }

        let mut iter = list.iter();
        let iter_count = list.iter();
        mem::drop(list);
        assert_eq!(iter_count.count(), num);
        assert_eq!(iter.next(), Some(&0));
    }

    #[test]
    fn elements_1m() {
        elements_n(500_000);
        elements_n(1_000_000);
    }

    #[ignore]
    #[test]
    fn elements_50m() {
        elements_n(10_000_000);
        elements_n(50_000_000);
    }

    #[test]
    #[ignore]
    fn elements_500m() {
        elements_n(100_000_000);
        elements_n(500_000_000);
    }

    #[test]
    #[ignore]
    fn elements_1b() {
        elements_n(1_000_000_000);
    }
}
