# Voluntary Servitude

* [API docs](https://docs.rs/voluntary-servitude/3.0.0/voluntary-servitude)

- Implements a lock-free thread-safe appendable list with a lock-free iterator
- FFI implementation available, C examples are in **./examples** folder

    `cd examples && make test` (runs rust and C examples)
- Last release is in **./dist** (uses system allocator)
- Uses rust allocator by default, system allocator can be enable with the 'system-alloc' feature
- Logging is available behind the "logs" feature and RUST_LOG env var
- Serde's Serialize/Deserialize traits available behind the 'serde-traits' feature

## Licenses

[MIT](LICENSE_MIT) or [Apache-2.0](LICENSE_APACHE)

## Examples

### Single thread

```
const ELEMENTS: usize = 10000;
// Creates VoluntaryServitude with 3 elements
// vs![], voluntary_servitude![] and VoluntaryServitude::default() make an empty VoluntaryServitude
// vs![1; 3] or voluntary_servitude![1; 3] make a VoluntaryServitude with 3 elements equal to 1
// vs![0, 1, 2] or voluntary_servitude![0, 1, 2] make a VoluntaryServitude with [1, 2, 3]
let list = voluntary_servitude![0, 1, 2];

// Current VoluntaryServitude length
// Be careful with data-races since the value, when used, may not be true anymore
assert_eq!(list.len(), 3);

// The 'iter' method makes a one-time lock-free iterator (VSIter) based on VoluntaryServitude
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

println!("Single thread example ended without errors");
```

### Multi producer, multi consumer

```
use std::{sync::Arc, thread::spawn};

const CONSUMERS: usize = 8;
const PRODUCERS: usize = 4;
const ELEMENTS: usize = 10000;

fn main() {
    let list = Arc::new(vs![]); // or voluntary_servitude![] or VoluntaryServitude::default();
    let mut handlers = vec![];

    // Creates producer threads to insert 10k elements
    for _ in 0..PRODUCERS {
        let l = Arc::clone(&list);
        handlers.push(spawn(move || { let _ = (0..ELEMENTS).map(|i| l.append(i)).count(); }));
    }

    // Creates consumer threads to print number of elements until all of them are inserted
    for _ in 0..CONSUMERS {
        let consumer = Arc::clone(&list);
        handlers.push(spawn(move || {
            loop {
                let count = consumer.iter().count();
                println!("{} elements", count);
                if count == PRODUCERS * ELEMENTS { break; }
            }
        }));
    }

    // Join threads
    for handler in handlers.into_iter() {
        handler.join().expect("Failed to join thread");
    }

    println!("Multi thread example ended without errors");
}
```

### Single thread C example (FFI)

```
#include<assert.h>
#include<stdio.h>
#include "include/voluntary_servitude.h"

int main(int argc, char **argv) {
    vs_t * vs = vs_new();

    // Current vs_t length
    // Be careful with data-races since the value, when used, may not be true anymore
    assert(vs_len(vs) == 0);

    const unsigned int data[2] = {12, 25};
    // Inserts void pointer to data to the end of vs_t
    vs_append(vs, (void *) &data[0]);
    vs_append(vs, (void *) &data[1]);

    // Creates a one-time lock-free iterator based on vs_t
    vs_iter_t * iter = vs_iter(vs);
    // Index changes as you iter through vs_iter_t
    assert(vs_iter_index(iter) == 0);

    // Clearing vs_t, doesn't change existing iterators
    vs_clear(vs);
    assert(vs_len(vs) == 0);
    assert(vs_iter_len(iter) == 2);

    assert(*(unsigned int *) vs_iter_next(iter) == 12);
    assert(vs_iter_index(iter) == 1);
    assert(*(unsigned int *) vs_iter_next(iter) == 25);
    assert(vs_iter_index(iter) == 2);

    assert(vs_iter_next(iter) == NULL);
    assert(vs_iter_index(iter) == 2);
    assert(vs_iter_len(iter) == 2);

    // Never forget to free vs_iter_t
    assert(vs_iter_destroy(iter) == 0);

    // Create updated vs_iter_t
    vs_iter_t * iter2 = vs_iter(vs);

    // Never forget to free vs_t
    assert(vs_destroy(vs) == 0);

    // vs_iter_t keeps existing after the original vs_t is freed
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

### Multi thread C example (FFI)

```
#include<pthread.h>
#include<assert.h>
#include<stdio.h>
#include "../include/voluntary_servitude.h"

const unsigned int num_producers = 4;
const unsigned int num_consumers = 8;

const unsigned int num_producer_values = 1000;
const unsigned int data[3] = {12, 25, 89};
const size_t last_index = sizeof(data) / sizeof(data[0]) - 1;

void * producer();
void * consumer();

int main(int argc, char** argv) {
    // You are responsible for making sure 'vs' exists
    // while the producers and consumers do
    vs_t * const vs = vs_new();
    unsigned int current_thread = 0;
    pthread_attr_t attr;
    pthread_t consumers[num_consumers],
              producers[num_producers];

    if (pthread_attr_init(&attr) != 0) {
        fprintf(stderr, "Failed to initialize pthread arguments.\n");
        exit(-1);
    }

    // Creates producer threads
    for (current_thread = 0; current_thread < num_producers; ++current_thread) {
        if (pthread_create(&producers[current_thread], &attr, &producer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create producer thread %d.\n", current_thread);
            exit(-2);
        }

    }

    // Creates consumers threads
    for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
        if (pthread_create(&consumers[current_thread], &attr, &consumer, (void *) vs) != 0) {
            fprintf(stderr, "Failed to create consumer thread %d.\n", current_thread);
            exit(-3);
        }
    }

    // Join all threads, ensuring vs_t* is not used anymore
    for (current_thread = 0; current_thread < num_producers; ++current_thread) {
        pthread_join(producers[current_thread], NULL);
    }
    for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
        pthread_join(consumers[current_thread], NULL);
    }

    // Never forget to free the memory allocated through rust
    assert(vs_destroy(vs) == 0);

    printf("Multi thread example ended without errors\n");
    (void) argc;
    (void) argv;
    return 0;
}


void * producer(void * const vs){
    unsigned int index;
    for (index = 0; index < num_producer_values; ++index) {
        assert(vs_append(vs, (void *) &data[index % last_index]) == 0);
    }
    return NULL;
}

void * consumer(void * const vs) {
    const unsigned int total_values = num_producers * num_producer_values;
    unsigned int values;

    while (values < total_values) {
        unsigned int sum = (values = 0);
        vs_iter_t * const iter = vs_iter(vs);
        const void * value;

        while ((value = vs_iter_next(iter)) != NULL) {
            ++values;
            sum += *(unsigned int *) value;
        }
        printf("Consumer counts %d elements summing %d.\n", values, sum);

        assert(vs_iter_destroy(iter) == 0);
    }
    return NULL;
}
```
