#[macro_use]
extern crate voluntary_servitude;

fn main() {
    // Create VSRead with 3 elements
    // vsread![] makes an empty VSRead
    // vsread![1; 3] makes a VSRead with 3 elements with 1 as value
    let list = vsread![0, 1, 2];
    assert_eq!(list.len(), 3);

    // The 'iter' method makes a one-time lock-free iterator (VSReadIter) based on VSRead
    assert_eq!(list.iter().len(), 3);

    // Appends 9997 elements to it
    assert_eq!((3..10000).map(|i| list.append(i)).count(), 9997);
    assert_eq!(list.len(), 10000);
    assert_eq!(list.iter().len(), 10000);
    assert_eq!(list.iter().count(), 10000);

    let mut iter = list.iter();
    let mut index = 0;
    loop {
        match iter.next() {
            Some(_) => index += 1,
            None => break,
        };
        // You can get the current iteration index (can be compared with the length 'len')
        assert_eq!(iter.index(), index);
    }

    // Iterates through all elements to ensure it's what we inserted
    let count = list
        .iter()
        .enumerate()
        .map(|(i, el)| assert_eq!(&i, el))
        .count();
    assert_eq!(count, 10000);

    // Inserts 10k more elements and ensure we iterate through them
    assert_eq!((0..10000).map(|i| list.append(i)).count(), 10000);
    let count = list
        .iter()
        .enumerate()
        .map(|(i, el)| assert_eq!(&(i % 10000), el))
        .count();
    assert_eq!(count, 20000);

    // List can also be cleared
    list.clear();
    assert_eq!(list.len(), 0);
    assert_eq!(list.iter().len(), 0);
    assert_eq!(list.iter().count(), 0);

    const ELEMENTS: usize = 10000;
    // Creates VSRead with 3 elements
    // vsread![] and VSRead::default() make an empty VSRead
    // vsread![1; 3] makes a VSRead with 3 elements equal to 1
    let list = vsread![0, 1, 2];

    // Current VSRead length
    // Be careful with data-races since the value, when used, may not be true anymore
    assert_eq!(list.len(), 3);

    // The 'iter' method makes a one-time lock-free iterator (VSReadIter) based on VSRead
    assert_eq!(list.iter().len(), 3);

    // You can get the current iteration index
    // (if iter.index() is equal to iter.len(), then the iteration ended - iter.next() is None)
    let mut iter = list.iter();
    assert_eq!(iter.index(), 0);
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.index(), 1);

    // Appends 9997 elements to it
    assert_eq!((3..ELEMENTS).map(|i| list.append(i)).count(), ELEMENTS - 3);

    // Iterates through all elements to ensure it's what we inserted
    let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
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

    println!("Test ended without errors");
}
