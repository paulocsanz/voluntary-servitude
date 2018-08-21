# Voluntary Servitude

*Uses system allocator by default, jemmaloc can be enabled with the 'jemmaloc' feature*

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

  **Appendable list that can create lock-free iterator**

- VSReadIter (can't instantiate it)

  **Lock-free iterator based on VSRead**

## Basic usage

### Single thread

```
    // Create VSRead with 3 elements
    // vsread![] makes an empty VSRead
    // vsread![1; 3] makes a VSRead with 3 elements with 1 as value
    let list = vsread![0, 1, 2];
    assert_eq!(list.len(), 3);

    // The 'iter' method makes a one-time lock-free iterator (VSReadIter) based on VSRead
    assert_eq!(list.iter().len(), 3);

    // You can get the current iteration index (can be compared with the length 'len')
    assert_eq!(list.iter().index(), 0);

    // Appends 9997 elements to it
    assert_eq!((3..10000).map(|i| list.append(i)).count(), 9997);

    // Iterates through all elements to ensure it's what we inserted
    let count = list.iter().enumerate().map(|(i, el)| assert_eq!(&i, el)).count();
    assert_eq!(count, 10000);

    // List can also be cleared
    list.clear();
    assert_eq!(list.len(), 0);
```

### Multi producer, multi consumer

```
    use std::{thread::spawn, sync::Arc};

    const CONSUMERS: usize = 8;
    const PRODUCERS: usize = 4;

    fn main() {
        let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
        let mut handlers = vec![];

        // Creates producer threads to insert 10k elements each
        for _ in 0..PRODUCERS {
            let l = Arc::clone(&list);
            handlers.push(spawn(move || { let _ = (0..10000).map(|i| l.append(i)).count(); }));
        }

        // Creates consumer threads to print number of elements until all elements are inserted
        for _ in 0..CONSUMERS {
            let consumer = Arc::clone(&list);
            handlers.push(spawn(move || {
                loop {
                    let count = consumer.iter().count();
                    println!("{} elements", count);
                    if count == PRODUCERS * 10000 { break; }
                }
            }));
        }

        // Join threads
        for handler in handlers.into_iter() {
            handler.join().expect("Failed to join thread");
        }
    }
```

### Single thread C example (FFI)

```
    #include<assert.h>
    #include "include/voluntary_servitude.h"

    int main(int argc, char **argv) {
        // Rust allocates memory through malloc
        vsread_t * const vsread = vsread_new();

        // Current vsread_t length
        // Be careful with data-races since the value, when used, may not be true anymore
        assert(vsread_len(vsread) == 0);

        const unsigned int data[2] = {12, 25};
        // Inserts void pointer to data to end of vsread_t
        vsread_append(vsread, (void *) data);
        vsread_append(vsread, (void *) (data + 1));

        // Creates a one-time lock-free iterator based on vsread_t
        vsread_iter_t * const iter = vsread_iter(vsread);
        // Index changes as you iter through vsread_iter_t
        assert(vsread_iter_index(iter) == 0);

        // Clears vsread_t, doesn't change existing iterators
        vsread_clear(vsread);
        assert(vsread_len(vsread) == 0);
        assert(vsread_iter_len(iter) == 2);

        assert(*(unsigned int *) vsread_iter_next(iter) == 12);
        assert(vsread_iter_index(iter) == 1);
        assert(*(unsigned int *) vsread_iter_next(iter) == 25);
        assert(vsread_iter_index(iter) == 2);

        assert(vsread_iter_next(iter) == NULL);
        assert(vsread_iter_index(iter) == 2);
        assert(vsread_iter_len(iter) == 2);

        // Never forget to free vsread_iter_t
        vsread_iter_destroy(iter);

        // Create updated vsread_iter_t
        vsread_iter_t * const iter2 = vsread_iter(vsread);
        assert(vsread_iter_len(iter2) == 0);
        assert(vsread_iter_next(iter2) == NULL);
        vsread_iter_destroy(iter2);

        // Never forget to free vsread_t
        vsread_destroy(vsread);

        return 0;
    }
```

### Multi thread C example (FFI)

```
    #include<pthread.h>
    #include<stdio.h>
    #include "../include/voluntary_servitude.h"

    const unsigned int num_producers = 4;
    const unsigned int num_consumers = 8;

    const unsigned int num_producer_values = 1000;
    const unsigned int data[3] = {12, 25, 89};

    void* producer();
    void* consumer();

    int main(int argc, char** argv)
    {
        // Rust allocates memory through malloc
        vsread_t * const vsread = vsread_new();
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
            if (pthread_create(&producers[current_thread], &attr, &producer, (void *) vsread) != 0) {
                fprintf(stderr, "Failed to create producer thread %d.\n", current_thread);
                exit(-2);
            }

        }

        // Creates consumers threads
        for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
            if (pthread_create(&consumers[current_thread], &attr, &consumer, (void *) vsread) != 0) {
                fprintf(stderr, "Failed to create consumer thread %d.\n", current_thread);
                exit(-3);
            }
        }

        // Join all threads, ensuring vsread_t* is not used anymore
        for (current_thread = 0; current_thread < num_producers; ++current_thread) {
            pthread_join(producers[current_thread], NULL);
        }
        for (current_thread = 0; current_thread < num_consumers; ++current_thread) {
            pthread_join(consumers[current_thread], NULL);
        }

        // Never forget to free the memory allocated through rust
        vsread_destroy(vsread);

        (void) argc;
        (void) argv;
        return 0;
    }


    void * producer(void * const vsread){
        unsigned int index;
        for (index = 0; index < num_producer_values; ++index) {
            vsread_append(vsread, (void *) (data + (index % 2)));
        }
        return NULL;
    }

    void * consumer(void * const vsread) {
        const unsigned int total_values = num_producers * num_producer_values;
        unsigned int values = 0;
        while (values < total_values) {
            unsigned int sum = (values = 0);
            vsread_iter_t * const iter = vsread_iter(vsread);
            const void * value;
            while ((value = vsread_iter_next(iter)) != NULL) {
                ++values;
                sum += *(unsigned int *) value;
            }
            printf("Consumer counts %d elements summing %d.\n", values, sum);

            vsread_iter_destroy(iter);
        }
        return NULL;
    }
```
