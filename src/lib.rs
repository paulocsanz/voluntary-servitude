#[macro_use]
extern crate log;

use std::{cell::UnsafeCell,
          fmt::{self, Debug, Formatter},
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

impl<T: Debug> Debug for VoluntaryServitude<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "VoluntaryServitude {{ cell: UnsafeCell {{ {:?} }} }}",
            unsafe { &*self.cell.get() }
        )
    }
}

impl<T: Debug> VoluntaryServitude<T> {
    fn new(value: T) -> VoluntaryServitude<T> {
        trace!("New VoluntaryServitude based on {:?}", value);
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
        trace!("New ArcNode Based on {:?}", value);
        Arc::new(VoluntaryServitude::new(Node {
            value,
            next: VoluntaryServitude::new(None),
        }))
    }
}

#[derive(Debug)]
pub struct VSReadIter<'a, T: 'a> {
    current: Option<ArcNode<T>>,
    current_index: usize,
    size: usize,
    data: Option<&'a T>,
}

impl<'a, T: 'a + Debug> Iterator for VSReadIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        trace!("Next element in {:?}", self);
        if self.current_index == self.size {
            trace!("No more elements in VSReadIter");
            self.data = None;
        } else if self.current_index == 0 && self.current.is_some() {
            trace!("First VSReadIter element");
            if let Some(ref current) = self.current {
                self.data = unsafe { Some(&(*current.cell.get()).value) };
                self.current_index += 1;
            } else {
                crit!("Expected first value but found none: {:?}", self);
                self.data = None;
            }
        } else if self.current.is_some() {
            trace!("Current is Some");
            self.current = if let Some(ref current) = self.current {
                let curr = current.cell.get();
                let node = match unsafe { &(*(*curr).next.cell.get()) } {
                    Some(ref next) => {
                        trace!("Found next node: {:?}", next);
                        self.current_index += 1;
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

pub struct VSRead<T> {
    writing: Mutex<()>,
    size: AtomicUsize,
    last_element: WrappedWeak<T>,
    node: WrappedNode<T>,
}

impl<T: Debug> Debug for VSRead<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
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
            writing: Mutex::new(()),
            size: ATOMIC_USIZE_INIT,
            last_element: VoluntaryServitude::new(None),
            node: VoluntaryServitude::new(None),
        }
    }
}

impl<T: Debug> VSRead<T> {
    pub fn iter<'a>(&self) -> VSReadIter<'a, T> {
        trace!("Converting VSRead to VSReadIter: {:?}", self);
        if let Some(ref node) = unsafe { &*self.node.cell.get() } {
            trace!("VSReadIter start node: {:?}", node);
            VSReadIter {
                current: Some(Arc::clone(node)),
                current_index: 0,
                size: self.size.load(Ordering::Relaxed),
                data: None,
            }
        } else {
            VSReadIter {
                current: None,
                current_index: 0,
                size: 0,
                data: None,
            }
        }
    }

    pub fn append(&self, value: T) {
        trace!(
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
                }
                trace!("Inserted new node after last (unsafe)");
                Some(weak)
            } else {
                crit!("Weak was unable to upgrade, but it should: {:?}", self);
                let mut size = 0;
                let mut last = unsafe { &*self.node.cell.get() };
                loop {
                    last = if let Some(ref last) = last {
                        let next_last = unsafe { &*(*last.cell.get()).next.cell.get() };
                        if next_last.is_some() {
                            size += 1;
                        } else {
                            break;
                        }
                        next_last
                    } else {
                        last
                    }
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
                    crit!("No element in list");
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
            trace!("First element to be inserted");
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
        for i in 0..20 {
            list.append(i);
        }

        for (i, el) in list.iter().enumerate() {
            assert_eq!(&i, el);
        }
    }

    #[test]
    fn single_producer_single_consumer() {
        setup();
        let count = 20;
        let list = Arc::new(VSRead::default());

        let list_clone = Arc::clone(&list);
        let _ = spawn(move || {
            for i in 0..count {
                list_clone.append(i + 1)
            }
        });

        let mut total_max = 0;
        let mut last_len = 0;
        for _ in 0..count {
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
    }

    #[test]
    fn multi_producers_single_consumer() {
        setup();
        let count = 200;
        let list = Arc::new(VSRead::default());
        let num_producers = 10;
        let mut producers = vec![];
        let finished = Arc::new(AtomicUsize::new(0));

        for _ in 0..num_producers {
            let finished_clone = Arc::clone(&finished);
            let list_clone = Arc::clone(&list);
            producers.push(spawn(move || {
                for i in 0..count {
                    list_clone.append(i);
                }
                finished_clone.fetch_add(1, Ordering::Relaxed);
            }));
        }

        while finished.load(Ordering::Relaxed) < num_producers {}
        let len = list.iter().count();
        assert_eq!(len, num_producers * count);
    }

    #[test]
    fn single_producer_multi_consumer() {
        setup();
        let count = 200;
        let list = Arc::new(VSRead::default());
        let num_consumers = 10;
        let mut consumers = vec![];
        let finished = Arc::new(AtomicBool::new(false));

        for _ in 0..num_consumers {
            let finished_clone = Arc::clone(&finished);
            let list_clone = Arc::clone(&list);
            consumers.push(spawn(move || {
                let mut len = 0;
                while !finished_clone.load(Ordering::Relaxed) {
                    let inner_len = list_clone.iter().count();
                    assert!(inner_len >= len);
                    len = inner_len;
                }
                len = list_clone.iter().count();
                assert_eq!(len, list_clone.iter().count());
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
}
