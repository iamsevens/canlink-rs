//! Queue performance benchmarks (T051)
//!
//! Verifies O(1) enqueue/dequeue operations for BoundedQueue.
//!
//! ## Success Criteria
//!
//! - Push operation: O(1) - constant time regardless of queue size
//! - Pop operation: O(1) - constant time regardless of queue size
//! - Overflow handling: O(1) - constant time for DropOldest/DropNewest

use canlink_hal::message::CanMessage;
use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Create a test message with given ID
fn make_message(id: u32) -> CanMessage {
    CanMessage::new_extended(id, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap()
}

/// Benchmark push operation at different queue fill levels
///
/// Verifies O(1) push regardless of queue size
fn bench_push_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_push");
    group.throughput(Throughput::Elements(1));

    // Test at different queue sizes to verify O(1)
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("empty_queue", size), size, |b, &size| {
            let mut queue = BoundedQueue::new(size);
            let msg = make_message(0x100);
            b.iter(|| {
                queue.push(black_box(msg.clone())).unwrap();
                queue.clear(); // Reset for next iteration
            });
        });

        group.bench_with_input(BenchmarkId::new("half_full", size), size, |b, &size| {
            let mut queue = BoundedQueue::new(size);
            // Fill to 50%
            for i in 0..(size / 2) as u32 {
                queue.push(make_message(i)).unwrap();
            }
            let msg = make_message(0x100);
            b.iter(|| {
                queue.push(black_box(msg.clone())).unwrap();
                queue.pop(); // Keep at 50%
            });
        });

        group.bench_with_input(BenchmarkId::new("nearly_full", size), size, |b, &size| {
            let mut queue = BoundedQueue::new(size);
            // Fill to 90%
            for i in 0..((size * 9) / 10) as u32 {
                queue.push(make_message(i)).unwrap();
            }
            let msg = make_message(0x100);
            b.iter(|| {
                queue.push(black_box(msg.clone())).unwrap();
                queue.pop(); // Keep at 90%
            });
        });
    }

    group.finish();
}

/// Benchmark pop operation at different queue fill levels
///
/// Verifies O(1) pop regardless of queue size
fn bench_pop_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_pop");
    group.throughput(Throughput::Elements(1));

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("from_full", size), size, |b, &size| {
            let mut queue = BoundedQueue::new(size);
            // Fill completely
            for i in 0..size as u32 {
                queue.push(make_message(i)).unwrap();
            }
            let msg = make_message(0x100);
            b.iter(|| {
                black_box(queue.pop());
                queue.push(msg.clone()).unwrap(); // Refill
            });
        });

        group.bench_with_input(BenchmarkId::new("from_half", size), size, |b, &size| {
            let mut queue = BoundedQueue::new(size);
            // Fill to 50%
            for i in 0..(size / 2) as u32 {
                queue.push(make_message(i)).unwrap();
            }
            let msg = make_message(0x100);
            b.iter(|| {
                black_box(queue.pop());
                queue.push(msg.clone()).unwrap(); // Refill
            });
        });
    }

    group.finish();
}

/// Benchmark overflow handling with DropOldest policy
///
/// Verifies O(1) overflow handling
fn bench_overflow_drop_oldest(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_overflow_drop_oldest");
    group.throughput(Throughput::Elements(1));

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("overflow", size), size, |b, &size| {
            let mut queue = BoundedQueue::with_policy(size, QueueOverflowPolicy::DropOldest);
            // Fill completely
            for i in 0..size as u32 {
                queue.push(make_message(i)).unwrap();
            }
            let msg = make_message(0x100);
            b.iter(|| {
                // This will drop oldest and add new
                queue.push(black_box(msg.clone())).unwrap();
            });
        });
    }

    group.finish();
}

/// Benchmark overflow handling with DropNewest policy
///
/// Verifies O(1) overflow handling
fn bench_overflow_drop_newest(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_overflow_drop_newest");
    group.throughput(Throughput::Elements(1));

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("overflow", size), size, |b, &size| {
            let mut queue = BoundedQueue::with_policy(size, QueueOverflowPolicy::DropNewest);
            // Fill completely
            for i in 0..size as u32 {
                queue.push(make_message(i)).unwrap();
            }
            let msg = make_message(0x100);
            b.iter(|| {
                // This will reject the new message
                let _ = queue.push(black_box(msg.clone()));
            });
        });
    }

    group.finish();
}

/// Benchmark batch operations (push 1000 messages)
fn bench_batch_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_batch");
    group.throughput(Throughput::Elements(1000));

    // Pre-create messages
    let messages: Vec<CanMessage> = (0..1000u32).map(make_message).collect();

    group.bench_function("push_1000_messages", |b| {
        let mut queue = BoundedQueue::new(2000);
        b.iter(|| {
            for msg in &messages {
                queue.push(black_box(msg.clone())).unwrap();
            }
            queue.clear();
        });
    });

    group.bench_function("push_pop_1000_messages", |b| {
        let mut queue = BoundedQueue::new(2000);
        b.iter(|| {
            for msg in &messages {
                queue.push(black_box(msg.clone())).unwrap();
            }
            for _ in 0..1000 {
                black_box(queue.pop());
            }
        });
    });

    group.finish();
}

/// Benchmark peek operation
fn bench_peek_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_peek");
    group.throughput(Throughput::Elements(1));

    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(BenchmarkId::new("peek", size), size, |b, &size| {
            let mut queue = BoundedQueue::new(size);
            // Fill to 50%
            for i in 0..(size / 2) as u32 {
                queue.push(make_message(i)).unwrap();
            }
            b.iter(|| {
                black_box(queue.peek());
            });
        });
    }

    group.finish();
}

/// Benchmark queue statistics retrieval
fn bench_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_stats");

    let mut queue = BoundedQueue::new(1000);
    // Do some operations to populate stats
    for i in 0..500u32 {
        queue.push(make_message(i)).unwrap();
    }
    for _ in 0..100 {
        queue.pop();
    }

    group.bench_function("get_stats", |b| {
        b.iter(|| {
            black_box(queue.stats());
        });
    });

    group.finish();
}

/// Benchmark capacity adjustment
fn bench_adjust_capacity(c: &mut Criterion) {
    let mut group = c.benchmark_group("queue_adjust_capacity");

    group.bench_function("shrink_by_half", |b| {
        b.iter_batched(
            || {
                let mut queue = BoundedQueue::new(1000);
                for i in 0..1000u32 {
                    queue.push(make_message(i)).unwrap();
                }
                queue
            },
            |mut queue| {
                queue.adjust_capacity(black_box(500));
                queue
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("expand_capacity", |b| {
        b.iter_batched(
            || {
                let mut queue = BoundedQueue::new(500);
                for i in 0..500u32 {
                    queue.push(make_message(i)).unwrap();
                }
                queue
            },
            |mut queue| {
                queue.adjust_capacity(black_box(1000));
                queue
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_push_operation,
    bench_pop_operation,
    bench_overflow_drop_oldest,
    bench_overflow_drop_newest,
    bench_batch_push,
    bench_peek_operation,
    bench_stats,
    bench_adjust_capacity,
);

criterion_main!(benches);
