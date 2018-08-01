use std::{
    fmt::Debug, sync::atomic::{AtomicUsize, Ordering},
};
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
        trace!("Next element in {:?}", self);

        let data = self.current
            .as_ref()
            .map(|vs| unsafe { &(&*vs.cell.get()).value });
        debug!("Element: {:?}", data);

        let ended = self.current_index >= self.size;
        always!(ended || data.is_some(), "data = {:?}", self);
        always!(ended || self.current.is_some(), "self.current = {:?}", self);

        trace!("Increasing 1 in self.current_index");
        self.current_index += 1;
        self.current = self.current
            .take()
            .filter(|_| self.current_index < self.size)
            .map(|vs| unsafe { (&*vs.cell.get()).next.cell.get() })
            .and_then(|node| unsafe { (&*node).as_ref().cloned() });
        data
    }
}
