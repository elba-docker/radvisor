use criterion::{criterion_group, criterion_main, Criterion};
use libc::{clock_gettime, timespec, CLOCK_REALTIME};
use std::time::SystemTime;

// Designed to verify the fastest way of getting the read time

const SEC_TO_NS: u128 = 1000000000;
pub fn nano_ts() -> u128 {
    let mut tp: timespec = timespec {
        tv_sec:  0,
        tv_nsec: 0,
    };
    unsafe {
        clock_gettime(CLOCK_REALTIME, &mut tp);
    }
    (tp.tv_nsec as u128) + ((tp.tv_sec as u128) * SEC_TO_NS)
}

pub fn second_ts() -> u128 {
    let mut tp: timespec = timespec {
        tv_sec:  0,
        tv_nsec: 0,
    };
    unsafe {
        clock_gettime(CLOCK_REALTIME, &mut tp);
    }
    tp.tv_sec as u128
}

pub fn nano_ts2() -> u128 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t.as_nanos(),
        Err(_) => 0,
    }
}

pub fn second_ts2() -> u64 {
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(t) => t.as_secs(),
        Err(_) => 0,
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("nano_ts", |b| b.iter(|| nano_ts()));
    c.bench_function("nano_ts2", |b| b.iter(|| nano_ts2()));
    c.bench_function("second_ts", |b| b.iter(|| second_ts()));
    c.bench_function("second_ts2", |b| b.iter(|| second_ts2()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
