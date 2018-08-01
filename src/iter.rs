use std::{fmt::Debug,
          sync::{atomic::{AtomicUsize, Ordering},
                 Arc}};
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
        self.current = if self.current_index == self.size {
            info!("No more elements in VSReadIter");
            self.data = None;
            self.current.take()
        } else if let Some(current) = self.current.take() {
            let curr = unsafe &*current.cell.get()};
            if self.current_index == 0 {
                self.data = unsafe { Some(&(*curr).value) };
                self.current_index += 1;
                info!("First element in VSReadIter: {:?}", current);
                Some(current)
            } else {
                trace!("Found element: {:?}", current);
                let node = match unsafe { &(*(*curr).next.cell.get()) } {
                    Some(ref next) => {
                        self.current_index += 1;
                        info!("Found next node ({}): {:?}", self.current_index, next);
                        Arc::clone(next)
                    }
                    None => {
                        crit!("Expected node, but found None");
                        self.size = self.current_index;
                        self.data = None;
                        return self.data;
                    }
                };

                unsafe {
                    self.data = Some(&(*node.cell.get()).value);
                }

                Some(node)
            }
        } else {
            crit!("self.current is None but it shouldn't: {:?}", self);
            self.size = self.current_index;
            self.data = None;
            return self.data;
        };

        trace!("Element: {:?}", self.data);
        self.data
    }
}
