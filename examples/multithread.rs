use voluntary_servitude::vs;
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
            if count >= TOTAL {
                break;
            };
        }));
    }

    // Join threads
    for handler in handlers.into_iter() {
        handler.join().expect("Failed to join thread");
    }

    println!("Multi-thread example ended without errors");
}
