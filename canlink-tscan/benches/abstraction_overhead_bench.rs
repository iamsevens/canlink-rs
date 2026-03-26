//! SC-005 Abstraction Layer Overhead Benchmark
//!
//! This benchmark verifies that the abstraction layer overhead is < 5% compared
//! to direct structure operations.
//!
//! ## Test Environment
//!
//! As specified in spec.md SC-005:
//! - CPU: Modern x86_64 processor (≥ 2.0 GHz, ≥ 4 cores)
//! - Memory: ≥ 8 GB RAM
//! - OS: Windows 10/11 (x64, verified; Linux/macOS not verified)
//! - Compilation: Release mode (`cargo bench`)
//!
//! ## Running Benchmarks
//!
//! ```bash
//! cargo bench -p canlink-tscan --bench abstraction_overhead_bench
//! ```
//!
//! ## Benchmark Scenarios
//!
//! This benchmark measures the overhead of message conversion:
//!
//! - **Scenario 1**: Convert CanMessage to TLIBCAN (abstraction layer)
//! - **Scenario 2**: Direct TLIBCAN structure creation (baseline)
//!
//! ## Success Criteria
//!
//! Overhead = (Scenario1_time - Scenario2_time) / Scenario2_time < 5%
//!
//! ## Note on Hardware Testing
//!
//! For complete SC-005 verification with actual hardware message transmission:
//! 1. Connect validated `LibTSCAN`-compatible hardware (currently TOSUN-related devices in this repository)
//! 2. The actual send overhead will be measured with real hardware

use canlink_hal::{CanId, CanMessage};
use canlink_tscan_sys::*;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Test message data: 8 bytes (0x01-0x08)
const TEST_DATA: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
const TEST_ID: u32 = 0x123;

/// Helper function to convert CanMessage to TLIBCAN (simulates abstraction layer)
fn convert_to_tlibcan(msg: &CanMessage, channel: u8) -> TLIBCAN {
    let identifier = match msg.id() {
        CanId::Standard(id) => id as i32,
        CanId::Extended(id) => id as i32,
    };

    let properties = if msg.id().is_extended() { 0x04 } else { 0x00 }
        | if msg.is_remote() { 0x02 } else { 0x00 };

    let mut data = [0u8; 8];
    let len = msg.data().len().min(8);
    data[..len].copy_from_slice(&msg.data()[..len]);

    TLIBCAN {
        FIdxChn: channel,
        FProperties: properties,
        FDLC: len as u8,
        FReserved: 0,
        FIdentifier: identifier,
        FTimeUs: 0,
        FData: data,
    }
}

/// Helper function to create TLIBCAN directly (baseline)
fn create_tlibcan_direct() -> TLIBCAN {
    TLIBCAN {
        FIdxChn: 0,
        FProperties: 0,
        FDLC: 8,
        FReserved: 0,
        FIdentifier: TEST_ID as i32,
        FTimeUs: 0,
        FData: TEST_DATA,
    }
}

/// Benchmark single message conversion (abstraction layer)
fn bench_abstraction_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("sc005_conversion");

    // Pre-create the message to isolate conversion overhead
    let msg = CanMessage::new_standard(TEST_ID as u16, &TEST_DATA).unwrap();

    group.bench_function("abstraction_layer", |b| {
        b.iter(|| {
            let tlibcan = convert_to_tlibcan(black_box(&msg), black_box(0));
            black_box(tlibcan)
        });
    });

    group.bench_function("direct_creation", |b| {
        b.iter(|| {
            let tlibcan = create_tlibcan_direct();
            black_box(tlibcan)
        });
    });

    group.finish();
}

/// Benchmark overhead comparison
fn bench_overhead_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("sc005_overhead_analysis");
    group.sample_size(1000);

    // Pre-create message
    let msg = CanMessage::new_standard(TEST_ID as u16, &TEST_DATA).unwrap();

    group.bench_function("measure_overhead", |b| {
        b.iter(|| {
            // Measure abstraction layer
            let start_abs = std::time::Instant::now();
            for _ in 0..1000 {
                let tlibcan = convert_to_tlibcan(&msg, 0);
                black_box(tlibcan);
            }
            let abstraction_time = start_abs.elapsed();

            // Measure direct creation
            let start_direct = std::time::Instant::now();
            for _ in 0..1000 {
                let tlibcan = create_tlibcan_direct();
                black_box(tlibcan);
            }
            let direct_time = start_direct.elapsed();

            // Calculate overhead
            let overhead_nanos =
                abstraction_time.as_nanos() as i128 - direct_time.as_nanos() as i128;
            let overhead_percent = if direct_time.as_nanos() > 0 {
                (overhead_nanos as f64 / direct_time.as_nanos() as f64) * 100.0
            } else {
                0.0
            };

            black_box((abstraction_time, direct_time, overhead_percent))
        });
    });

    group.finish();
}

/// Benchmark different message types
fn bench_message_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("sc005_message_types");

    // Standard ID
    let msg_std = CanMessage::new_standard(0x123, &TEST_DATA).unwrap();
    group.bench_function("standard_id", |b| {
        b.iter(|| {
            let tlibcan = convert_to_tlibcan(black_box(&msg_std), 0);
            black_box(tlibcan)
        });
    });

    // Extended ID
    let msg_ext = CanMessage::new_extended(0x12345678, &TEST_DATA).unwrap();
    group.bench_function("extended_id", |b| {
        b.iter(|| {
            let tlibcan = convert_to_tlibcan(black_box(&msg_ext), 0);
            black_box(tlibcan)
        });
    });

    // Remote frame
    let msg_remote = CanMessage::new_remote(CanId::Standard(0x123), 8).unwrap();
    group.bench_function("remote_frame", |b| {
        b.iter(|| {
            let tlibcan = convert_to_tlibcan(black_box(&msg_remote), 0);
            black_box(tlibcan)
        });
    });

    // Different data lengths
    for len in [1, 4, 8].iter() {
        let data = vec![0x42; *len];
        let msg = CanMessage::new_standard(0x123, &data).unwrap();
        group.bench_function(format!("{}_bytes", len), |b| {
            b.iter(|| {
                let tlibcan = convert_to_tlibcan(black_box(&msg), 0);
                black_box(tlibcan)
            });
        });
    }

    group.finish();
}

/// Final overhead report
fn bench_final_report(c: &mut Criterion) {
    let mut group = c.benchmark_group("sc005_final_report");
    group.sample_size(10);

    let msg = CanMessage::new_standard(TEST_ID as u16, &TEST_DATA).unwrap();

    group.bench_function("overhead_report", |b| {
        b.iter(|| {
            const ITERATIONS: usize = 10000;

            // Measure abstraction layer
            let start_abs = std::time::Instant::now();
            for _ in 0..ITERATIONS {
                let tlibcan = convert_to_tlibcan(&msg, 0);
                black_box(tlibcan);
            }
            let abstraction_time = start_abs.elapsed();

            // Measure direct creation
            let start_direct = std::time::Instant::now();
            for _ in 0..ITERATIONS {
                let tlibcan = create_tlibcan_direct();
                black_box(tlibcan);
            }
            let direct_time = start_direct.elapsed();

            // Calculate overhead
            let overhead_nanos =
                abstraction_time.as_nanos() as i128 - direct_time.as_nanos() as i128;
            let overhead_percent = if direct_time.as_nanos() > 0 {
                (overhead_nanos as f64 / direct_time.as_nanos() as f64) * 100.0
            } else {
                0.0
            };

            // Print report
            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║          SC-005 Abstraction Overhead Report             ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!(
                "║ Iterations:           {:>10}                       ║",
                ITERATIONS
            );
            println!(
                "║ Abstraction time:     {:>10.2?}                     ║",
                abstraction_time
            );
            println!(
                "║ Direct time:          {:>10.2?}                     ║",
                direct_time
            );
            println!(
                "║ Per-operation (abs):  {:>10.2?}                     ║",
                abstraction_time / ITERATIONS as u32
            );
            println!(
                "║ Per-operation (dir):  {:>10.2?}                     ║",
                direct_time / ITERATIONS as u32
            );
            println!(
                "║ Overhead:             {:>9.2}%                      ║",
                overhead_percent
            );
            println!(
                "║ Target:               {:>9}%                       ║",
                "< 5.00"
            );
            println!(
                "║ Status:               {:>10}                       ║",
                if overhead_percent < 5.0 {
                    "✅ PASS"
                } else {
                    "❌ FAIL"
                }
            );
            println!("╚══════════════════════════════════════════════════════════╝");
            println!("\nNote: This measures conversion overhead only.");
            println!("For complete hardware testing with actual message transmission,");
            println!("connect TSMaster device and run hardware integration tests.");

            black_box((abstraction_time, direct_time, overhead_percent))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_abstraction_conversion,
    bench_overhead_analysis,
    bench_message_types,
    bench_final_report,
);

criterion_main!(benches);
