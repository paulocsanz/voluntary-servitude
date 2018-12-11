//! Thread-safe appendable list that can create a lock-free iterator

use parking_lot::RwLock;
use std::fmt::{self, Debug, Formatter};
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::{iter::Extend, iter::FromIterator, ptr::null_mut, ptr::NonNull, sync::Arc, mem::swap};
use {node::Node, FillOnceAtomicOption, IntoPtr, Iter, NotEmpty};

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

unsafe impl<T: Sync> Sync for Inner<T> {}
unsafe impl<T: Send> Send for Inner<T> {}

impl<T> Default for Inner<T> {
    #[inline]
    fn default() -> Self {
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
        let nn = NonNull::new(self.first_node.get_raw(Ordering::SeqCst));
        trace!("first_node() = {:?}", nn);
        nn
    }

    /// Atomically extracts pointer to last node
    #[inline]
    pub fn last_node(&self) -> Option<NonNull<Node<T>>> {
        let nn = NonNull::new(self.last_node.load(Ordering::SeqCst));
        trace!("last_node() = {:?}", nn);
        nn
    }

    /// Atomically extracts `Inner`'s size
    #[inline]
    pub fn len(&self) -> usize {
        let len = self.size.load(Ordering::SeqCst);
        trace!("len() = {}", len);
        len
    }

    /// Atomically checks if `Inner`'s size is `0`
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Set first node in chain
    #[inline]
    fn set_first(&self, node: Box<Node<T>>) -> Result<(), NotEmpty> {
        trace!("set_first({:p})", node);
        let ret = self.first_node.try_store(node, Ordering::SeqCst);
        debug_assert!(ret.is_ok());
        ret
    }

    /// Swaps last node, returning old one
    #[inline]
    fn swap_last(&self, ptr: *mut Node<T>) -> Option<NonNull<Node<T>>> {
        trace!("swap_last({:p})", ptr);
        NonNull::new(self.last_node.swap(ptr, Ordering::SeqCst))
    }

    /// Unsafelly append a `Node<T>` chain to `Inner<T>`
    #[inline]
    pub unsafe fn append_chain(&self, first: *mut Node<T>, last: *mut Node<T>, length: usize) {
        debug!("append_chain({:p}, {:p}, {})", first, last, length);
        let _ = self
            .swap_last(last)
            .or_else(|| self.set_first(Box::from_raw(first)).ok().and(None))
            .map(|nn| nn.as_ref().set_next(Box::from_raw(first)));

        info!("Increased size by {}", length);
        let _ = self.size.fetch_add(length, Ordering::SeqCst);
    }

    /// Appends node to end of `Inner` (inserts first_node if it's the first)
    #[inline]
    pub fn append(&self, value: T) {
        let ptr = Node::new(value).into_ptr();
        unsafe { self.append_chain(ptr, ptr, 1) };
    }

    #[inline]
    /// Extracts chain and drops itself without dropping it
    pub fn into_inner(mut self) -> (usize, *mut Node<T>, *mut Node<T>) {
        trace!("into_inner()");
        let size = self.size.swap(0, Ordering::SeqCst);
        let first = unsafe { self.first_node.dangle().into_ptr() };
        let last = self.last_node.swap(null_mut(), Ordering::SeqCst);
        (size, first, last)
    }
}

impl<T> FromIterator<T> for Inner<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        trace!("FromIterator<T>");
        let inner = Self::default();
        let _ = iter.into_iter().map(|v| inner.append(v)).count();
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
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// let (a, b, c) = (0usize, 1usize, 2usize);
/// // VS alias to VoluntaryServitude
/// // vs! alias to voluntary_servitude! (and operate like vec!)
/// let list = vs![a, b, c];
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&a, &b, &c]);
///
/// // Current VS's length
/// // Be careful with race conditions since the value, when used, may not be true anymore
/// assert_eq!(list.len(), 3);
///
/// // The 'iter' method makes a one-time lock-free iterator (Iter)
/// for (index, element) in list.iter().enumerate() {
///     assert_eq!(index, *element);
/// }
///
/// // You can get the current iteration index
/// // iter.next() == iter.len() means iteration ended (iter.next() == None)
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
/// #[macro_use]
/// extern crate voluntary_servitude;
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
///     println!("Multi-thread rust example ended without errors");
/// }
/// ```
pub struct VoluntaryServitude<T>(RwLock<Arc<Inner<T>>>);

/// [`VoluntaryServitude`]'s alias
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub type VS<T> = VoluntaryServitude<T>;

impl<T> VoluntaryServitude<T> {
    /// Empties list, returning its Inner if it's the last one
    ///
    /// It's used to manually drop each element (like in FFI)
    #[inline]
    pub fn try_unwrap(&self) -> Option<Inner<T>> {
        let mut write = self.0.write();
        let old = write.clone();
        *write = Arc::new(Inner::default());
        Arc::try_unwrap(old).ok()
    }

    /// Creates new `VoluntaryServitude` from [`Inner`]
    ///
    /// [`Inner`]: ./struct.Inner.html
    #[inline]
    pub(crate) fn new(inner: Inner<T>) -> Self {
        trace!("new()");
        VoluntaryServitude(RwLock::new(Arc::new(inner)))
    }

    /// Returns current size, be careful with race conditions when using it
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
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

    /// Checks if `VS` is currently empty, be careful with race conditions when using it
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![];
    /// assert!(list.is_empty());
    /// list.append(());
    /// assert!(!list.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.read().is_empty()
    }

    /// Inserts element after last node
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![];
    /// let mut iter = list.iter();
    /// list.append(3);
    /// assert!(iter.is_empty());
    /// iter = list.iter();
    /// list.append(8);
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
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![3, 2];
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// for (index, element) in list.iter().enumerate() {
    ///     assert_eq!(*element, [3, 2][index]);
    /// }
    /// ```
    #[inline]
    pub fn iter(&self) -> Iter<T> {
        Iter::new(self.0.read().clone())
    }

    /// Clears list (iterators referencing old chain will still work)
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
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

    /// Clears list returning iterator to it
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![3, 2];
    /// let iter = list.empty();
    /// assert_eq!(iter.len(), 2);
    /// assert_eq!(list.len(), 0);
    /// assert_eq!(list.iter().len(), 0);
    /// ```
    #[inline]
    pub fn empty(&self) -> Iter<T> {
        debug!("empty()");
        let mut write = self.0.write();
        let old = write.clone();
        *write = Arc::new(Inner::default());
        Iter::new(old)
    }

    /// Replaces `VS`
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![3, 2];
    /// let list2 = vs![5, 4];
    /// list.swap(&list2);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&5, &4]);
    /// assert_eq!(list2.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// ```
    #[inline]
    pub fn swap(&self, other: &Self) {
        debug!("empty()");
        swap(&mut *self.0.write(), &mut *other.0.write());
    }

    /// Extends `VS` like the `Extend` trait, but without needing a mutable reference
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![1, 2, 3];
    /// list.extend(vec![4, 5, 6]);
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);
    ///
    /// let list = vs![1, 2, 3];
    /// list.extend(vs![4, 5, 6].iter().cloned());
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);
    ///
    /// let list = vs![1, 2, 3];
    /// list.extend(vec![&4, &5, &6].into_iter().cloned());
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&1, &2, &3, &4, &5, &6]);
    /// ```
    #[inline]
    pub fn extend<I: IntoIterator<Item = T>>(&self, iter: I) {
        trace!("extend()");
        let (size, first, last) = Inner::from_iter(iter).into_inner();
        unsafe { self.0.read().append_chain(first, last, size) };
    }
}

impl<T> Default for VoluntaryServitude<T> {
    #[inline]
    fn default() -> Self {
        Self::new(Inner::default())
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
        Self::new(Inner::from_iter(iter))
    }
}

impl<'a, T: 'a + Copy> FromIterator<&'a T> for VoluntaryServitude<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        Self::from_iter(iter.into_iter().cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::drop;

    fn setup_logger() {
        #[cfg(feature = "logs")]
        ::setup_logger();
    }

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
    fn test_send() {
        fn assert_send<T>() {}
        assert_send::<VoluntaryServitude<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VoluntaryServitude<()>>();
    }
}
