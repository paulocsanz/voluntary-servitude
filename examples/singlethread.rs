#[macro_use]
extern crate voluntary_servitude;

fn main() {
    let (a, b, c) = (0usize, 1usize, 2usize);
    // VS alias to VoluntaryServitude
    // vs! alias to voluntary_servitude! (and operates like vec!)
    let list = vs![a, b, c];
    assert_eq!(list.iter().collect::<Vec<_>>(), vec![&a, &b, &c]);

    // Current VS's length
    // Be careful with race conditions since the value, when used, may not be true anymore
    assert_eq!(list.len(), 3);

    // The 'iter' method makes a one-time lock-free iterator (Iter)
    for (index, element) in list.iter().enumerate() {
        assert_eq!(index, *element);
    }

    // You can get the current iteration index
    // iter.next() == iter.len() means iteration ended (iter.next() == None)
    let mut iter = &mut list.iter();
    assert_eq!(iter.index(), 0);
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.index(), 1);

    // List can also be cleared (but current iterators are not affected)
    list.clear();

    assert_eq!(iter.len(), 3);
    assert_eq!(list.len(), 0);
    assert_eq!(list.iter().len(), 0);
    assert_eq!((&mut list.iter()).next(), None);

    println!("Single thread example ended without errors");
}
