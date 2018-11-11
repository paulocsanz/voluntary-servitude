# Voluntary Servitude

* [Documentation](https://docs.rs/voluntary-servitude/3.0.6/voluntary-servitude)

# Features
 - Atomic abstractions (`Atomic`, `AtomicOption`, `FillOnceAtomicOption`, `FillOnceAtomicArc`)
 - Thread-safe appendable list with a lock-free iterator (`VoluntaryServitude` - also called `VS`)
 - Serde serialization (`serde-traits` feature)
 - `par_extend`, `from_par_iter` rayon implementation
 - Call this code from C (FFI) (also in **./examples**)
 - Logging (`logs` feature)

# Atomic abstractions
 - **Atomic** -> atomic `Box<T>`
 - **AtomicOption** -> atomic `Option<Box<T>>`
 - **FillOnceAtomicOption** -> atomic `Option<Box<T>>` that can give references (ideal for iterators)
 - **FillOnceAtomicArc** -> atomic `Option<Arc<T>>` with a limited Api (like `FillOnceAtomicOption`)

With `Atomic` and `AtomicOption` it's not safe to get a reference, you must replace the value to access it

To safely get a reference to T you must use `FillOnceAtomicOption` and accept the API limitations

A safe `AtomicArc` is impossible, so you must use `ArcCell` from crossbeam (locks to clone) or `FillOnceAtomicArc`

## Licenses

[MIT](LICENSE_MIT) and [Apache-2.0](LICENSE_APACHE)

## `VoluntaryServitude` Examples

- [Single thread - Rust](#single-thread)
- [Multi producers, multi consumers - Rust](#multi-producers-multi-consumers)
- [Single thread - C](#single-thread-c-ffi)
- [Multi producers, multi consumers - C](#multi-producers-multi-consumers---c-ffi)

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

### Single thread C (FFI)

```c
#include<assert.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

int main(int argc, char **argv) {
    // You are responsible for making sure 'vs' exists while accessed
    vs_t * vs = vs_new();

    // Current vs_t length
    // Be careful with race conditions since the value, when used, may not be true anymore
    assert(vs_len(vs) == 0);

    const unsigned int data[2] = {12, 25};
    // Inserts void pointer to data to end of vs_t
    vs_append(vs, (void *) &data[0]);
    vs_append(vs, (void *) &data[1]);

    // Creates a one-time lock-free iterator based on vs_t
    vs_iter_t * iter = vs_iter(vs);

    // Clearing vs_t, doesn't change existing iterators
    vs_clear(vs);
    assert(vs_len(vs) == 0);
    assert(vs_iter_len(iter) == 2);

    assert(*(unsigned int *) vs_iter_next(iter) == 12);
    // Index changes as you iter through vs_iter_t
    assert(vs_iter_index(iter) == 1);
    assert(*(unsigned int *) vs_iter_next(iter) == 25);
    assert(vs_iter_index(iter) == 2);

    assert(vs_iter_next(iter) == NULL);
    assert(vs_iter_index(iter) == 2);
    // Index doesn't increase after it gets equal to 'len'
    // Length also is unable to increase after iterator is consumed
    assert(vs_iter_index(iter) == vs_iter_len(iter));

    // Never forget to free vs_iter_t
    assert(vs_iter_destroy(iter) == 0);

    // Create updated vs_iter_t
    vs_iter_t * iter2 = vs_iter(vs);

    // Never forget to free vs_t
    assert(vs_destroy(vs) == 0);

    // vs_iter_t keeps existing after the original vs_t is freed (or cleared)
    assert(vs_iter_len(iter2) == 0);
    assert(vs_iter_next(iter2) == NULL);
    assert(vs_iter_index(iter2) == 0);

    assert(vs_iter_destroy(iter2) == 0);

    printf("Single thread example ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}
```

### Multi producers, multi consumers - C (FFI)

```c
#include<pthread.h>
#include<assert.h>
#include<stdlib.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

const unsigned int num_consumers = 8;
const unsigned int num_producers = 4;
const unsigned int num_threads = 12;

const unsigned int num_producer_values = 10000000;
const unsigned int data = 3;

void * producer(void *);
void * consumer(void *);

int main(int argc, char** argv) {
    // You are responsible for making sure 'vs' exists while accessed
    vs_t * vs = vs_new();
    uint8_t thread = 0;
    pthread_attr_t attr;
    pthread_t threads[num_threads];

    if (pthread_attr_init(&attr) != 0) {
        fprintf(stderr, "Failed to initialize pthread arguments.\n");
        exit(-1);
    }

    // Creates producer threads
    for (thread = 0; thread < num_producers; ++thread) {
        if (pthread_create(&threads[thread], &attr, &producer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create producer thread %d.\n", thread);
            exit(-2);
        }

    }

    // Creates consumers threads
    for (thread = 0; thread < num_consumers; ++thread) {
        if (pthread_create(&threads[num_producers + thread], &attr, &consumer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create consumer thread %d.\n", thread);
            exit(-3);
        }
    }

    // Join all threads, ensuring vs_t* is not used anymore
    for (thread = 0; thread < num_threads; ++thread) {
        pthread_join(threads[thread], NULL);
    }

    // Never forget to free the memory allocated through the lib
    assert(vs_destroy(vs) == 0);

    printf("Multi-thread C example ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}

void * producer(void * vs){
    unsigned int index;
    for (index = 0; index < num_producer_values; ++index) {
        assert(vs_append(vs, (void *) &data) == 0);
    }
    return NULL;
}

void * consumer(void * vs) {
    const unsigned int total_values = num_producers * num_producer_values;
    unsigned int values = 0;

    while (values < total_values) {
        vs_iter_t * iter = vs_iter(vs);
        void * value;

        values = 0;
        while ((value = vs_iter_next(iter)) != NULL) {
            ++values;
        }
        printf("%d elements\n", values);

        // Never forget to free the memory allocated through the lib
        assert(vs_iter_destroy(iter) == 0);
    }
    return NULL;
}
```
