#[macro_use]
extern crate voluntary_servitude;

const ELEMENTS: usize = 10000;

fn main() {
    // VS alias to VoluntaryServitude
    // vs! alias to voluntary_servitude! (and operate like vec!)
    let list = vs![0, 1, 2];

    // Current VS's length
    // Be careful with data-races since the value, when used, may not be true anymore
    assert_eq!(list.len(), 3);

    // The 'iter' method makes a one-time lock-free iterator (VSIter)
    assert_eq!(list.iter().len(), 3);

    // You can get the current iteration index
    // iter.next() == iter.le() means iteration ended (iter.next() == None)
    let mut iter = list.iter();
    assert_eq!(iter.index(), 0);
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.index(), 1);

    // Appends 9997 elements to it
    assert_eq!((3..ELEMENTS).map(|i| list.append(i)).count(), ELEMENTS - 3);

    // Iterates through all elements to ensure it's what we inserted
    let count = list
        .iter()
        .enumerate()
        .map(|(i, el)| assert_eq!(&i, el))
        .count();
    assert_eq!(count, ELEMENTS);

    let iter2 = list.iter();

    // List can also be cleared (but current iterators are not affected)
    list.clear();

    assert_eq!(list.len(), 0);
    assert_eq!(list.iter().len(), 0);
    assert_eq!(list.iter().next(), None);
    assert_eq!(iter2.len(), ELEMENTS);
    let count = iter2.enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
    assert_eq!(count, ELEMENTS);

    println!("Single thread example ended without errors");
}
