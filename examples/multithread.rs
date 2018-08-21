#[macro_use]
extern crate voluntary_servitude;

use std::{sync::Arc, thread::spawn};

const CONSUMERS: usize = 8;
const PRODUCERS: usize = 4;

fn main() {
    let list = Arc::new(vsread![]); // or Arc::new(VSRead::default());
    let mut handlers = vec![];

    // Creates producer threads to insert 10k elements each
    for _ in 0..PRODUCERS {
        let l = Arc::clone(&list);
        handlers.push(spawn(move || {
            let _ = (0..10000).map(|i| l.append(i)).count();
        }));
    }

    // Creates consumer threads to print number of elements until all elements are inserted
    for _ in 0..CONSUMERS {
        let consumer = Arc::clone(&list);
        handlers.push(spawn(move || loop {
            let count = consumer.iter().count();
            println!("{} elements", count);
            if count == PRODUCERS * 10000 {
                break;
            }
        }));
    }

    // Join threads
    for handler in handlers.into_iter() {
        handler.join().expect("Failed to join thread");
    }
    println!("Test ended");
}
