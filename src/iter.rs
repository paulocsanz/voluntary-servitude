use std::{fmt::Debug,
          sync::atomic::{AtomicUsize, Ordering}};
use types::*;

#[derive(Debug)]
pub struct VSReadIter<'a, T: 'a + Debug> {
    current: Option<ArcNode<T>>,
    current_index: usize,
    size: usize,
    data: Option<&'a T>,
}

impl<'a, T: 'a + Debug> VSReadIter<'a, T> {
    pub fn new(current: &Option<ArcNode<T>>, size: &AtomicUsize) -> VSReadIter<'a, T> {
        trace!("VSReadIter start node: {:?}", current);
        // Get size before to ensure it's always lower or equal to current (no data race)
        let size = size.load(Ordering::Relaxed);
        VSReadIter {
            size,
            current: current.as_ref().cloned(),
            current_index: 0,
            data: None,
        }
    }
}

impl<'a, T: 'a + Debug> Iterator for VSReadIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        debug!("Next element in {:?}", self);
        if self.current_index < self.size {
            if self.current_index > 0 {
                self.current = self.current.as_ref().cloned().and_then(|vs| unsafe {
                    (&*(&*vs.cell.get()).next.cell.get()).as_ref().cloned()
                });
                debug_assert!(self.current.is_some());
            }
            let data = self.current
                .as_ref()
                .map(|vs| unsafe { &(&*vs.cell.get()).value });
            debug_assert!(data.is_some());

            trace!("Adding 1 to index: {}", self.current_index);
            self.current_index += 1;

            trace!("Element: {:?}", data);
            data
        } else {
            trace!("No more elements");
            None
        }
    }
}
