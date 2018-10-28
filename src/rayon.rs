//! Rayon trait implementations for [`VoluntaryServitude`] (also has a useful thread-safe iterator)
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html

use node::Node;
use rayon_lib::{iter::plumbing::UnindexedConsumer, prelude::*};
use std::fmt::{self, Debug, Formatter};
use std::marker::PhantomData;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::{ptr::null_mut, ptr::NonNull};
use voluntary_servitude::Inner;

/// Trait that ensures constructor is not available to this crate's users
pub trait CrateConstructor<'a, T: 'a> {
    /// [`ParIter`] constructor that is only available to this crate
    ///
    /// [`ParIter`] ./struct.ParIter.html
    fn new(inner: &'a Inner<T>) -> Self;
}

/// Parallel lock-free iterator based on [`VS`]
///
/// [`VS`]: ./type.VS.html
#[derive(Debug)]
pub struct ParIter<'a, T: 'a + Sync> {
    /// References `Inner` extracted from [`VS`]
    ///
    /// [`VS`]: ./struct.VS.html
    inner: Option<&'a Inner<T>>,
    /// Current node in iteration
    current: AtomicPtr<Node<T>>,
    /// Iteration index
    index: AtomicUsize,
}

/*
impl<'a, T: Sync> CrateConstructor<'a, T> for ParIter<'a, T> {
    /// Creates a new lock-free iterator based on `Inner`
    #[inline]
    fn new(inner: &'a Inner<T>) -> Self {
        trace!("New ParIter");
        Self {
            inner,
            current: AtomicPtr::new(inner.first_node()),
            index: AtomicUsize::new(0),
            data: PhantomData,
        }
    }
}

/// Creates new [`ParIter`] with the same data and index, increasing reference counter
///
/// [`ParIter`]: ./struct.ParIter.html
impl<'a, T: Sync> Clone for ParIter<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        debug!("Clone ParIter");
        let inner = self.inner
            .map(|mut nn| unsafe { nn.as_mut().create_ref()} )
            .unwrap_or(null_mut());
        ParIter {
            inner: NonNull::new(inner),
            current: AtomicPtr::new(self.current.load(Ordering::SeqCst)),
            index: AtomicUsize::new(self.index.load(Ordering::SeqCst)),
            data: PhantomData,
        }
    }
}

impl<'a, T: Sync> Drop for ParIter<'a, T> {
    #[inline]
    fn drop(&mut self) {
        trace!("Drop ParIter");
        self.current = None;
        let _ = unsafe { self.inner.take().map(|inner| inner.as_ref().drop_ref()) };
    }
}

/// Default Debug is recursive and causes a stackoverflow easily
impl<'a, T: Debug + Sync> Debug for ParIter<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug ParIter");
        write!(
            f,
            "ParIter {{ inner: {:?}, current: {:?}, index: {:?} }}",
            self.inner(),
            unsafe { *self.current.load(Ordering::SeqCst) },
            self.index
        )
    }
}

impl<'a, T: Sync> ParIter<'a, T> {
    /// Derefs to `Inner`, returning Option
    #[inline]
    fn inner(&self) -> Option<&'a Inner<T>> {
        trace!("Inner from ParIter");
        unsafe { self.inner.map(|nn| &*nn.as_ptr()) }
    }

    /// Atomically extracts current size of iterator (may grow, but not decrease)
    ///
    /// If [`ParIter`] is empty it will never grow
    ///
    /// Length won't increase after iterator is emptied (self.next() == None)
    ///
    /// [`ParIter`]: ./struct.ParIter.html
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let vs = vs![3];
    /// let iter = vs.par_iter();
    /// assert_eq!(iter.len(), 1);
    /// vs.append(2);
    /// vs.clear();
    /// assert_eq!(vs.next(), None);
    /// assert_eq!(vs.len(), 0);
    /// assert_eq!(iter.len(), 2);
    /// let iter = vs.par_iter();
    /// vs.append(2);
    /// assert_eq!(iter.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        trace!("ParIter length");
        self.inner()
            .filter(|_| self.current.is_some())
            .map(|inner| inner.len())
            .unwrap_or(self.index.load(Ordering::SeqCst))
    }

    /// Atomically checks if iterator is empty (it will always return the same value)
    ///
    /// If a iterator is empty, it will never grow
    ///
    /// If a iterator is filled, it will never be empty
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let vs = vs![3];
    /// let iter = vs.par_iter();
    /// assert!(!iter.is_empty());
    /// vs.clear();
    /// assert!(!iter.is_empty());
    /// let iter = vs.par_iter();
    /// assert!(iter.is_empty());
    /// vs.append(2);
    /// assert!(iter.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        trace!("ParIter is empty");
        self.len() == 0
    }
}

impl<'a, T: Sync> ParallelIterator for ParIter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn drive_unindexed<C: UnindexedConsumer<Self::Item>>(self, consumer: C) -> C::Result {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use voluntary_servitude::VS;

    fn setup_logger() {
        #[cfg(feature = "logs")]
        ::setup_logger();
    }

    #[test]
    fn iter_all() {
        let vs = voluntary_servitude![1, 2, 3];
        let mut iter = vs.par_iter();
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

    fn new_iter<'a>(num: i32) -> Iter<'a, i32> {
        let vs = voluntary_servitude![];
        for i in 1..num + 1 {
            vs.append(i);
        }
        vs.iter()
    }

    #[test]
    fn iter_many() {
        setup_logger();
        let mut iter = new_iter(5);
        let iter1 = iter.clone();
        let iter2 = iter1.clone();
        let iter3 = iter2.clone();
        assert_eq!(iter2.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        assert_eq!(iter1.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);

        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.clone().collect::<Vec<&i32>>(), vec![&2, &3, &4, &5]);
        let iter4 = iter.clone();

        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter4.collect::<Vec<&i32>>(), vec![&2, &3, &4, &5]);
        assert_eq!(iter.collect::<Vec<&i32>>(), vec![&3, &4, &5]);

        assert_eq!(iter3.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
    }

    #[test]
    fn iter_after_use() {
        setup_logger();
        let mut iter = new_iter(1);
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
        drop(new_iter(5));

        let mut iter = new_iter(5);
        assert_eq!(iter.next(), Some(&1));
        drop(iter);

        let mut iter = new_iter(5);
        while iter.next().is_some() {}
        drop(iter);
    }

    #[test]
    fn iter_drop_many() {
        setup_logger();
        let iter = new_iter(5);
        let mut iter1 = iter.clone();
        let mut iter2 = iter.clone();
        assert_eq!(iter2.next(), Some(&1));
        assert_eq!(iter2.next(), Some(&2));
        let mut iter3 = iter2.clone();
        assert_eq!(iter2.next(), Some(&3));
        assert_eq!(iter2.next(), Some(&4));
        assert_eq!(iter2.next(), Some(&5));
        drop(iter2);
        assert_eq!(iter1.next(), Some(&1));
        drop(iter);
        drop(iter1);
        assert_eq!(iter3.next(), Some(&3));
        assert_eq!(iter3.next(), Some(&4));
        drop(iter3);
    }
}
*/
