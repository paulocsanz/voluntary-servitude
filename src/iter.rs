//! VSRead lockfree iterator

use std::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};
use types::*;

#[derive(Debug, Clone)]
/// Lockfree iterator
pub struct VSReadIter<'a, T: 'a + Debug> {
    current: Option<ArcNode<T>>,
    current_index: usize,
    size: usize,
    data: Option<&'a T>,
}

impl<'a, T: 'a + Debug> VSReadIter<'a, T> {
    /// Creates new lockfree iterator based on first node and total size
    pub fn new(current: &Option<ArcNode<T>>, size: &AtomicUsize) -> VSReadIter<'a, T> {
        trace!("VSReadIter start node: {:?}", current);
        // Get size before to ensure it's always lower or equal to current (no data race)
        let size = size.load(Ordering::Relaxed);
        VSReadIter {
            size,
            current: current.as_ref().cloned(),
            current_index: 0,
            data: None,
        }
    }

    /// Obtains current iterator index
    pub fn index(&self) -> usize {
        self.current_index
    }

    /// Obtains total size of iterator, this never changes
    pub fn len(&self) -> usize {
        if self.current_index == 0 && self.current.is_none() {
            0
        } else {
            self.size
        }
    }
}

impl<'a, T: 'a + Debug> Iterator for VSReadIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        trace!("Next element in {:?}", self);

        let data = self
            .current
            .as_ref()
            .map(|vs| unsafe { &(*vs.cell.get()).value })
            .map(|v| {
                trace!("Increasing 1 in self.current_index");
                self.current_index += 1;
                v
            });
        debug!("Element: {:?}", data);

        debug_assert!(
            data.is_some() || self.size == 0 || self.current_index >= self.size,
            "data = None {:?}",
            self
        );

        self.current = self
            .current
            .take()
            .filter(|_| self.current_index < self.size)
            .and_then(|vs| unsafe {
                let cell = &*vs.cell.get();
                (&*cell.next.cell.get()).as_ref().cloned()
            });
        data
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.current_index, Some(self.len()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_logger() {
        #[cfg(feature = "logs")] ::setup_logger();
    }

    #[test]
    fn iter_len_index() {
        let list = vsread![1, 2, 3];
        let mut iter = list.iter();
        list.append(4);
        assert_eq!(list.len(), 4);
        assert_eq!(iter.index(), 0);
        assert_eq!(iter.len(), 3);
        let _ = (1..4)
            .map(|n| {
                assert_eq!(iter.next(), Some(&n));
                assert_eq!(iter.index(), n);
            })
            .count();

        list.clear();
        assert_eq!(list.len(), 0);
        assert_eq!(iter.len(), 3);
        let iter = list.iter();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.index(), 0);
    }

    #[test]
    #[should_panic]
    fn iter_lied_size_more_empty() {
        setup_logger();
        for _ in VSReadIter::<()>::new(&None, &AtomicUsize::new(100)) {}
    }

    #[test]
    #[should_panic]
    fn iter_lied_size_more() {
        setup_logger();
        for _ in VSReadIter::new(&Some(Node::arc_node(0)), &AtomicUsize::new(2)) {}
    }

    #[test]
    fn iter_lied_size_less_more() {
        setup_logger();
        for _ in VSReadIter::new(&new_iter().current, &AtomicUsize::new(5)) {}
    }

    #[test]
    fn iter_lied_size_less() {
        setup_logger();
        for _ in VSReadIter::new(&Some(Node::arc_node(0)), &AtomicUsize::new(0)) {}
    }

    fn new_iter<'a>() -> VSReadIter<'a, i32> {
        let count = 5;
        let first = Some(Node::arc_node(0));
        let mut node = &first;
        for i in 1..count {
            unsafe {
                let this = &*node.as_ref().unwrap().cell.get();
                *this.next.cell.get() = Some(Node::arc_node(i));
                node = &*this.next.cell.get();
            }
        }
        VSReadIter::new(&first, &AtomicUsize::new(count as usize))
    }

    #[test]
    fn iter_many() {
        setup_logger();
        let count = 5;
        let first = Some(Node::arc_node(0));
        let mut node = &first;
        for i in 1..count {
            unsafe {
                let this = &*node.as_ref().unwrap().cell.get();
                *this.next.cell.get() = Some(Node::arc_node(i));
                node = &*this.next.cell.get();
            }
        }
        let iter1 = VSReadIter::new(&first.as_ref().cloned(), &AtomicUsize::new(count));
        let iter2 = VSReadIter::new(&first.as_ref().cloned(), &AtomicUsize::new(count));
        for _ in iter2 {}
        let iter3 = VSReadIter::new(&first.as_ref().cloned(), &AtomicUsize::new(count));
        for _ in iter1 {}
        for _ in iter3 {}
    }

    #[test]
    fn iter_empty() {
        setup_logger();
        let mut iter = VSReadIter::<()>::new(&None, &AtomicUsize::new(0));
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_after_use() {
        setup_logger();
        let node = Node::arc_node(0);
        let mut iter = VSReadIter::new(&Some(node), &AtomicUsize::new(1));
        assert_eq!(Some(&0), iter.next());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_drop_new() {
        setup_logger();
        let _ = new_iter();
    }

    #[test]
    fn iter_drop_next() {
        setup_logger();
        let mut iter = new_iter();
        assert_eq!(iter.next(), Some(&0));
    }

    #[test]
    fn iter_drop_empty() {
        setup_logger();
        let mut iter = new_iter();
        while iter.next().is_some() {}
    }

    #[test]
    fn iter_drop_many() {
        setup_logger();
        let count = 5;
        let first = Some(Node::arc_node(0));
        let mut node = &first;
        for i in 1..count {
            unsafe {
                let this = &*node.as_ref().unwrap().cell.get();
                *this.next.cell.get() = Some(Node::arc_node(i));
                node = &*this.next.cell.get();
            }
        }
        let mut iter1 = VSReadIter::new(&first.as_ref().cloned(), &AtomicUsize::new(count));
        let mut iter2 = VSReadIter::new(&first.as_ref().cloned(), &AtomicUsize::new(count));
        let _ = iter2.next();
        let _ = iter2.next();
        drop(iter2);
        let iter3 = VSReadIter::new(&first.as_ref().cloned(), &AtomicUsize::new(count));
        let _ = iter1.next();
        drop(iter1);
        drop(iter3);
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VSReadIter<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VSReadIter<()>>();
    }
}
