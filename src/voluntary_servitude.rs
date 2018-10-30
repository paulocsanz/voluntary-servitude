//! Thread-safe appendable list that can create a lock-free iterator

use crossbeam::sync::ArcCell;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::sync::{atomic::AtomicPtr, atomic::AtomicUsize, atomic::Ordering, Arc};
use std::{iter::Extend, iter::FromIterator, mem::drop, ptr::NonNull};
use {node::Node, FillOnceAtomicOption, Filled, IntoPtr, Iter, NotEmpty};

#[cfg(feature = "serde-traits")]
use serde_lib::{Deserialize, Deserializer};

#[cfg(feature = "rayon-traits")]
use rayon_lib::prelude::*;

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
        Self {
            size: AtomicUsize::default(),
            first_node: FillOnceAtomicOption::default(),
            last_node: AtomicPtr::default(),
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

    /// Tries to insert first element
    #[inline]
    fn start(&self, boxed: Box<Node<T>>) -> Result<(), NotEmpty> {
        trace!("start({:p})", boxed);
        self.first_node.try_store(boxed, Ordering::SeqCst)
    }

    /// Swaps last node, returning old one
    #[inline]
    fn swap_last(&self, ptr: *mut Node<T>) -> Option<NonNull<Node<T>>> {
        trace!("swap_last({:p})", ptr);
        NonNull::new(self.last_node.swap(ptr, Ordering::SeqCst))
    }

    #[inline]
    /// Unsafelly append a `Node<T>` chain to `Inner<T>`
    pub unsafe fn append_chain(&self, first: *mut Node<T>, last: *mut Node<T>, length: usize) {
        debug!("append_chain({:p}, {:p}, {})", first, last, length);
        let _ = self
            .swap_last(last)
            .or_else(|| self.start(Box::from_raw(first)).filled_default("First"))
            .map(|nn| nn.as_ref().set_next(Box::from_raw(first)).filled("Last"));

        info!("Increased size by {}", length);
        let _ = self.size.fetch_add(length, Ordering::SeqCst);
    }

    /// Appends node to end of `Inner` (inserts first_node if it's the first)
    #[inline]
    pub fn append(&self, value: T) {
        let ptr = Box::into_raw(Box::new(Node::new(value)));
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
/// let mut iter = list.iter();
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
/// assert_eq!(list.iter().next(), None);
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
/// const ELEMENTS: usize = 10000000;
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
pub struct VoluntaryServitude<T>(ArcCell<Inner<T>>);

/// [`VoluntaryServitude`]'s alias
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub type VS<T> = VoluntaryServitude<T>;

impl<T> VoluntaryServitude<T> {
    /// Creates new `VoluntaryServitude` from [`Inner`]
    ///
    /// [`Inner`]: ./struct.Inner.html
    #[inline]
    fn new(inner: Inner<T>) -> Self {
        trace!("new()");
        VoluntaryServitude(ArcCell::new(Arc::new(inner)))
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
        self.0.get().len()
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
        self.0.get().is_empty()
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
        self.0.get().append(value);
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
        Iter::new(self.0.get())
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
        drop(self.0.set(Arc::new(Inner::default())));
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
        unsafe { self.0.get().append_chain(first, last, size) };
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
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.debug_struct("VoluntaryServitude")
            .field("arc_cell", &self.0.get())
            .finish()
    }
}

#[cfg(feature = "serde-traits")]
impl<'a, T: 'a + Deserialize<'a>> Deserialize<'a> for VoluntaryServitude<T> {
    #[inline]
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        Inner::deserialize(des).map(Self::new)
    }
}

#[cfg(feature = "rayon-traits")]
impl<T: Send + Sync> FromParallelIterator<T> for VoluntaryServitude<T> {
    #[inline]
    fn from_par_iter<I: IntoParallelIterator<Item = T>>(par_iter: I) -> Self {
        trace!("from_par_iter()");
        let vs = vs![];
        par_iter.into_par_iter().for_each(|el| vs.append(el));
        vs
    }
}

#[cfg(feature = "rayon-traits")]
impl<T: Send + Sync> ParallelExtend<T> for VoluntaryServitude<T> {
    #[inline]
    fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, par_iter: I) {
        trace!("ParExtend");
        VS::par_extend(self, par_iter);
    }
}

impl<T> Extend<T> for VoluntaryServitude<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        VS::extend(self, iter)
    }
}

impl<'a, T: 'a + Copy> Extend<&'a T> for VoluntaryServitude<T> {
    #[inline]
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, iter: I) {
        VS::extend(self, iter.into_iter().cloned())
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

impl<T: Send + Sync> VoluntaryServitude<T> {
    /// Parallely Extends [`VS`] like the ParallelExtend trait, but without a mutable reference
    ///
    /// [`VS`]: ./type.VS.html
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![1u8, 2, 3];
    /// list.par_extend(vec![4, 5, 6]);
    /// assert_eq!(list.iter().sum::<u8>(), 21u8);
    /// ```
    #[cfg(feature = "rayon-traits")]
    #[cfg_attr(docs_rs_workaround, doc(cfg(feature = "rayon-traits")))]
    #[inline]
    pub fn par_extend<I: IntoParallelIterator<Item = T>>(&self, par_iter: I) {
        trace!("par_extend()");
        par_iter.into_par_iter().for_each(|el| self.append(el));
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
        let vs: VS<u8> = vs![1, 2, 3, 4, 5];
        let iter = vs.iter();
        vs.extend(iter.into_iter().cloned());
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

    #[cfg(feature = "rayon-traits")]
    #[test]
    fn from_par_iter() {
        let vec = vec![1, 2, 3, 4, 5, 6];
        let sum: u8 = vec.iter().sum();
        let vs = VS::from_par_iter(vec);
        assert_eq!(vs.iter().sum::<u8>(), sum);
    }
}
