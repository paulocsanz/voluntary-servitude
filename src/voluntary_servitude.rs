//! Lock-free appendable list

use iterator::{PrivateConstructor, VSIter};
use node::Node;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};
use std::{fmt, fmt::Debug, fmt::Formatter, iter::FromIterator, ptr::null_mut};
use std::{hash::Hash, hash::Hasher, mem::drop, ptr::NonNull};

/// Holds actual [`VoluntaryServitude`]'s data, abstracts safety
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub struct VSInner<T> {
    /// Number of elements inside `VSInner`
    size: AtomicUsize,
    /// Atomic references counter
    copies: AtomicUsize,
    /// First node in `VSInner`
    first_node: AtomicPtr<Node<T>>,
    /// Last node in `VSInner`
    last_node: AtomicPtr<Node<T>>,
}

impl<T> Default for VSInner<T> {
    #[inline]
    fn default() -> Self {
        trace!("Default VSInner");
        Self {
            size: AtomicUsize::new(0),
            copies: AtomicUsize::new(1),
            first_node: AtomicPtr::new(null_mut()),
            last_node: AtomicPtr::new(null_mut()),
        }
    }
}

impl<T> VSInner<T> {
    #[inline]
    /// Increases references count and returns pointer to self
    pub fn create_ref(&mut self) -> *mut Self {
        info!("Cloning VSInner, increasing references count");
        let _ = self.copies.fetch_add(1, Ordering::SeqCst);
        self as *mut _
    }

    /// Decreases references count (drop all nodes if it's the last reference)
    pub fn drop_ref(&self) {
        info!("Decreasing references counter, drop all nodes if it's the last reference");
        if self.copies.fetch_sub(1, Ordering::SeqCst) == 1 {
            debug!("Last reference, dropping nodes");
            let first = NonNull::new(self.first_node.swap(null_mut(), Ordering::SeqCst));
            let _ = first.map(|nn| unsafe { drop(Box::from_raw(nn.as_ptr())) });
        }
    }

    #[inline]
    /// Atomically extracts pointer to first node
    pub fn first_node(&self) -> *mut Node<T> {
        trace!("First Node in VSInner");
        self.first_node.load(Ordering::SeqCst)
    }

    #[inline]
    /// Atomically extracts `VSInner`'s size
    pub fn len(&self) -> usize {
        trace!("VSInner length");
        self.size.load(Ordering::SeqCst)
    }

    #[inline]
    /// Atomically checks if `VSInner`'s size is 0
    pub fn is_empty(&self) -> bool {
        trace!("VSInner is empty");
        self.len() == 0
    }

    /// Appends node to end of `VSInner` (inserts first_node if it's the first)
    pub fn append(&self, value: T) {
        trace!("Append to VSInner");
        let ptr = Box::into_raw(Box::new(Node::new(value)));
        if let Some(nn) = NonNull::new(self.last_node.swap(ptr, Ordering::SeqCst)) {
            debug!("Adding element to the end of the list");
            let before = unsafe { nn.as_ref().swap_next(ptr) };
            debug_assert!(before.is_none(), "First node wasn't actually first");
        } else {
            debug!("First element to be added");
            let before = self.first_node.swap(ptr, Ordering::SeqCst).is_null();
            debug_assert!(
                self.is_empty() || before,
                "Last node wasn't empty but should"
            );
        }

        trace!("Increased size");
        let _ = self.size.fetch_add(1, Ordering::SeqCst);
    }
}

/// Default Debug is recursive and causes a stackoverflow easily
impl<T: Debug> Debug for VSInner<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug VSInner");
        unsafe {
            let first_node = self.first_node.load(Ordering::SeqCst);
            let first_node = NonNull::new(first_node).map(|nn| &*nn.as_ptr());

            let last_node = self.last_node.load(Ordering::SeqCst);
            let last_node = NonNull::new(last_node).map(|nn| &*nn.as_ptr());
            write!(
                f,
                "VoluntaryServitude {{ size: {:?}, copies: {:?}, first_node: {:?}, last_node: {:?} }}",
                self.size.load(Ordering::SeqCst),
                self.copies.load(Ordering::SeqCst),
                first_node,
                last_node
            )
        }
    }
}

/// Lock-free appendable list (also called [`VS`])
///
/// Parallel examples in main lib docs
///
/// [`VS`]: ./type.VS.html
///
/// ```rust
/// # #[macro_use] extern crate voluntary_servitude;
/// # use voluntary_servitude::VS;
/// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
/// let list = vs![3, 2];
/// assert_eq!(list.len(), 2);
/// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
///
/// list.clear();
/// assert!(list.is_empty());
///
/// // You can deep copy a `VS` if T is Copy
/// use std::iter::FromIterator;
/// let list2 = VS::from_iter(list.iter());
/// list2.append(3);
/// assert_eq!(list.len() + 1, list2.len());
///
/// for el in list.iter() {
///     assert_eq!(el, &3);
/// }
/// ```
pub struct VoluntaryServitude<T> {
    /// Atomic reference to VSInner
    inner: AtomicPtr<VSInner<T>>,
}

/// [`VoluntaryServitude`]'s alias
///
/// [`VoluntaryServitude`]: ./struct.VoluntaryServitude.html
pub type VS<T> = VoluntaryServitude<T>;

#[cfg(feature = "serde-traits")]
use serde_lib::{Deserialize, Deserializer};

#[cfg(feature = "serde-traits")]
impl<'a, T: 'a + Deserialize<'a>> Deserialize<'a> for VoluntaryServitude<T> {
    fn deserialize<D: Deserializer<'a>>(des: D) -> Result<Self, D::Error> {
        trace!("Deserialize VoluntaryServitude");
        VSInner::deserialize(des).map(|inner| Self {
            inner: AtomicPtr::new(Box::into_raw(Box::new(inner))),
        })
    }
}

impl<T> FromIterator<T> for VoluntaryServitude<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        debug!("VSInner<T> from IntoIterator<T>");
        let vs = vs![];
        let _ = iter.into_iter().map(|v| vs.append(v)).count();
        vs
    }
}

impl<'a, T: 'a + Copy> FromIterator<&'a T> for VoluntaryServitude<T> {
    #[inline]
    fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
        trace!("VoluntaryServitude<T> from IntoIterator<&'a T> where T: Copy");
        VS::from_iter(iter.into_iter().map(|v| *v))
    }
}

impl<T: Hash> Hash for VoluntaryServitude<T> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        trace!("Hash VoluntaryServitude");
        self.iter().hash(state);
    }
}

impl<T: PartialEq> PartialEq for VoluntaryServitude<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        trace!("PartialEq VoluntaryServitude");
        let len = self.len();
        if len == 0 && other.len() == 0 {
            return true;
        };
        self.iter()
            .zip(other.iter())
            .zip(0..len)
            .filter(|((a, b), _)| a == b)
            .count()
            > 0
    }
}
impl<T: Eq> Eq for VoluntaryServitude<T> {}

impl<T: Debug> Debug for VoluntaryServitude<T> {
    #[inline]
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        trace!("Debug VoluntaryServitude");
        write!(f, "VoluntaryServitude {{ inner: {:?} }}", self.inner())
    }
}

impl<T> Default for VoluntaryServitude<T> {
    #[inline]
    fn default() -> Self {
        trace!("Default VoluntaryServitude");
        Self {
            inner: AtomicPtr::new(Box::into_raw(Box::new(VSInner::default()))),
        }
    }
}

impl<T> Drop for VoluntaryServitude<T> {
    #[inline]
    fn drop(&mut self) {
        debug!("Drop VoluntaryServitude");
        unsafe { (*self.inner.load(Ordering::SeqCst)).drop_ref() };
    }
}

impl<T> VoluntaryServitude<T> {
    /// Extracts reference to VSInner from AtomicPtr
    #[inline]
    fn inner(&self) -> &VSInner<T> {
        trace!("VSInner from VoluntaryServitude");
        unsafe { &*self.inner.load(Ordering::SeqCst) }
    }

    /// Atomically extracts current size, be careful with data-races when using it
    ///
    /// May grow or be set to 0
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
        self.inner().len()
    }

    /// Atomically checks if [`VS`] is empty, be careful with data-races when using it
    ///
    /// [`VS`]: ./type.VS.html
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
        self.inner().is_empty()
    }

    /// Makes lock-free iterator based on VoluntaryServitude
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![3, 2];
    /// assert_eq!(list.iter().collect::<Vec<_>>(), vec![&3, &2]);
    /// ```
    #[inline]
    pub fn iter<'a>(&self) -> VSIter<'a, T> {
        debug!("Iter VoluntaryServitude");
        unsafe { VSIter::new(&mut *(*self.inner.load(Ordering::SeqCst)).create_ref()) }
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
    /// ```
    #[inline]
    pub fn clear(&self) {
        debug!("Clear VoluntaryServitude");

        let ptr = Box::into_raw(Box::new(VSInner::<T>::default()));
        unsafe { (*self.inner.swap(ptr, Ordering::SeqCst)).drop_ref() };
    }

    /// Insert element after last node
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
        self.inner().append(value);
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
    fn test_send() {
        fn assert_send<T: Send>() {}
        assert_send::<VoluntaryServitude<()>>();
    }

    #[test]
    fn test_sync() {
        fn assert_sync<T: Sync>() {}
        assert_sync::<VoluntaryServitude<()>>();
    }

    #[test]
    fn partial_eq() {
        assert_eq!(vs![1, 2, 3], vs![1, 2, 3]);
        let vs = vs![2, 3, 4];
        assert_eq!(&vs, &vs);
    }
}
