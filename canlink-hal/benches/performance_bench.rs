//! Performance benchmarks for CAN hardware abstraction layer.
//!
//! This benchmark verifies SC-005: Abstraction layer overhead < 5%
//!
//! ## Test Environment Requirements (from spec.md)
//!
//! - CPU: Modern x86_64 processor (≥ 2.0 GHz, ≥ 4 cores)
//! - Memory: ≥ 8 GB RAM
//! - OS: Windows 10/11 or Linux (kernel ≥ 5.0)
//! - Compile mode: Release mode (`cargo bench`)
//!
//! ## Benchmark Scenarios
//!
//! - Scenario 1: Send 1000 messages via abstraction layer (CanBackend trait)
//! - Scenario 2: Direct backend operations (baseline)
//!
//! ## Success Criteria
//!
//! - Abstraction overhead = (Scenario1 - Scenario2) / Scenario2 < 5%

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Create a mock config that limits message recording to prevent memory issues.
fn create_limited_mock_config() -> MockConfig {
    MockConfig {
        max_recorded_messages: 100, // Limit to prevent memory growth
        ..MockConfig::default()
    }
}

/// Benchmark message sending through the abstraction layer.
///
/// This measures the time to send messages through the CanBackend trait,
/// which is the primary interface for all hardware backends.
fn bench_send_message_via_trait(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_send");
    group.throughput(Throughput::Elements(1));

    // Create and initialize mock backend with limited recording
    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Create test message: fixed ID (0x123), 8 bytes data (0x01-0x08)
    let msg =
        CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap();

    group.bench_function("single_message", |b| {
        b.iter(|| {
            backend.send_message(black_box(&msg)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark batch message sending (1000 messages).
///
/// This is the primary benchmark for SC-005 verification.
fn bench_send_1000_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_batch");
    group.throughput(Throughput::Elements(1000));

    // Create and initialize mock backend with limited recording
    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Pre-create 1000 messages
    let messages: Vec<CanMessage> = (0..1000)
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
        .collect();

    group.bench_function("1000_messages_via_trait", |b| {
        b.iter(|| {
            for msg in &messages {
                backend.send_message(black_box(msg)).unwrap();
            }
        });
    });

    group.finish();
}

/// Benchmark message receiving through the abstraction layer.
fn bench_receive_message_via_trait(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_receive");
    group.throughput(Throughput::Elements(1));

    // Create preset messages for receiving (smaller set to avoid memory issues)
    let preset_messages: Vec<CanMessage> = (0..1000)
        .map(|i| {
            CanMessage::new_standard(
                (0x100 + (i % 0x700)) as u16,
                &[i as u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],
            )
            .unwrap()
        })
        .collect();

    // Create mock backend with preset messages
    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    group.bench_function("single_message", |b| {
        b.iter(|| {
            let _ = black_box(backend.receive_message().unwrap());
        });
    });

    group.finish();
}

/// Benchmark CAN-FD message operations.
fn bench_canfd_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("canfd");
    group.throughput(Throughput::Elements(1));

    let mock_config = create_limited_mock_config();
    let mut backend = MockBackend::with_config(mock_config);
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Create CAN-FD message with 64 bytes
    let fd_data: Vec<u8> = (0..64).collect();
    let fd_msg = CanMessage::new_fd(CanId::Standard(0x123), &fd_data).unwrap();

    group.bench_function("send_fd_64bytes", |b| {
        b.iter(|| {
            backend.send_message(black_box(&fd_msg)).unwrap();
        });
    });

    // Compare with standard CAN message
    let std_msg =
        CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]).unwrap();

    group.bench_function("send_std_8bytes", |b| {
        b.iter(|| {
            backend.send_message(black_box(&std_msg)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark message creation overhead.
///
/// This measures the cost of creating CanMessage instances,
/// which is part of the abstraction layer overhead.
fn bench_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_creation");

    group.bench_function("new_standard", |b| {
        b.iter(|| {
            let msg = CanMessage::new_standard(
                black_box(0x123),
                black_box(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
            )
            .unwrap();
            black_box(msg)
        });
    });

    group.bench_function("new_extended", |b| {
        b.iter(|| {
            let msg = CanMessage::new_extended(
                black_box(0x12345678),
                black_box(&[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]),
            )
            .unwrap();
            black_box(msg)
        });
    });

    let fd_data: Vec<u8> = (0..64).collect();
    group.bench_function("new_fd_64bytes", |b| {
        b.iter(|| {
            let msg =
                CanMessage::new_fd(black_box(CanId::Standard(0x123)), black_box(&fd_data)).unwrap();
            black_box(msg)
        });
    });

    group.finish();
}

/// Benchmark varying message sizes.
fn bench_message_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_sizes");

    // Use separate backends for each size to avoid memory accumulation
    for size in [0, 1, 4, 8].iter() {
        let mock_config = create_limited_mock_config();
        let mut backend = MockBackend::with_config(mock_config);
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();

        let data: Vec<u8> = (0..*size as u8).collect();
        let msg = CanMessage::new_standard(0x123, &data).unwrap();

        group.bench_with_input(BenchmarkId::new("send", size), size, |b, _| {
            b.iter(|| {
                backend.send_message(black_box(&msg)).unwrap();
            });
        });
    }

    // CAN-FD sizes
    for size in [12, 16, 20, 24, 32, 48, 64].iter() {
        let mock_config = create_limited_mock_config();
        let mut backend = MockBackend::with_config(mock_config);
        let config = BackendConfig::new("mock");
        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();

        let data: Vec<u8> = (0..*size as u8).collect();
        let msg = CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap();

        group.bench_with_input(BenchmarkId::new("send_fd", size), size, |b, _| {
            b.iter(|| {
                backend.send_message(black_box(&msg)).unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_send_message_via_trait,
    bench_send_1000_messages,
    bench_receive_message_via_trait,
    bench_canfd_messages,
    bench_message_creation,
    bench_message_sizes,
);

criterion_main!(benches);
