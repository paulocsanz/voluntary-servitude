//! Thread-safe appendable list that can create a lock-free iterator

use crate::{node::Node, prelude::*};
use parking_lot::RwLock;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::{iter::Extend, iter::FromIterator, mem::swap, ptr::null_mut, ptr::NonNull, sync::Arc};

/// Holds actual [`VoluntaryServitude`]'s data, abstracts safety
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
#[derive(Debug)]
pub struct Inner<T> {
    /// Number of elements inside `Inner`
    size: AtomicUsize,
    /// First node in `Inner`
    first_node: FillOnceAtomicOption<Node<T>>,
    /// Last node in `Inner`
    last_node: AtomicPtr<Node<T>>,
}

impl<T> Default for Inner<T> {
    #[inline]
    fn default() -> Self {
        trace!("default()");
        Self {
            size: AtomicUsize::new(0),
            first_node: FillOnceAtomicOption::default(),
            last_node: AtomicPtr::new(null_mut()),
        }
    }
}

impl<T> Inner<T> {
    /// Atomically extracts pointer to first node
    #[inline]
    pub fn first_node(&self) -> Option<NonNull<Node<T>>> {
        let nn = NonNull::new(self.first_node.get_raw(Ordering::Relaxed));
        trace!("first_node() = {:?}", nn);
        nn
    }

    /// Atomically extracts pointer to last node
    #[inline]
    pub fn last_node(&self) -> Option<NonNull<Node<T>>> {
        let nn = NonNull::new(self.last_node.load(Ordering::Relaxed));
        trace!("last_node() = {:?}", nn);
        nn
    }

    /// Atomically extracts `Inner`'s size
    #[inline]
    pub fn len(&self) -> usize {
        let len = self.size.load(Ordering::Relaxed);
        trace!("len() = {}", len);
        len
    }

    /// Atomically checks if `Inner`'s size is `0`
    #[inline]
    pub fn is_empty(&self) -> bool {
        trace!("is_empty()");
        self.len() == 0
    }

    /// Set first node in chain
    #[inline]
    fn set_first(&self, node: Box<Node<T>>) -> Result<(), NotEmpty> {
        trace!("set_first({:p})", node);
        let ret = self.first_node.try_store(node, Ordering::Relaxed);
        debug_assert!(ret.is_ok());
        ret
    }

    /// Swaps last node, returning old one
    #[inline]
    fn swap_last(&self, ptr: *mut Node<T>) -> Option<NonNull<Node<T>>> {
        trace!("swap_last({:p})", ptr);
        NonNull::new(self.last_node.swap(ptr, Ordering::Relaxed))
    }

    /// Unsafelly append a `Node<T>` chain to `Inner<T>`
    ///
    /// # Safety
    ///
    /// It's unsafe because we can't be sure of the ownership of `first` or `last`.
    ///
    /// To call this you must ensure the objects pointed by `first` and `last` are owned by no-one, so `Inner` will take its ownership.
    ///
    /// Nobody can use these pointers (without using `Inner`'s API) or drop them after calling this function
    ///
    /// (The objects pointed must exist while `Inner` exists and they can't be accessed after)
    #[inline]
    pub unsafe fn append_chain(&self, first: *mut Node<T>, last: *mut Node<T>, length: usize) {
        debug!("append_chain({:p}, {:p}, {})", first, last, length);
        if let Some(nn) = self.swap_last(last) {
            // To call `Box::from_raw` unsafe is needed
            // But since `Inner` owns what they point to, it can be sure they will exist while `Inner` does
            // (as long as `append_chain` was properly called)
            #[allow(unused)]
            let old = nn.as_ref().try_store_next(Box::from_raw(first));
            debug_assert!(old.is_ok());
        } else {
            // To call `Box::from_raw` you must make sure `Inner` now owns the `Node<T>`
            let _ = self.set_first(Box::from_raw(first));
        }

        info!("Increased size by {}", length);
        let _ = self.size.fetch_add(length, Ordering::Relaxed);
    }

    /// Appends node to end of `Inner` (inserts first_node if it's the first)
    #[inline]
    pub fn append(&self, value: T) {
        let ptr = Node::new(value).into_ptr();
        // We own `Node<T>` so we can pass its ownership to `append_chain`
        // And we don't drop it
        unsafe { self.append_chain(ptr, ptr, 1) };
    }

    #[inline]
    /// Extracts chain and drops itself without dropping it
    pub fn into_inner(self) -> (usize, *mut Node<T>, *mut Node<T>) {
        trace!("into_inner()");
        let size = self.size.into_inner();
        let first = self.first_node.into_inner().into_ptr();
        let last = self.last_node.into_inner();
        (size, first, last)
    }
}

impl<T> FromIterator<T> for Inner<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        trace!("FromIterator<T>");
        let inner = Self::default();
        for element in iter {
            inner.append(element);
        }
        inner
    }
}

/// Appendable list with lock-free iterator (also called [`VS`])
///
///
/// # Examples
///  - [`Single-thread`]
///  - [`Multi-producer, multi-consumer`]
///
/// [`Single-thread`]: #single-thread
/// [`Multi-producer, multi-consumer`]: #multi-producer-multi-consumer
/// [`VS`]: ./type.VS.html
///
/// # Single thread
///
/// ```rust
/// # #[macro_use] extern crate voluntary_servitude;
/// # env_logger::init();
/// let (a, b, c) = (0usize, 1usize, 2usize);
/// // VS alias to VoluntaryServitude
/// // vs! alias to voluntary_servitude! (and operates like vec!)
/// let list = vs![a, b, c];
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&a, &b, &c]);
///
/// // Current VS's length
/// // Be careful with race conditions since the value, when used, may not be true anymore
/// assert_eq!(list.len(), 3);
///
/// // The 'iter' method makes a lock-free iterator (Iter)
/// for (index, element) in list.iter().enumerate() {
///     assert_eq!(index, *element);
/// }
///
/// // You can get the current iteration index
/// // iter.index() == iter.len() means iteration ended (iter.next() == None)
/// let mut iter = &mut list.iter();
/// assert_eq!(iter.index(), 0);
/// assert_eq!(iter.next(), Some(&0));
/// assert_eq!(iter.index(), 1);
///
/// // List can also be cleared (but current iterators are not affected)
/// list.clear();
///
/// assert_eq!(iter.len(), 3);
/// assert_eq!(list.len(), 0);
/// assert_eq!(list.iter().len(), 0);
/// assert_eq!((&mut list.iter()).next(), None);
///
/// println!("Single thread example ended without errors");
/// ```
///
/// # Multi-producer, multi-consumer
///
/// ```rust
/// # #[macro_use] extern crate voluntary_servitude;
/// use std::{sync::Arc, thread::spawn};
///
/// const CONSUMERS: usize = 8;
/// const PRODUCERS: usize = 4;
/// const ELEMENTS: usize = 10_000_000;
///
/// fn main() {
///     let list = Arc::new(vs![]);
///     let mut handlers = vec![];
///
///     // Creates producer threads to insert 10k elements
///     for _ in 0..PRODUCERS {
///         let l = Arc::clone(&list);
///         handlers.push(spawn(move || {
///             let _ = (0..ELEMENTS).map(|i| l.append(i)).count();
///         }));
///     }
///
///     // Creates consumer threads to print number of elements
///     // Until all of them are inserted
///     for _ in 0..CONSUMERS {
///         const TOTAL: usize = PRODUCERS * ELEMENTS;
///         let consumer = Arc::clone(&list);
///         handlers.push(spawn(move || loop {
///             let count = consumer.iter().count();
///             println!("{} elements", count);
///             if count >= TOTAL { break };
///         }));
///     }
///
///     // Join threads
///     for handler in handlers.into_iter() {
///         handler.join().expect("Failed to join thread");
///     }
///
///     println!("Multi-thread example ended without errors");
/// }
/// ```
pub struct VoluntaryServitude<T>(RwLock<Arc<Inner<T>>>);

/// [`VoluntaryServitude`]'s alias
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub type VS<T> = VoluntaryServitude<T>;

impl<T> VoluntaryServitude<T> {
    /// Creates new empty `VS` (like `Default` trait)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # use voluntary_servitude::VS;
    /// # env_logger::init();
    /// let list: VS<()> = VS::new();
    /// assert!(list.is_empty());
    /// ```
    #[inline]
    pub fn new() -> Self {
        trace!("new()");
        Self::default()
    }

    /// Inserts element after last node
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![];
    /// let mut iter = list.iter();
    ///
    /// list.append(3);
    /// // Iter doesn't grow if it's empty (originally empty or was consumed)
    /// assert!(iter.is_empty());
    ///
    /// iter = list.iter();
    /// list.append(8);
    /// // Iter grows if it has not been consumed
    /// assert_eq!(iter.collect::<Vec<_>>(), vec![&3, &8]);
    /// ```
    #[inline]
    pub fn append(&self, value: T) {
        self.0.read().append(value);
    }

    /// Makes lock-free iterator based on `VS`
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![3, 2];
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
    ///
    /// for (element, expected) in list.iter().zip(&[3, 2][..]) {
    ///     assert_eq!(element, expected);
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        debug!("iter()");
        Iter::from(self.0.read().clone())
    }

    /// Returns current size, be careful with race conditions when using it since other threads can change it right after the read
    ///
    /// `Relaxed` ordering is used to extract the length, so you shouldn't depend on this being sequentially consistent, only atomic
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![3, 2];
    /// assert_eq!(list.len(), 2);
    /// list.append(5);
    /// assert_eq!(list.len(), 3);
    /// list.clear();
    /// assert_eq!(list.len(), 0);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.read().len()
    }

    /// Checks if `VS` is currently empty, be careful with race conditions when using it since other threads can change it right after the read
    ///
    /// `Relaxed` ordering is used to extract the length, so you shouldn't depend on this being sequentially consistent, only atomic
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![];
    /// assert!(list.is_empty());
    /// list.append(());
    /// assert!(!list.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.read().is_empty()
    }

    /// Clears list (iterators referencing the old chain will still work)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![3, 2];
    /// let iter = list.iter();
    /// list.clear();
    /// assert_eq!(iter.len(), 2);
    /// assert_eq!(list.len(), 0);
    /// assert_eq!(list.iter().len(), 0);
    /// ```
    #[inline]
    pub fn clear(&self) {
        debug!("clear()");
        *self.0.write() = Arc::new(Inner::default());
    }

    /// Clears list returning iterator to it (other iterators referencing the old chain will still work)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![3, 2];
    /// let iter = list.empty();
    /// assert_eq!(iter.len(), 2);
    /// assert_eq!(list.len(), 0);
    /// assert_eq!(list.iter().len(), 0);
    /// ```
    #[inline]
    pub fn empty(&self) -> Iter<T> {
        debug!("empty()");
        let old = Self::default();
        self.swap(&old);
        old.iter()
    }

    /// Swaps two `VS`
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![3, 2];
    /// let list2 = vs![5, 4];
    /// list.swap(&list2);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&5, &4]);
    /// assert_eq!(list2.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// ```
    #[inline]
    pub fn swap(&self, other: &Self) {
        debug!("swap({:p})", other);
        swap(&mut *self.0.write(), &mut *other.0.write());
    }

    /// Extends `VS` like the `Extend` trait, but without a mutable reference
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # env_logger::init();
    /// let list = vs![1, 2, 3];
    /// list.extend(vec![4, 5, 6]);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);
    ///
    /// // You can extend from another `VS` if you clone (or copy) each element
    /// let list = vs![1, 2, 3];
    /// list.extend(vs![4, 5, 6].iter().cloned());
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);
    /// # let list = vs![1, 2, 3];
    /// # list.extend(vec![&4, &5, &6].into_iter().cloned());
    /// # assert_eq!(list.iter().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);
    /// ```
    #[inline]
    pub fn extend<I: IntoIterator<Item = T>>(&self, iter: I) {
        trace!("extend()");
        let (size, first, last) = Inner::from_iter(iter).into_inner();
        // We own `Inner<T>` so we can pass its ownership of its nodes to `append_chain`
        // And we don't drop them
        unsafe { self.0.read().append_chain(first, last, size) };
    }
}

impl<T> Default for VoluntaryServitude<T> {
    #[inline]
    fn default() -> Self {
        trace!("default()");
        Self::from(Inner::default())
    }
}

impl<T: Debug> Debug for VoluntaryServitude<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("VoluntaryServitude")
            .field(&self.iter().collect::<Vec<_>>())
            .finish()
    }
}

impl<T> Extend<T> for VoluntaryServitude<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        Self::extend(self, iter)
    }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for VoluntaryServitude<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        Self::extend(self, iter.into_iter().cloned())
    }
}

impl<T> FromIterator<T> for VoluntaryServitude<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::from(Inner::from_iter(iter))
    }
}

impl<'a, T: 'a + Copy> FromIterator<&'a T> for VoluntaryServitude<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        Self::from_iter(iter.into_iter().cloned())
    }
}

impl<T> From<Inner<T>> for VoluntaryServitude<T> {
    #[inline]
    fn from(inner: Inner<T>) -> Self {
        trace!("From<Inner<T>>");
        VoluntaryServitude(RwLock::new(Arc::new(inner)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup_logger;
    use std::mem::drop;

    #[test]
    fn iter_outlives() {
        setup_logger();
        let vs = vs![1, 2, 3, 4];
        let iter = vs.iter();
        drop(vs);
        drop(iter);
    }

    #[test]
    fn voluntary_servitude_len_append_clear() {
        setup_logger();
        let list = vs![1, 2, 3];
        assert_eq!(list.len(), 3);
        list.append(4);
        assert_eq!(list.len(), 4);
        list.clear();
        assert!(list.is_empty());
        list.append(4);
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn extend_partial_eq() {
        setup_logger();
        let vs: VS<u8> = vs![1, 2, 3, 4, 5];
        let iter = &mut vs.iter();
        vs.extend(iter.cloned());
        assert_eq!(
            vs.iter().collect::<Vec<_>>(),
            vec![&1u8, &2, &3, &4, &5, &1, &2, &3, &4, &5]
        );
    }

    #[test]
    fn swap_empty() {
        let vs: VS<u8> = vs![1, 2, 3, 4, 5];
        let mut old: VS<u8> = vs![5, 4, 3, 2, 1];
        vs.swap(&mut old);
        assert_eq!(vs.empty().collect::<Vec<_>>(), vec![&5, &4, &3, &2, &1]);
        assert_eq!(old.empty().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5]);
        assert!(vs.is_empty());
    }

    #[test]
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VoluntaryServitude<()>>();
        assert_send::<Inner<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VoluntaryServitude<()>>();
        assert_sync::<Inner<()>>();
    }
}
