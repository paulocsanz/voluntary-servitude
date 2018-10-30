//! Lock-free iterator based on [`VoluntaryServitude`] (also called [`VS`])
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html

use std::sync::Arc;
use {node::Node, voluntary_servitude::Inner};

/// Lock-free iterator based on [`VS`]
///
/// [`VS`]: ./type.VS.html
#[derive(Debug)]
pub struct Iter<'a, T: 'a> {
    /// References `Inner` extracted from [`VS`]
    ///
    /// [`VS`]: ./struct.VS.html
    inner: Arc<Inner<T>>,
    /// Current node in iteration
    current: Option<&'a Node<T>>,
    /// Iteration index
    index: usize,
}

impl<'a, T: 'a> Iter<'a, T> {
    /// Creates a new lock-free iterator
    #[inline]
    pub(crate) fn new(inner: Arc<Inner<T>>) -> Iter<'a, T> {
        trace!("new()");
        Self {
            current: inner.first_node().map(|nn| unsafe { &*nn.as_ptr() }),
            inner,
            index: 0,
        }
    }

    /// Obtains current iterator index
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let vs = vs![3];
    /// let mut iter = vs.iter();
    /// assert_eq!(iter.next(), Some(&3));
    /// assert_eq!(iter.index(), 1);
    /// assert!(iter.next().is_none());
    /// assert_eq!(iter.index(), 1);
    /// ```
    #[inline]
    pub fn index(&self) -> usize {
        trace!("index() = {}", self.index);
        self.index
    }

    /// Returns current iterator size (may grow, but not decrease)
    ///
    /// If `Iter` is empty it will never grow
    ///
    /// Length won't increase after iterator is emptied (`self.next() == None`)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let vs = vs![3];
    /// let iter = vs.iter();
    /// assert_eq!(iter.len(), 1);
    /// vs.append(2);
    /// vs.clear();
    /// assert_eq!(iter.len(), 2);
    ///
    /// let mut iter2 = vs.iter();
    /// assert_eq!(iter2.next(), None);
    /// assert_eq!(iter2.len(), 0);
    ///
    /// let iter = vs.iter();
    /// vs.append(2);
    /// assert_eq!(iter.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.current
            .map_or_else(|| self.index(), |_| self.inner.len())
    }

    /// Checks if iterator's length is 0 (it will always return the same value)
    ///
    /// If the iterator is empty, it will never grow
    ///
    /// If the iterator is filled, it will never be empty
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let vs = vs![3];
    /// let iter = vs.iter();
    /// assert!(!iter.is_empty());
    /// vs.clear();
    /// assert!(!iter.is_empty());
    /// let iter = vs.iter();
    /// assert!(iter.is_empty());
    /// vs.append(2);
    /// assert!(iter.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        trace!("is_empty()");
        self.len() == 0
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        trace!("next()");

        let data = self.current.map(|node| (node.value(), self.index += 1).0);
        debug!(
            "Current: data.is_some() = {}, self.index = {}",
            data.is_some(),
            self.index
        );

        let current = self.current.take().filter(|_| self.index() <= self.len());
        self.current = current.and_then(|node| node.next());
        data
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        trace!("size_hint()");
        (self.index, Some(self.len()))
    }
}

#[cfg(test)]
mod tests {
    use voluntary_servitude::VS;

    fn setup_logger() {
        #[cfg(feature = "logs")]
        ::setup_logger();
    }

    #[test]
    fn iter_all() {
        setup_logger();
        let vs = vs![1, 2, 3];
        let mut iter = vs.iter();
        assert_eq!(iter.index(), 0);
        assert_eq!(iter.len(), 3);

        vs.append(4);
        assert_eq!(vs.len(), 4);
        assert_eq!(iter.index(), 0);
        assert_eq!(iter.len(), 4);

        let _ = (1..5)
            .map(|n| {
                assert_eq!(iter.next(), Some(&n));
                assert_eq!(iter.index(), n);
            }).count();
        assert_eq!(iter.index(), iter.len());

        vs.clear();
        assert_eq!(vs.len(), 0);
        assert_eq!(iter.len(), 4);
        let iter = vs.iter();
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn iter_isnt_growable_when_consumed() {
        setup_logger();
        let vs: VS<()> = voluntary_servitude![];
        let mut iter = vs.iter();
        vs.append(());
        assert!(iter.is_empty());
        assert!(iter.next().is_none());

        let vs: VS<()> = voluntary_servitude![()];
        vs.append(());
        let mut iter = vs.iter();
        assert_eq!(iter.next(), Some(&()));
        assert_eq!(iter.next(), Some(&()));
        vs.append(());
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_doesnt_clear() {
        setup_logger();
        let vs = voluntary_servitude![()];
        let mut iter = vs.iter();

        assert!(!vs.is_empty());
        vs.clear();
        assert!(vs.is_empty());

        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some(&()));
    }

    #[test]
    fn iter_grows() {
        setup_logger();
        let vs = voluntary_servitude![1, 2, 3];
        let iter = vs.iter();
        let iter2 = vs.iter();
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3]);

        vs.append(4);
        assert_eq!(iter2.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
        let iter = vs.iter();
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
    }

    #[test]
    fn iter_many() {
        setup_logger();
        let vs = vs![1, 2, 3, 4, 5];
        let mut iter = vs.iter();
        let iter1 = vs.iter();
        let iter2 = vs.iter();
        assert_eq!(iter2.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        let iter3 = vs.iter();
        assert_eq!(iter1.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter3.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        assert_eq!(iter.collect::<Vec<&i32>>(), vec![&2, &3, &4, &5]);
    }

    #[test]
    fn iter_after_use() {
        setup_logger();
        let vs = vs![1];
        let mut iter = vs.iter();
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.index(), iter.len());

        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert!(iter.next().is_none());
        assert_eq!(iter.index(), iter.len());
    }

    #[test]
    fn iter_drop() {
        setup_logger();
        let vs = vs![1, 2, 3, 4, 5];
        drop(vs.iter());

        let mut iter = vs.iter();
        assert_eq!(iter.next(), Some(&1));
        drop(iter);

        let mut iter = vs.iter();
        while iter.next().is_some() {}
        drop(iter);
    }

    #[test]
    fn iter_drop_many() {
        setup_logger();
        let vs = vs![1, 2, 3, 4, 5];
        let iter = vs.iter();
        let mut iter1 = vs.iter();
        let mut iter2 = vs.iter();
        assert_eq!(iter2.next(), Some(&1));
        assert_eq!(iter2.next(), Some(&2));
        let mut iter3 = vs.iter();
        assert_eq!(iter2.next(), Some(&3));
        assert_eq!(iter2.next(), Some(&4));
        assert_eq!(iter2.next(), Some(&5));
        drop(iter2);
        assert_eq!(iter1.next(), Some(&1));
        drop(iter);
        drop(iter1);
        assert_eq!(iter3.next(), Some(&1));
        assert_eq!(iter3.next(), Some(&2));
        drop(iter3);
    }
}
