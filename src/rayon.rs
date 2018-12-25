//! Integration with rayon

use rayon_lib::prelude::*;
use {VoluntaryServitude, VS};

impl<T: Send + Sync> VoluntaryServitude<T> {
    /// Parallely Extends [`VS`] like the `ParallelExtend` trait, but without a mutable reference
    ///
    /// [`VS`]: ./type.VS.html
    ///
    /// ```rust
    /// # #[macro_use] extern crate voluntary_servitude;
    /// # #[cfg(feature = "logs")] voluntary_servitude::setup_logger();
    /// let list = vs![1, 2, 3];
    /// list.par_extend(vec![4, 5, 6]);
    /// assert_eq!(list.iter().sum::<i32>(), 21);
    /// ```
    #[cfg(feature = "rayon-traits")]
    #[cfg_attr(docs_rs_workaround, doc(cfg(feature = "rayon-traits")))]
    #[inline]
    pub fn par_extend<I>(&self, par_iter: I)
    where
        I: IntoParallelIterator<Item = T>
    {
        trace!("par_extend()");
        par_iter.into_par_iter().for_each(|el| self.append(el));
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "rayon-traits")))]
impl<T: Send + Sync> FromParallelIterator<T> for VoluntaryServitude<T> {
    #[inline]
    fn from_par_iter<I: IntoParallelIterator<Item = T>>(par_iter: I) -> Self {
        trace!("from_par_iter()");
        let vs = vs![];
        par_iter.into_par_iter().for_each(|el| vs.append(el));
        vs
    }
}

#[cfg_attr(docs_rs_workaround, doc(cfg(feature = "rayon-traits")))]
impl<T: Send + Sync> ParallelExtend<T> for VoluntaryServitude<T> {
    #[inline]
    fn par_extend<I: IntoParallelIterator<Item = T>>(&mut self, par_iter: I) {
        trace!("ParExtend");
        VS::par_extend(self, par_iter);
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
    fn from_par_iter() {
        setup_logger();
        let vec = vec![1, 2, 3, 4, 5, 6];
        let sum: u8 = vec.iter().sum();
        let vs = VS::from_par_iter(vec);
        assert_eq!(vs.iter().sum::<u8>(), sum);
    }
}
