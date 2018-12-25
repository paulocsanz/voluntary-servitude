#[macro_use]
extern crate criterion;
#[macro_use]
extern crate voluntary_servitude;

use criterion::Criterion;
use std::iter::FromIterator;
use voluntary_servitude::VS;

fn new(c: &mut Criterion) {
    c.bench_function("new", move |b| b.iter(|| VS::<()>::new()));
}

fn append(c: &mut Criterion) {
    let vs: VS<u8> = VS::default();
    c.bench_function("append", move |b| b.iter(|| vs.append(10)));
}

fn iter(c: &mut Criterion) {
    let vs = vs![10u8; 1000];
    c.bench_function("iter", move |b| b.iter(|| vs.iter()));
}

fn len(c: &mut Criterion) {
    let vs = vs![10u8; 1000];
    c.bench_function("len", move |b| b.iter(|| vs.len()));
}

fn is_empty(c: &mut Criterion) {
    let vs = vs![10u8; 1000];
    c.bench_function("is_empty", move |b| b.iter(|| vs.is_empty()));
}

fn clear(c: &mut Criterion) {
    c.bench_function("clear", move |b| b.iter(|| vs![2, 3].clear()));
}

fn empty(c: &mut Criterion) {
    c.bench_function("empty", move |b| b.iter(|| vs![2, 3].empty()));
}

fn swap(c: &mut Criterion) {
    let vs = vs![3, 2];
    c.bench_function("swap", move |b| b.iter(|| vs.swap(&mut vs![2, 3])));
}

fn extend(c: &mut Criterion) {
    let vs = vs![3, 2];
    c.bench_function("extend", move |b| b.iter(|| vs.extend(vs.iter().cloned())));
}

fn from_iter(c: &mut Criterion) {
    let vs = vs![3, 2];
    c.bench_function("from_iter", move |b| {
        b.iter(|| VS::from_iter(vs.iter().cloned()))
    });
}

criterion_group!(methods, new, append, iter, len, is_empty, clear, empty, swap, extend, from_iter);
criterion_main!(methods);
