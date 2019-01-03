//! Lock-free iterator based on [`VoluntaryServitude`] (also called [`VS`])
//!
//! [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
//! [`VS`]: ./type.VS.html

#[cfg(feature = "logs")]
use crate::prelude::*;
use crate::{node::Node, voluntary_servitude::Inner};
use std::fmt::{self, Debug, Formatter};
use std::{iter::FusedIterator, ptr::NonNull, sync::Arc};

/// Lock-free iterator based on [`VS`]
///
/// To ensure it can exist after `VS` `Iterator` is implemented for `&mut Iter<T>`, so you may have to iterate over `&mut Iter<T>`
///
/// ```rust
/// # #[macro_use] extern crate voluntary_servitude;
/// # env_logger::init();
/// let vs = vs![3, 4, 5];
/// for number in &mut vs.iter() {
///     println!("Number: {}", number);
/// }
/// ```
///
/// That can be avoided with iterator combinators
///
/// ```rust
/// # #[macro_use] extern crate voluntary_servitude;
/// # env_logger::init();
/// let vs = vs![3, 4, 5];
/// let _ = vs.iter().map(|n| println!("Number: {}", n)).count();
/// ```
///
/// [`VS`]: ./type.VS.html
pub struct Iter<T> {
    /// References `Inner` extracted from `VS`
    inner: Arc<Inner<T>>,
    /// Current node in iteration
    current: Option<NonNull<Node<T>>>,
    /// Iteration index
    index: usize,
}

impl<T> Clone for Iter<T> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            current: self.current,
            index: self.index,
        }
    }
}

impl<T: Debug> Debug for Iter<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Iter")
            .field("inner", &self.inner)
            .field(
                "current",
                &self.current.map(|ptr| unsafe { &*ptr.as_ptr() }),
            )
            .field("index", &self.index)
            .finish()
    }
}

impl<T> From<Arc<Inner<T>>> for Iter<T> {
    #[inline]
    fn from(inner: Arc<Inner<T>>) -> Self {
        trace!("From<Arc<Inner<T>>>");
        Self {
            current: inner.first_node(),
            inner,
            index: 0,
        }
    }
}

impl<T> Iter<T> {
    /// Returns reference to last element in list
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let vs = vs![2, 3, 4];
    /// let iter = vs.iter();
    /// assert_eq!(iter.last_node(), Some(&4));
    /// ```
    #[inline]
    pub fn last_node(&self) -> Option<&T> {
        trace!("last_node()");
        self.inner
            .last_node()
            .map(|nn| unsafe { (*nn.as_ptr()).value() })
    }

    /// Returns current iterator size (may grow, but not decrease, be careful with race-conditions)
    ///
    /// If `Iter` was originally empty or was already consumed it will not grow (`FusedIterator`)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let vs = vs![3];
    /// let iter = vs.iter();
    /// assert_eq!(iter.len(), 1);
    ///
    /// vs.append(2);
    /// vs.clear();
    /// // Iterator is not cleared and will grow with original `VS`
    /// assert_eq!(iter.len(), 2);
    ///
    /// let mut iter2 = &mut vs.iter();
    /// assert_eq!(iter2.next(), None);
    /// assert_eq!(iter2.len(), 0);
    ///
    /// let iter = vs.iter();
    /// vs.append(2);
    /// // Iterator is fused
    /// assert_eq!(iter.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        trace!("len()");
        self.current.map_or(self.index, |_| self.inner.len())
    }

    /// Checks if iterator's length is empty (will return `None` on `next`)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let vs = vs![3];
    ///
    /// let mut iter = vs.iter();
    /// assert!(!iter.is_empty());
    /// vs.clear();
    /// // Iterator isn't cleared with `VS` is
    /// assert!(!iter.is_empty());
    ///
    /// // Consumes iterator to make it empty
    /// let _ = iter.count();
    /// assert!(iter.is_empty());
    ///
    /// // Iterator is fused
    /// let iter = vs.iter();
    /// assert!(iter.is_empty());
    /// vs.append(2);
    /// assert!(iter.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        trace!("is_empty()");
        self.current.map_or(true, |_| self.len() == 0)
    }

    /// Obtains current iterator index
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let vs = vs![3, 4];
    /// let mut iter = &mut vs.iter();
    ///
    /// assert_eq!(iter.next(), Some(&3));
    /// assert_eq!(iter.index(), 1);
    /// assert_eq!(iter.next(), Some(&4));
    /// assert_eq!(iter.index(), 2);
    ///
    /// // Index doesn't grow after iterator is consumed
    /// assert!(iter.next().is_none());
    /// assert_eq!(iter.index(), 2);
    /// ```
    #[inline]
    pub fn index(&self) -> usize {
        trace!("index() = {}", self.index);
        self.index
    }
}

impl<'a, T> Iterator for &'a mut Iter<T> {
    type Item = &'a T;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        trace!("next()");

        let data = if let Some(ptr) = self.current {
            self.index += 1;
            Some(unsafe { (*ptr.as_ptr()).value() })
        } else {
            None
        };

        debug!("{} at {} of {}", data.is_some(), self.index, self.len());
        debug_assert!(
            self.is_empty() && self.index == 0 && data.is_none() || self.inner.len() != 0
        );
        debug_assert!((self.index <= self.len() && data.is_some()) || self.index >= self.len());
        debug_assert!((self.index > self.len() && data.is_none()) || self.index <= self.len());

        self.current = self
            .current
            .and_then(|n| unsafe { (*n.as_ptr()).next() })
            .and_then(|n| NonNull::new(n as *const Node<T> as *mut Node<T>));
        data
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        trace!("size_hint()");
        (self.index, Some(self.len()))
    }
}

impl<'a, T> FusedIterator for &'a mut Iter<T> {}

#[cfg(test)]
mod tests {
    use crate::{setup_logger, voluntary_servitude::VS};

    #[test]
    fn iter_all() {
        setup_logger();
        let vs = vs![1, 2, 3];
        let mut iter = &mut vs.iter();
        assert_eq!(iter.last_node(), Some(&3));
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
            })
            .count();
        assert_eq!(iter.index(), iter.len());

        vs.clear();
        assert_eq!(vs.iter().last_node(), None);
        assert_eq!(vs.len(), 0);
        assert_eq!(iter.len(), 4);
        assert_eq!(iter.last_node(), Some(&4));
        let iter = &mut vs.iter();
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn iter_isnt_growable_when_consumed() {
        setup_logger();
        let vs: VS<()> = voluntary_servitude![];
        let mut iter = &mut vs.iter();
        vs.append(());
        assert!(iter.is_empty());
        assert!(iter.next().is_none());

        let vs: VS<()> = voluntary_servitude![()];
        vs.append(());
        let mut iter = &mut vs.iter();
        assert_eq!(iter.next(), Some(&()));
        assert_eq!(iter.next(), Some(&()));
        vs.append(());
        assert!(iter.next().is_none());
    }

    #[test]
    fn iter_doesnt_clear() {
        setup_logger();
        let vs = voluntary_servitude![()];
        let mut iter = &mut vs.iter();

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
        let iter = &mut vs.iter();
        let iter2 = &mut vs.iter();
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3]);

        vs.append(4);
        assert_eq!(iter2.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
        let iter = &mut vs.iter();
        assert_eq!(iter.collect::<Vec<_>>(), vec![&1, &2, &3, &4]);
    }

    #[test]
    fn iter_many() {
        setup_logger();
        let vs = vs![1, 2, 3, 4, 5];
        let mut iter = &mut vs.iter();
        let iter1 = &mut vs.iter();
        let iter2 = &mut vs.iter();
        assert_eq!(iter2.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        let iter3 = &mut vs.iter();
        assert_eq!(iter1.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter3.collect::<Vec<&i32>>(), vec![&1, &2, &3, &4, &5]);
        assert_eq!(iter.collect::<Vec<&i32>>(), vec![&2, &3, &4, &5]);
    }

    #[test]
    fn iter_after_use() {
        setup_logger();
        let vs = vs![1];
        let mut iter = &mut vs.iter();
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

        let mut iter = &mut vs.iter();
        assert_eq!(iter.next(), Some(&1));
        drop(iter);

        let mut iter = &mut vs.iter();
        while iter.next().is_some() {}
        drop(iter);
    }

    #[test]
    fn iter_drop_many() {
        setup_logger();
        let vs = vs![1, 2, 3, 4, 5];
        let iter = &mut vs.iter();
        let mut iter1 = &mut vs.iter();
        let mut iter2 = &mut vs.iter();
        assert_eq!(iter2.next(), Some(&1));
        assert_eq!(iter2.next(), Some(&2));
        let mut iter3 = &mut vs.iter();
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
