//! Capability query benchmarks for CAN hardware abstraction layer.
//!
//! This benchmark verifies SC-004: Capability query response time < 1ms
//!
//! ## Test Environment Requirements (from spec.md)
//!
//! - CPU: Modern x86_64 processor (≥ 2.0 GHz, ≥ 4 cores)
//! - Memory: ≥ 8 GB RAM
//! - OS: Windows 10/11 or Linux (kernel ≥ 5.0)
//! - Compile mode: Release mode (`cargo bench`)
//!
//! ## Success Criteria
//!
//! - Average response time < 1ms
//! - 99th percentile < 2ms

use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::MockBackend;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Benchmark capability query through the CanBackend trait.
///
/// This is the primary benchmark for SC-004 verification.
/// The spec requires:
/// - Average response time < 1ms
/// - 99th percentile < 2ms
fn bench_get_capability(c: &mut Criterion) {
    let mut group = c.benchmark_group("capability_query");

    // Create and initialize mock backend
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    group.bench_function("get_capability", |b| {
        b.iter(|| {
            let cap = backend.get_capability().unwrap();
            black_box(cap)
        });
    });

    group.finish();
}

/// Benchmark repeated capability queries (1000 times).
///
/// This simulates a scenario where an application frequently
/// checks hardware capabilities.
fn bench_capability_1000_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("capability_batch");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    group.bench_function("1000_queries", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let cap = backend.get_capability().unwrap();
                black_box(cap);
            }
        });
    });

    group.finish();
}

/// Benchmark capability field access patterns.
///
/// This measures the cost of accessing individual capability fields
/// after querying.
fn bench_capability_field_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("capability_fields");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Get capability once
    let capability = backend.get_capability().unwrap();

    group.bench_function("access_channel_count", |b| {
        b.iter(|| {
            let count = black_box(&capability).channel_count;
            black_box(count)
        });
    });

    group.bench_function("access_supports_canfd", |b| {
        b.iter(|| {
            let supports = black_box(&capability).supports_canfd;
            black_box(supports)
        });
    });

    group.bench_function("access_max_bitrate", |b| {
        b.iter(|| {
            let bitrate = black_box(&capability).max_bitrate;
            black_box(bitrate)
        });
    });

    group.bench_function("access_all_fields", |b| {
        b.iter(|| {
            let cap = black_box(&capability);
            black_box(cap.channel_count);
            black_box(cap.supports_canfd);
            black_box(cap.max_bitrate);
            black_box(cap.filter_count);
            black_box(&cap.timestamp_precision);
        });
    });

    group.finish();
}

/// Benchmark capability query with helper methods.
fn bench_capability_helper_methods(c: &mut Criterion) {
    let mut group = c.benchmark_group("capability_helpers");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    let capability = backend.get_capability().unwrap();

    group.bench_function("has_channel", |b| {
        b.iter(|| {
            let has = capability.has_channel(black_box(0));
            black_box(has)
        });
    });

    group.bench_function("supports_bitrate", |b| {
        b.iter(|| {
            let supports = capability.supports_bitrate(black_box(500_000));
            black_box(supports)
        });
    });

    group.finish();
}

/// Benchmark backend version query.
fn bench_version_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("version_query");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    group.bench_function("version", |b| {
        b.iter(|| {
            let version = backend.version();
            black_box(version)
        });
    });

    group.bench_function("name", |b| {
        b.iter(|| {
            let name = backend.name();
            black_box(name)
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_get_capability,
    bench_capability_1000_queries,
    bench_capability_field_access,
    bench_capability_helper_methods,
    bench_version_query,
);

criterion_main!(benches);
