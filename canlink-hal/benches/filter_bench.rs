//! Filter performance benchmarks (T031)
//!
//! Verifies SC-003: Software filtering latency < 10 μs/message

use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter};
use canlink_hal::message::CanMessage;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Create a test message with the given ID
fn make_message(id: u16) -> CanMessage {
    CanMessage::new_standard(id, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap()
}

/// Benchmark single ID filter matching
fn bench_id_filter_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("id_filter");
    group.throughput(Throughput::Elements(1));

    let filter = IdFilter::new(0x123);
    let msg_match = make_message(0x123);
    let msg_no_match = make_message(0x456);

    group.bench_function("exact_match", |b| {
        b.iter(|| {
            use canlink_hal::filter::MessageFilter;
            black_box(filter.matches(black_box(&msg_match)))
        });
    });

    group.bench_function("no_match", |b| {
        b.iter(|| {
            use canlink_hal::filter::MessageFilter;
            black_box(filter.matches(black_box(&msg_no_match)))
        });
    });

    // Mask filter
    let mask_filter = IdFilter::with_mask(0x120, 0x7F0);
    group.bench_function("mask_match", |b| {
        b.iter(|| {
            use canlink_hal::filter::MessageFilter;
            black_box(mask_filter.matches(black_box(&msg_match)))
        });
    });

    group.finish();
}

/// Benchmark range filter matching
fn bench_range_filter_match(c: &mut Criterion) {
    let mut group = c.benchmark_group("range_filter");
    group.throughput(Throughput::Elements(1));

    let filter = RangeFilter::new(0x100, 0x1FF);
    let msg_in_range = make_message(0x150);
    let msg_out_of_range = make_message(0x300);

    group.bench_function("in_range", |b| {
        b.iter(|| {
            use canlink_hal::filter::MessageFilter;
            black_box(filter.matches(black_box(&msg_in_range)))
        });
    });

    group.bench_function("out_of_range", |b| {
        b.iter(|| {
            use canlink_hal::filter::MessageFilter;
            black_box(filter.matches(black_box(&msg_out_of_range)))
        });
    });

    group.finish();
}

/// Benchmark filter chain with varying number of filters
fn bench_filter_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("filter_chain");

    for num_filters in [1, 4, 8, 16].iter() {
        group.throughput(Throughput::Elements(1));

        let mut chain = FilterChain::new(4);
        for i in 0..*num_filters {
            chain.add_filter(Box::new(IdFilter::new((0x100 + i * 0x10) as u32)));
        }

        let msg_first_match = make_message(0x100);
        let msg_last_match = make_message((0x100 + (num_filters - 1) * 0x10) as u16);
        let msg_no_match = make_message(0x7FF);

        group.bench_with_input(
            BenchmarkId::new("first_match", num_filters),
            num_filters,
            |b, _| {
                b.iter(|| black_box(chain.matches(black_box(&msg_first_match))));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("last_match", num_filters),
            num_filters,
            |b, _| {
                b.iter(|| black_box(chain.matches(black_box(&msg_last_match))));
            },
        );

        group.bench_with_input(
            BenchmarkId::new("no_match", num_filters),
            num_filters,
            |b, _| {
                b.iter(|| black_box(chain.matches(black_box(&msg_no_match))));
            },
        );
    }

    group.finish();
}

/// Benchmark empty filter chain (pass-through mode)
fn bench_empty_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("empty_chain");
    group.throughput(Throughput::Elements(1));

    let chain = FilterChain::new(4);
    let msg = make_message(0x123);

    group.bench_function("pass_through", |b| {
        b.iter(|| black_box(chain.matches(black_box(&msg))));
    });

    group.finish();
}

/// Benchmark mixed filter types in chain
fn bench_mixed_filters(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_filters");
    group.throughput(Throughput::Elements(1));

    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100)));
    chain.add_filter(Box::new(IdFilter::with_mask(0x200, 0x7F0)));
    chain.add_filter(Box::new(RangeFilter::new(0x300, 0x3FF)));
    chain.add_filter(Box::new(IdFilter::new(0x400)));

    let msg_id_match = make_message(0x100);
    let msg_mask_match = make_message(0x205);
    let msg_range_match = make_message(0x350);
    let msg_no_match = make_message(0x500);

    group.bench_function("id_match", |b| {
        b.iter(|| black_box(chain.matches(black_box(&msg_id_match))));
    });

    group.bench_function("mask_match", |b| {
        b.iter(|| black_box(chain.matches(black_box(&msg_mask_match))));
    });

    group.bench_function("range_match", |b| {
        b.iter(|| black_box(chain.matches(black_box(&msg_range_match))));
    });

    group.bench_function("no_match_full_scan", |b| {
        b.iter(|| black_box(chain.matches(black_box(&msg_no_match))));
    });

    group.finish();
}

/// Benchmark high-throughput filtering scenario
fn bench_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");

    // Simulate realistic filter configuration
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(IdFilter::new(0x100))); // Engine RPM
    chain.add_filter(Box::new(IdFilter::new(0x200))); // Vehicle speed
    chain.add_filter(Box::new(RangeFilter::new(0x300, 0x3FF))); // Diagnostic range

    // Pre-generate messages
    let messages: Vec<CanMessage> = (0..1000)
        .map(|i| make_message((i % 0x7FF) as u16))
        .collect();

    group.throughput(Throughput::Elements(1000));
    group.bench_function("1000_messages", |b| {
        b.iter(|| {
            let mut matched = 0u32;
            for msg in &messages {
                if chain.matches(black_box(msg)) {
                    matched += 1;
                }
            }
            black_box(matched)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_id_filter_match,
    bench_range_filter_match,
    bench_filter_chain,
    bench_empty_chain,
    bench_mixed_filters,
    bench_throughput,
);

criterion_main!(benches);
