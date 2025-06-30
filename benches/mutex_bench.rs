use atomics::mutex::Mutex; // adjust the path if needed
use criterion::{Criterion, criterion_group, criterion_main};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::thread;

fn bench_custom_mutex(c: &mut Criterion) {
    c.bench_function("custom_mutex", |b| {
        b.iter(|| {
            let m = Arc::new(Mutex::new(0));
            let mut handles = vec![];
            for _ in 0..4 {
                let m = m.clone();
                handles.push(thread::spawn(move || {
                    for _ in 0..1000 {
                        *m.lock() += 1;
                    }
                }));
            }
            for h in handles {
                h.join().unwrap();
            }
        });
    });
}

fn bench_std_mutex(c: &mut Criterion) {
    c.bench_function("std_mutex", |b| {
        b.iter(|| {
            let m = Arc::new(StdMutex::new(0));
            let mut handles = vec![];
            for _ in 0..4 {
                let m = m.clone();
                handles.push(thread::spawn(move || {
                    for _ in 0..1000 {
                        *m.lock().unwrap() += 1;
                    }
                }));
            }
            for h in handles {
                h.join().unwrap();
            }
        });
    });
}

criterion_group!(benches, bench_std_mutex, bench_custom_mutex);
criterion_main!(benches);
