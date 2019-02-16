use criterion::*;
use std::iter::FromIterator;
use voluntary_servitude::{VS, vs};

fn vs_new(c: &mut Criterion) {
    c.bench_function("vs_new", move |b| b.iter(|| VS::<()>::new()));
}

fn vs_append(c: &mut Criterion) {
    let vs: VS<u8> = VS::default();
    c.bench_function("vs_append", move |b| b.iter(|| vs.append(10)));
}

fn vs_iter(c: &mut Criterion) {
    let vs = vs![10u8; 1000];
    c.bench_function("vs_iter", move |b| b.iter(|| vs.iter()));
}

fn vs_len(c: &mut Criterion) {
    let vs = vs![10u8; 1000];
    c.bench_function("vs_len", move |b| b.iter(|| vs.len()));
}

fn vs_is_empty(c: &mut Criterion) {
    let vs = vs![10u8; 1000];
    c.bench_function("vs_is_empty", move |b| b.iter(|| vs.is_empty()));
}

fn vs_clear(c: &mut Criterion) {
    c.bench_function("vs_clear", move |b| b.iter(|| vs![2, 3].clear()));
}

fn vs_empty(c: &mut Criterion) {
    c.bench_function("vs_empty", move |b| b.iter(|| vs![2, 3].empty()));
}

fn vs_swap(c: &mut Criterion) {
    let vs = vs![3, 2];
    c.bench_function("vs_swap", move |b| b.iter(|| vs.swap(&mut vs![2, 3])));
}

fn vs_extend(c: &mut Criterion) {
    let vs = vs![3, 2];
    c.bench_function("vs_extend", move |b| b.iter(|| vs.extend(vec![1, 0, -1, -2, -3, -4])));
}

fn vs_from_iter(c: &mut Criterion) {
    let vs = vs![3, 2];
    c.bench_function("vs_from_iter", move |b| {
        b.iter(|| VS::from_iter(vs.iter().cloned()))
    });
}

fn vec_new(c: &mut Criterion) {
    c.bench_function("vec_new", move |b| b.iter(|| Vec::<()>::new()));
}

fn vec_append(c: &mut Criterion) {
    let mut vec: Vec<u8> = Vec::default();
    c.bench_function("vec_append", move |b| b.iter(|| vec.push(10)));
}

fn vec_iter(c: &mut Criterion) {
    let vec = vec![10u8; 1000];
    c.bench_function("vec_iter", move |b| b.iter(|| vec.iter()));
}

fn vec_len(c: &mut Criterion) {
    let vec = vec![10u8; 1000];
    c.bench_function("vec_len", move |b| b.iter(|| vec.len()));
}

fn vec_is_empty(c: &mut Criterion) {
    let vec = vec![10u8; 1000];
    c.bench_function("vec_is_empty", move |b| b.iter(|| vec.is_empty()));
}

fn vec_clear(c: &mut Criterion) {
    c.bench_function("vec_clear", move |b| b.iter(|| vec![2, 3].clear()));
}

fn vec_extend(c: &mut Criterion) {
    let mut vec = vec![3, 2];
    c.bench_function("vec_extend", move |b| b.iter(|| vec.extend(vec![1, 0, -1, -2, -3, -4])));
}

fn vec_from_iter(c: &mut Criterion) {
    let vec = vec![3, 2];
    c.bench_function("vec_from_iter", move |b| {
        b.iter(|| Vec::from_iter(vec.iter().cloned()))
    });
}

criterion_group!(vs, vs_new, vs_append, vs_iter, vs_len, vs_is_empty, vs_clear, vs_empty, vs_swap, vs_extend, vs_from_iter);
//criterion_group!(vec, vec_new, vec_append, vec_iter, vec_len, vec_is_empty, vec_clear, vec_extend, vec_from_iter);
criterion_main!(vs);//, vec);
