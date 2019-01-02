#[macro_use]
extern crate voluntary_servitude;
#[cfg(feature = "logs")]
extern crate env_logger;

use std::sync::{atomic::AtomicBool, atomic::AtomicUsize, atomic::Ordering, Arc};
use std::{cmp::max, thread::spawn};

fn setup_logger() {
    use std::sync::Once;
    #[allow(unused)]
    static INITIALIZE: Once = Once::new();
    #[cfg(feature = "logs")]
    INITIALIZE.call_once(env_logger::init);
}

#[test]
fn single_thread() {
    setup_logger();
    let list = voluntary_servitude![];
    for i in 0..10000 {
        list.append(i);
    }

    for (i, el) in list.iter().enumerate() {
        assert_eq!(&i, el);
    }
}

#[test]
fn single_producer_single_consumer() {
    setup_logger();
    let count = 10000;
    let list = Arc::new(voluntary_servitude![]);
    let finished = Arc::new(AtomicBool::new(false));

    let list_clone = Arc::clone(&list);
    let finished_clone = Arc::clone(&finished);
    let _handler = spawn(move || {
        for i in 0..count {
            list_clone.append(i + 1)
        }
        finished_clone.store(true, Ordering::Relaxed);
    });

    let mut total_max = 0;
    let mut last_len = 0;
    while !finished.load(Ordering::Relaxed) {
        let mut inner_max = 0;
        let mut len = 0;
        for (i, num) in list.iter().enumerate() {
            assert_eq!(i + 1, *num);
            inner_max = max(i + 1, inner_max);
            len = i;
        }
        assert!(
            inner_max > total_max || inner_max == count || len == last_len,
            "{} > {} || {} == {} || {} == {}",
            inner_max,
            total_max,
            inner_max,
            count,
            len,
            last_len
        );
        last_len = len;
        total_max = inner_max
    }
    assert_eq!(list.iter().count(), count);
}

#[test]
fn multi_producer_single_consumer() {
    setup_logger();
    let count = 10;
    let list = Arc::new(voluntary_servitude![]);
    let num_producers = 1000;
    let mut producers = vec![];
    let finished = Arc::new(AtomicUsize::new(0));

    for _ in 0..num_producers {
        let finished = Arc::clone(&finished);
        let list = Arc::clone(&list);
        producers.push(spawn(move || {
            for i in 0..count {
                list.append(i);
            }
            finished.fetch_add(1, Ordering::Relaxed);
        }));
    }

    let mut last_len = 0;
    while finished.load(Ordering::Relaxed) < num_producers {
        let len = list.iter().count();
        assert!(len >= last_len);
        last_len = len;
    }
    let len = list.iter().count();
    assert_eq!(len, num_producers * count);
}

#[test]
fn single_producer_multi_consumer() {
    setup_logger();
    let count = 10000;
    let list = Arc::new(voluntary_servitude![]);
    let num_consumers = 50;
    let mut consumers = vec![];
    let finished = Arc::new(AtomicBool::new(false));

    for _ in 0..num_consumers {
        let finished = Arc::clone(&finished);
        let list = Arc::clone(&list);
        consumers.push(spawn(move || {
            let mut len = 0;
            while !finished.load(Ordering::Relaxed) {
                let inner_len = list.iter().count();
                assert!(inner_len >= len);
                len = inner_len;
            }
            len = list.iter().count();
            assert_eq!(len, list.iter().count());
            assert_eq!(len, count);
        }));
    }

    for i in 0..count {
        list.append(i);
    }
    finished.store(true, Ordering::Relaxed);

    for thread in consumers {
        thread.join().unwrap();
    }
}

#[test]
fn multi_producer_multi_consumer() {
    setup_logger();
    let count = 1000;
    let list = Arc::new(voluntary_servitude![]);
    let num_producers = 50;
    let mut producers = vec![];
    let finished = Arc::new(AtomicUsize::new(0));

    let num_consumers = 1000;
    let mut consumers = vec![];

    for _ in 0..num_producers {
        let finished = Arc::clone(&finished);
        let list = Arc::clone(&list);
        producers.push(spawn(move || {
            for i in 0..count {
                list.append(i);
            }
            finished.fetch_add(1, Ordering::Relaxed);
        }));
    }

    for _ in 0..num_consumers {
        let finished = Arc::clone(&finished);
        let list = Arc::clone(&list);
        consumers.push(spawn(move || {
            let mut len = 0;
            while finished.load(Ordering::Relaxed) < num_producers {
                let inner_len = list.iter().count();
                assert!(inner_len >= len);
                len = inner_len;
            }
            len = list.iter().count();
            assert_eq!(len, list.iter().count());
            assert_eq!(len, count * num_producers);
        }));
    }

    for (consumer, producer) in consumers.into_iter().zip(producers) {
        consumer.join().unwrap();
        producer.join().unwrap();
    }
}

#[test]
fn clear() {
    setup_logger();
    let list = voluntary_servitude![1];
    assert_eq!(list.iter().count(), 1);
    list.clear();
    assert_eq!(list.iter().count(), 0);
    list.append(3);
    list.append(3);
    list.append(3);
    list.clear();
    assert_eq!(list.iter().count(), 0);
}

fn elements_n(num: usize) {
    println!("{} users", num);
    setup_logger();
    let list = voluntary_servitude![];
    for i in 0..num {
        list.append(i);
    }
    assert_eq!(list.iter().count(), num);
    assert_eq!((&mut list.iter()).next(), Some(&0));
    for (i, el) in list.iter().enumerate() {
        assert_eq!(*el, i);
    }

    let mut iter = &mut list.iter();
    let iter_count = &mut list.iter();
    list.clear();
    assert_eq!(iter_count.count(), num);
    assert_eq!(iter.next(), Some(&0));
}

#[test]
fn elements_100k() {
    elements_n(100_000);
}

#[test]
#[ignore]
fn elements_1m() {
    elements_n(500_000);
    elements_n(1_000_000);
}

#[test]
#[ignore]
fn elements_50m() {
    elements_n(10_000_000);
    elements_n(50_000_000);
}

#[test]
#[ignore]
fn elements_500m() {
    elements_n(100_000_000);
    elements_n(500_000_000);
}

#[test]
#[ignore]
fn elements_1b() {
    elements_n(1_000_000_000);
}
