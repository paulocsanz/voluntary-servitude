//! Lock-free iterator based on [`VoluntaryServitude`] (also called [`VS`])
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html

use node::Node;
use std::fmt::{self, Debug, Formatter};
use std::{hash::Hash, hash::Hasher, ptr::null_mut, ptr::NonNull};
use std::{hint::unreachable_unchecked, marker::PhantomData};
use voluntary_servitude::VSInner;

/// Trait that ensures constructor is not available to this crate's users
pub trait CrateConstructor<T> {
    /// VSIter constructor that is only available to this crate
    fn new(inner: &mut VSInner<T>) -> Self;
}

/// Lock-free iterator based on [`VS`]
///
/// [`VS`]: ./type.VS.html
pub struct VSIter<'a, T: 'a> {
    /// References [`VSInner`] extracted from [`VS`]
    ///
    /// [`VSInner`]: ./voluntary_servitude/struct.VSInner.html
    /// [`VS`]: ./struct.VS.html
    inner: Option<NonNull<VSInner<T>>>,
    /// Current node in iteration
    current: Option<NonNull<Node<T>>>,
    /// Iteration index
    index: usize,
    /// Allows iteration to return &T with no cost
    data: PhantomData<&'a T>,
}

impl<'a, T> CrateConstructor<T> for VSIter<'a, T> {
    /// Creates a new lock-free iterator based on [`VSInner`]
    ///
    /// [`VSInner`]: ./struct.VSInner.html
    #[inline]
    fn new(inner: &mut VSInner<T>) -> VSIter<'a, T> {
        trace!("New VSIter");
        Self {
            inner: NonNull::new(inner as *mut _),
            current: NonNull::new(inner.first_node()),
            index: 0,
            data: PhantomData,
        }
    }
}

impl<'a, T> PartialEq for VSIter<'a, T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        trace!("PartialEq VSIter");
        self.inner == other.inner && self.index == other.index && self.current == other.current
    }
}
impl<'a, T> Eq for VSIter<'a, T> {}

/// Creates new [`VSIter`] with the same data and index, increasing reference counter
///
/// [`VSIter`]: ./struct.VSIter.html
impl<'a, T> Clone for VSIter<'a, T> {
    #[inline]
    fn clone(&self) -> Self {
        debug!("Clone VSIter");
        let inner = unsafe {
            self.inner
                .map(|mut nn| nn.as_mut().create_ref())
                .or_else(|| Some(null_mut()))
                .unwrap_or_else(|| unreachable_unchecked())
        };
        VSIter {
            inner: NonNull::new(inner),
            current: self.current,
            index: self.index,
            data: PhantomData,
        }
    }
}

impl<'a, T: Hash> Hash for VSIter<'a, T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        trace!("Hash VSIter");
        let _ = self.clone().map(|el| el.hash(state)).count();
    }
}

impl<'a, T> Drop for VSIter<'a, T> {
    #[inline]
    fn drop(&mut self) {
        trace!("Drop VSIter");
        self.current = None;
        let _ = unsafe { self.inner.take().map(|inner| inner.as_ref().drop_ref()) };
    }
}

/// Default Debug is recursive and causes a stackoverflow easily
impl<'a, T: Debug> Debug for VSIter<'a, T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug VSIter");
        write!(
            f,
            "VSIter {{ inner: {:?}, current: {:?}, index: {:?} }}",
            self.inner(),
            unsafe { self.current.map(|nn| &*nn.as_ptr()) },
            self.index
        )
    }
}

impl<'a, T> VSIter<'a, T> {
    /// Derefs to [`VSInner`], returning Option
    ///
    /// [`VSInner`]: ./struct.VSInner.html
    #[inline]
    fn inner(&self) -> Option<&'a VSInner<T>> {
        trace!("VSInner from VSIter");
        unsafe { self.inner.map(|nn| &*nn.as_ptr()) }
    }

    /// Obtains current iterator index
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let mut iter = vs![3].iter();
    /// assert_eq!(iter.next(), Some(&3));
    /// assert_eq!(iter.index(), 1);
    /// assert!(iter.next().is_none());
    /// assert_eq!(iter.index(), 1);
    /// ```
    #[inline]
    pub fn index(&self) -> usize {
        trace!("VSIter index");
        self.index
    }

    /// Atomically extracts current size of iterator (may grow, but not decrease)
    ///
    /// If [`VSIter`] is empty it will never grow
    ///
    /// Length won't increase after iterator is emptied (self.next() == None)
    ///
    /// [`VSIter`]: ./struct.VSIter.html
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let vs = vs![3];
    /// let iter = vs.iter();
    /// assert_eq!(iter.len(), 1);
    /// vs.append(2);
    /// vs.clear();
    /// assert_eq!(iter.len(), 2);
    /// let iter = vs.iter();
    /// vs.append(2);
    /// assert_eq!(iter.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        trace!("VSIter length");
        self.inner()
            .filter(|_| self.current.is_some())
            .map(|inner| inner.len())
            .or_else(|| Some(self.index))
            .unwrap_or_else(|| unsafe { unreachable_unchecked() })
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
        trace!("VSIter is empty");
        self.len() == 0
    }
}

impl<'a, T> Iterator for VSIter<'a, T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        trace!("Next VSIter element");

        let data = self
            .current
            .map(|nn| unsafe { (*nn.as_ptr()).value() })
            .filter(|_| truth!(self.index += 1));
        debug!("Next: data.is_some() = {}", data.is_some());

        self.current = self
            .current
            .take()
            .filter(|_| self.index <= self.len())
            .and_then(|node| unsafe { node.as_ref().next() });
        data
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        trace!("VSIter Size Hint");
        (self.index, Some(self.len()))
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

    fn new_iter<'a>(num: i32) -> VSIter<'a, i32> {
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
