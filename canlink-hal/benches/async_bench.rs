//! Async API performance benchmarks.
//!
//! This benchmark verifies SC-001: Async throughput / Sync throughput >= 0.95
//!
//! ## Test Environment Requirements (from spec.md)
//!
//! - CPU: Modern x86_64 processor (>= 2.0 GHz, >= 4 cores)
//! - Memory: >= 8 GB RAM
//! - OS: Windows 10/11 or Linux (kernel >= 5.0)
//! - Compile mode: Release mode (`cargo bench`)
//!
//! ## Benchmark Scenarios
//!
//! - Scenario 1: Send 1000 messages via sync API
//! - Scenario 2: Send 1000 messages via async API
//!
//! ## Success Criteria
//!
//! - Async throughput / Sync throughput >= 0.95 (95%)

use canlink_hal::{BackendConfig, CanBackend, CanBackendAsync, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

/// Create a mock config that limits message recording to prevent memory issues.
fn create_limited_mock_config() -> MockConfig {
    MockConfig {
        max_recorded_messages: 100,
        ..MockConfig::default()
    }
}

/// Pre-create test messages.
fn create_test_messages(count: usize) -> Vec<CanMessage> {
    (0..count)
        .map(|i| {
            CanMessage::new_standard(
                (0x100 + (i % 0x700)) as u16,
                &[
                    (i & 0xFF) as u8,
                    ((i >> 8) & 0xFF) as u8,
                    0x03,
                    0x04,
                    0x05,
                    0x06,
                    0x07,
                    0x08,
                ],
            )
            .unwrap()
        })
        .collect()
}

fn create_backend_with_messages(messages: Vec<CanMessage>) -> MockBackend {
    let config = MockConfig::with_preset_messages(messages);
    let mut backend = MockBackend::with_config(config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();
    backend
}

/// Benchmark synchronous message sending (baseline).
fn bench_sync_send(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_comparison");
    group.throughput(Throughput::Elements(1000));

    // Create and initialize mock backend
    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Pre-create 1000 messages
    let messages = create_test_messages(1000);

    group.bench_function("sync_1000_messages", |b| {
        b.iter(|| {
            for msg in &messages {
                backend.send_message(black_box(msg)).unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark asynchronous message sending.
fn bench_async_send(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_comparison");
    group.throughput(Throughput::Elements(1000));

    // Create tokio runtime for async benchmarks
    let rt = Runtime::new().unwrap();

    // Create and initialize mock backend
    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Pre-create 1000 messages
    let messages = create_test_messages(1000);

    // Use async Mutex to allow mutable access in async context
    let backend = Mutex::new(backend);

    group.bench_function("async_1000_messages", |b| {
        b.to_async(&rt).iter(|| {
            let backend = &backend;
            let messages = &messages;
            async move {
                for msg in messages {
                    let mut backend = backend.lock().await;
                    backend.send_message_async(black_box(msg)).await.unwrap();
                }
            }
        });
    });

    group.finish();
}

/// Benchmark single message sync vs async.
fn bench_single_message_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_message_comparison");
    group.throughput(Throughput::Elements(1));

    let rt = Runtime::new().unwrap();

    // Sync backend
    let mock_config = create_limited_mock_config();
    let mut sync_backend = MockBackend::with_config(mock_config.clone());
    let config = BackendConfig::new("mock");
    sync_backend.initialize(&config).unwrap();
    sync_backend.open_channel(0).unwrap();

    // Async backend (same instance, different API)
    let mut async_backend = MockBackend::with_config(mock_config);
    async_backend.initialize(&config).unwrap();
    async_backend.open_channel(0).unwrap();

    let msg =
        CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap();

    group.bench_function("sync_single", |b| {
        b.iter(|| {
            sync_backend.send_message(black_box(&msg)).unwrap();
        });
    });

    // Use async Mutex for async access
    let async_backend = Mutex::new(async_backend);

    group.bench_function("async_single", |b| {
        b.to_async(&rt).iter(|| {
            let backend = &async_backend;
            let msg = &msg;
            async move {
                let mut backend = backend.lock().await;
                backend.send_message_async(black_box(msg)).await.unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark receive operations sync vs async.
fn bench_receive_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("receive_comparison");
    group.throughput(Throughput::Elements(1));

    let rt = Runtime::new().unwrap();

    // Create preset messages for receiving
    let preset_messages: Vec<CanMessage> = (0..10000)
        .map(|i| {
            CanMessage::new_standard(
                (0x100 + (i % 0x700)) as u16,
                &[i as u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            )
            .unwrap()
        })
        .collect();

    // Sync backend with preset messages
    let sync_config = MockConfig::with_preset_messages(preset_messages.clone());
    let mut sync_backend = MockBackend::with_config(sync_config);
    let config = BackendConfig::new("mock");
    sync_backend.initialize(&config).unwrap();
    sync_backend.open_channel(0).unwrap();

    // Async backend with preset messages
    let async_config = MockConfig::with_preset_messages(preset_messages);
    let mut async_backend = MockBackend::with_config(async_config);
    async_backend.initialize(&config).unwrap();
    async_backend.open_channel(0).unwrap();

    group.bench_function("sync_receive", |b| {
        b.iter(|| {
            let _ = black_box(sync_backend.receive_message().unwrap());
        });
    });

    // Use async Mutex for async access
    let async_backend = Mutex::new(async_backend);

    group.bench_function("async_receive", |b| {
        b.to_async(&rt).iter(|| {
            let backend = &async_backend;
            async move {
                let _ = black_box(
                    backend
                        .lock()
                        .await
                        .receive_message_async(None)
                        .await
                        .unwrap(),
                );
            }
        });
    });

    group.finish();
}

/// Benchmark sustained send + receive throughput (sync).
fn bench_sustained_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_throughput");
    group.throughput(Throughput::Elements(2000));

    let messages = create_test_messages(1000);

    group.bench_function("sync_send_receive_1000", |b| {
        b.iter_batched(
            || create_backend_with_messages(messages.clone()),
            |mut backend| {
                for msg in &messages {
                    backend.send_message(black_box(msg)).unwrap();
                }
                for _ in 0..1000 {
                    let _ = backend.receive_message().unwrap();
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark 10k sync message sends.
fn bench_sync_send_10k(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_10k");
    group.throughput(Throughput::Elements(10_000));

    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    let messages = create_test_messages(10_000);

    group.bench_function("sync_10k_messages", |b| {
        b.iter(|| {
            for msg in &messages {
                backend.send_message(black_box(msg)).unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark 10k async message sends.
fn bench_async_send_10k(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_10k");
    group.throughput(Throughput::Elements(10_000));

    let rt = Runtime::new().unwrap();
    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    let messages = create_test_messages(10_000);
    let backend = Mutex::new(backend);

    group.bench_function("async_10k_messages", |b| {
        b.to_async(&rt).iter(|| {
            let backend = &backend;
            let messages = &messages;
            async move {
                for msg in messages {
                    let mut backend = backend.lock().await;
                    backend.send_message_async(black_box(msg)).await.unwrap();
                }
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sync_send,
    bench_async_send,
    bench_single_message_comparison,
    bench_receive_comparison,
    bench_sustained_throughput,
    bench_sync_send_10k,
    bench_async_send_10k,
);

criterion_main!(benches);
