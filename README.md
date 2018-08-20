# Voluntary Servitude

- Currently only implements a thread-safe appendable list with a lock-free iterator

FFI implementation available, C examples are in **./examples** folder

  *FFI docs are in 'ffi' module documentation*

Logging is available behind the 'logs' feature

  *Use RUST_LOG env var to config the level (trace, debug, info)*

  Example:

    RUST_LOG=trace cargo test --features "logs"

**This is production ready, but not currently used, so it's not in the package manager, if you need this, just ask and I will upload it**

## Api Docs

Since it's not in the package manager you have to generate it, run:

    cargo docs --open

## Macros

- vsread!

  **Exactly like the 'vec!' macro**

```
    assert_eq!(vsread![].iter().collect::<Vec<_>>(), vec![]);
    assert_eq!(vsread![1, 2, 3].iter().collect::<Vec<_>>(), vec![&1, &2, &3]);
    assert_eq!(vsread![1; 3].iter().collect::<Vec<_>>(), vec![&1, &1, &1]);
```

## Datastructures

- VSRead

  **Appendable list with lock-free iteration**

## Basic usage

### Single thread

```
    use voluntary_servitude::VSRead;

    let list = vsread![0, 1, 2];
    assert_eq!((3..10000).map(|i| list.append(i)).count(), 9997);

    let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
    assert_eq!(count, 10000);

    assert_eq!((0..10000).map(|i| list.append(i)).count(), 10000);
    let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&(i % 10000), el)).count();
    assert_eq!(count, 20000);

    // List can also be cleared
    list.clear();
    assert_eq!(list.iter().count(), 0);
```

### Single producer, single consumer

```
    use std::{thread::spawn, sync::Arc};
    use voluntary_servitude::VSRead;

    let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
    let producer = Arc::clone(&list);
    let _handler = spawn(move || {
        let _ = (0..10000).map(|i| producer.append(i)).count();
    });

    loop {
        let count = list.iter().count();
        println!("{} elements", count);
        if count == 10000 { break; }
    }
```

### Multi producer, multi consumer

```
    use std::{thread::spawn, sync::Arc};
    use voluntary_servitude::VSRead;

    const consumers: usize = 8;
    const producers: usize = 4;

    let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
    let mut handlers = vec![];

    for _ in (0..producers) {
        let l = Arc::clone(&list);
        handlers.push(spawn(move || { let _ = (0..10000).map(|i| l.append(i)).count(); }));
    }

    for c in (0..consumers) {
        let consumer = Arc::clone(&list);
        handlers.push(spawn(move || {
            loop {
                let count = consumer.iter().count();
                println!("{} elements", count);
                if count == producers * 10000 { break; }
            }
        }));
    }
```
