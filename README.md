# Voluntary Servitude

* [Documentation](https://docs.rs/voluntary-servitude/3.0.11/voluntary-servitude)

# Features
 - Atomic abstractions (`Atomic`, `AtomicOption`, `FillOnceAtomicOption`, `FillOnceAtomicArc`)
 - Thread-safe appendable list with a lock-free iterator (`VoluntaryServitude` - also called `VS`)
 - Serde serialization (`serde-traits` feature)
 - `par_extend`, `from_par_iter` rayon implementation
 - Logging (`logs` feature)

# Atomic abstractions
 - **Atomic** -> atomic `Box<T>`
 - **AtomicOption** -> atomic `Option<Box<T>>`
 - **FillOnceAtomicOption** -> atomic `Option<Box<T>>` that can give references (ideal for iterators)
 - **FillOnceAtomicArc** -> atomic `Option<Arc<T>>` with a limited Api (like `FillOnceAtomicOption`)

With `Atomic` and `AtomicOption` it's not safe to get a reference, you must replace the value to access it.

To safely get a reference to T you must use `FillOnceAtomicOption` and accept the API limitations (initially `None` but can be filled once).

A safe `AtomicArc` is impossible without a garbage collector, so you must use `ArcCell` from `crossbeam` (locks to clone), some data-structure from `arc-swap` or `FillOnceAtomicArc` and accept the limited Api.

## Licenses

[MIT](master/license/MIT) and [Apache-2.0](master/license/APACHE)

## `VoluntaryServitude` Examples

- [Single thread](#single-thread)
- [Multi producers, multi consumers](#multi-producers-multi-consumers)

### Single thread

```rust
#[macro_use]
extern crate voluntary_servitude;

fn main() {
    let (a, b, c) = (0usize, 1usize, 2usize);
    // VS alias to VoluntaryServitude
    // vs! alias to voluntary_servitude! (and operate like vec!)
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
    let mut iter = list.iter();
    assert_eq!(iter.index(), 0);
    assert_eq!(iter.next(), Some(&0));
    assert_eq!(iter.index(), 1);

    // List can also be cleared (but current iterators are not affected)
    list.clear();

    assert_eq!(iter.len(), 3);
    assert_eq!(list.len(), 0);
    assert_eq!(list.iter().len(), 0);
    assert_eq!(list.iter().next(), None);
}
```

### Multi producers, multi consumers

```rust
#[macro_use]
extern crate voluntary_servitude;
use std::{sync::Arc, thread::spawn};

const CONSUMERS: usize = 8;
const PRODUCERS: usize = 4;
const ELEMENTS: usize = 10_000_000;

fn main() {
    let list = Arc::new(vs![]);
    let mut handlers = vec![];

    // Creates producer threads to insert 10k elements
    for _ in 0..PRODUCERS {
        let l = Arc::clone(&list);
        handlers.push(spawn(move || {
            let _ = (0..ELEMENTS).map(|i| l.append(i)).count();
        }));
    }

    // Creates consumer threads to print number of elements
    // Until all of them are inserted
    for _ in 0..CONSUMERS {
        const TOTAL: usize = PRODUCERS * ELEMENTS;
        let consumer = Arc::clone(&list);
        handlers.push(spawn(move || loop {
            let count = consumer.iter().count();
            println!("{} elements", count);
            if count >= TOTAL { break };
        }));
    }

    // Join threads
    for handler in handlers.into_iter() {
        handler.join().expect("Failed to join thread");
    }

    println!("Multi-thread rust example ended without errors");
}
```
