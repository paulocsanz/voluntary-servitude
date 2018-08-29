//! VSRead lockfree iterator

use std::{
    self,
    cmp::max,
    marker::PhantomData,
    sync::{Arc, atomic::{AtomicUsize, Ordering}},
};
use types::*;

#[cfg(feature = "iter-sync")]
type InteriorMutability<T> = std::sync::RwLock<T>;

#[cfg(not(feature = "iter-sync"))]
type InteriorMutability<T> = std::cell::Cell<T>;

#[derive(Debug)]
/// Lockfree iterator
pub struct VSReadIter<'a, T: 'a> {
    current: Option<ArcNode<T>>,
    current_index: usize,
    size: Arc<AtomicUsize>,
    cached_size: InteriorMutability<usize>,
    data: PhantomData<&'a T>,
}

impl<'a, T: 'a> VSReadIter<'a, T> {
    /// Creates new lockfree iterator based on first node and total size
    pub fn new(current: &Option<ArcNode<T>>, size: &Arc<AtomicUsize>) -> VSReadIter<'a, T> {
        trace!(
            "New VSReadIter, current size: {}",
            size.load(Ordering::Relaxed)
        );
        VSReadIter {
            size: Arc::clone(size),
            cached_size: InteriorMutability::new(size.load(Ordering::Relaxed)),
            current: current.as_ref().cloned(),
            current_index: 0,
            data: PhantomData,
        }
    }

    /// Obtains current iterator index
    pub fn index(&self) -> usize {
        trace!("Index VSReadIter");
        self.current_index
    }

    /// Atomically extracts current size of iterator (may grow, but not decrease)
    ///
    /// If the size is originally 0 it will never grow
    /// So we can't continue on the chain if the chain is not there
    ///
    /// Be careful with data-races since the actual value may be bigger when used
    pub fn len(&self) -> usize {
        trace!("Len VSReadIter");
        if self.current_index == 0 && self.current.is_none() {
            0
        } else {
            self.update_cached_size()
        }
    }

    /// Update cache size to store biggest number between itself or self.size (atomic read)
    #[inline(always)]
    fn update_cached_size(&self) -> usize {
        self.set_cached_size(max(self.cached_size(), self.size.load(Ordering::Relaxed)));
        self.cached_size()
    }

    /// Checks if iteration should continue, if doesn't update cached_size and check again
    #[inline(always)]
    fn keep_going(&self) -> bool {
        self.current_index < self.cached_size() || self.current_index < self.update_cached_size()
    }

    #[cfg(not(feature = "iter-sync"))]
    #[inline(always)]
    fn set_cached_size(&self, value: usize) {
        self.cached_size.set(value);
    }

    #[cfg(feature = "iter-sync")]
    #[inline(always)]
    fn set_cached_size(&self, value: usize) {
        *self.cached_size.write().unwrap() = value;
    }

    #[cfg(not(feature = "iter-sync"))]
    #[inline(always)]
    fn cached_size(&self) -> usize {
        self.cached_size.get()
    }

    #[cfg(feature = "iter-sync")]
    #[inline(always)]
    fn cached_size(&self) -> usize {
        *self.cached_size.read().unwrap()
    }
}

impl<'a, T: 'a> Iterator for VSReadIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        trace!("Next VSReadIter element");

        let data = self
            .current
            .as_ref()
            .map(|vs| unsafe { &vs.cell().value })
            .map(|v| {
                trace!("Next: Increasing 1 in self.current_index");
                self.current_index += 1;
                v
            });
        debug!("Next: data.is_some() = {}", data.is_some());

        debug_assert!(
            data.is_some() || self.cached_size() == 0 || self.current_index == self.cached_size(),
            "data.is_some() = {}, self.size = {}, self.current_index = {}",
            data.is_some(),
            self.size.load(Ordering::Relaxed),
            self.current_index
        );

        self.current = self
            .current
            .take()
            .filter(|_| self.keep_going())
            .and_then(|vs| unsafe { vs.cell().next().as_ref().cloned() });
        data
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        trace!("VSReadIter Size Hint");
        (self.current_index, Some(self.len()))
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
    fn iter_len_index() {
        let list = vsread![1, 2, 3];
        let mut iter = list.iter();
        list.append(4);
        assert_eq!(list.len(), 4);
        assert_eq!(iter.index(), 0);
        assert_eq!(iter.len(), 4);
        let _ = (1..4)
            .map(|n| {
                assert_eq!(iter.next(), Some(&n));
                assert_eq!(iter.index(), n);
            })
            .count();

        list.clear();
        assert_eq!(list.len(), 0);
        assert_eq!(iter.len(), 4);
        let iter = list.iter();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.index(), 0);
    }

    #[test]
    #[should_panic]
    fn iter_lied_size_more_empty() {
        setup_logger();
        for _ in VSReadIter::<()>::new(&None, &Arc::new(AtomicUsize::new(100))) {}
    }

    #[test]
    #[should_panic]
    fn iter_lied_size_more() {
        setup_logger();
        for _ in VSReadIter::new(&Some(Node::arc_node(0)), &Arc::new(AtomicUsize::new(2))) {}
    }

    #[test]
    fn iter_lied_size_less_more() {
        setup_logger();
        for _ in VSReadIter::new(&new_iter().current, &Arc::new(AtomicUsize::new(5))) {}
    }

    #[test]
    fn iter_lied_size_less() {
        setup_logger();
        for _ in VSReadIter::new(&Some(Node::arc_node(0)), &Arc::new(AtomicUsize::new(0))) {}
    }

    fn new_iter<'a>() -> VSReadIter<'a, i32> {
        let count = 5;
        let first = Some(Node::arc_node(0));
        let mut node = &first;
        for i in 1..count {
            unsafe {
                let this = node.as_ref().unwrap().cell();
                *this.next() = Some(Node::arc_node(i));
                node = this.next();
            }
        }
        VSReadIter::new(&first, &Arc::new(AtomicUsize::new(count as usize)))
    }

    #[test]
    fn iter_many() {
        setup_logger();
        let count = 5;
        let first = Some(Node::arc_node(0));
        let mut node = &first;
        for i in 1..count {
            unsafe {
                let this = node.as_ref().unwrap().cell();
                *this.next() = Some(Node::arc_node(i));
                node = &*this.next();
            }
        }
        let iter1 = VSReadIter::new(&first.as_ref().cloned(), &Arc::new(AtomicUsize::new(count)));
        let iter2 = VSReadIter::new(&first.as_ref().cloned(), &Arc::new(AtomicUsize::new(count)));
        for _ in iter2 {}
        let iter3 = VSReadIter::new(&first.as_ref().cloned(), &Arc::new(AtomicUsize::new(count)));
        for _ in iter1 {}
        for _ in iter3 {}
    }

    #[test]
    fn iter_empty() {
        setup_logger();
        let mut iter = VSReadIter::<()>::new(&None, &Arc::new(AtomicUsize::new(0)));
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_after_use() {
        setup_logger();
        let node = Node::arc_node(0);
        let mut iter = VSReadIter::new(&Some(node), &Arc::new(AtomicUsize::new(1)));
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
                let this = node.as_ref().unwrap().cell();
                *this.next() = Some(Node::arc_node(i));
                node = this.next();
            }
        }
        let mut iter1 = VSReadIter::new(&first.as_ref().cloned(), &Arc::new(AtomicUsize::new(count)));
        let mut iter2 = VSReadIter::new(&first.as_ref().cloned(), &Arc::new(AtomicUsize::new(count)));
        let _ = iter2.next();
        let _ = iter2.next();
        drop(iter2);
        let iter3 = VSReadIter::new(&first.as_ref().cloned(), &Arc::new(AtomicUsize::new(count)));
        let _ = iter1.next();
        drop(iter1);
        drop(iter3);
    }

    #[test]
    fn iter_grows() {
        setup_logger();
        let list = vsread![1, 2, 3];
        let iter = list.iter();
        list.append(4);
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
        let iter = list.iter();
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
    }

    #[test]
    fn iter_doesnt_clear() {
        setup_logger();
        let list = vsread![1, 2, 3];
        let iter = list.iter();
        list.clear();
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3]);
        let iter = list.iter();
        assert_eq!(iter.collect::<Vec<_>>(), Vec::<&i32>::new());
    }

    #[test]
    #[cfg(feature = "iter-sync")]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VSReadIter<()>>();
    }

    #[test]
    #[cfg(feature = "iter-sync")]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VSReadIter<()>>();
    }
}
