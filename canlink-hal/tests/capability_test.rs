//! Capability query tests.
//!
//! Tests for hardware capability query functionality, including performance requirements.

use canlink_hal::{BackendConfig, CanBackend, TimestampPrecision};
use canlink_mock::{MockBackend, MockConfig};
use std::time::Instant;

/// Test basic capability query.
#[test]
fn test_basic_capability_query() {
    let backend = MockBackend::new();
    let capability = backend.get_capability().unwrap();

    // Verify default capabilities
    assert_eq!(capability.channel_count, 2);
    assert!(capability.supports_canfd);
    assert_eq!(capability.max_bitrate, 8_000_000);
    assert_eq!(capability.filter_count, 16);
    assert_eq!(
        capability.timestamp_precision,
        TimestampPrecision::Microsecond
    );
}

/// Test capability query with custom configuration.
#[test]
fn test_custom_capability_query() {
    let config = MockConfig::can20_only();
    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    // Verify CAN 2.0 only configuration
    assert_eq!(capability.channel_count, 1);
    assert!(!capability.supports_canfd);
    assert_eq!(capability.max_bitrate, 1_000_000);
    assert_eq!(capability.filter_count, 8);
    assert_eq!(
        capability.timestamp_precision,
        TimestampPrecision::Millisecond
    );
}

/// Test capability query performance (SC-004: < 1ms).
#[test]
fn test_capability_query_performance() {
    let backend = MockBackend::new();

    // Warm up
    let _ = backend.get_capability();

    // Measure performance over multiple queries
    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let _ = backend.get_capability().unwrap();
    }

    let elapsed = start.elapsed();
    let avg_time_us = elapsed.as_micros() / iterations;

    println!("Average capability query time: {} µs", avg_time_us);

    // SC-004: Response time < 1ms (1000 µs)
    assert!(
        avg_time_us < 1000,
        "Capability query took {} µs, expected < 1000 µs",
        avg_time_us
    );
}

/// Test capability query from different states.
#[test]
fn test_capability_query_from_different_states() {
    let mut backend = MockBackend::new();

    // Query from Uninitialized state
    let cap1 = backend.get_capability().unwrap();
    assert_eq!(cap1.channel_count, 2);

    // Query from Ready state
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    let cap2 = backend.get_capability().unwrap();
    assert_eq!(cap2.channel_count, 2);

    // Query from Closed state
    backend.close().unwrap();
    let cap3 = backend.get_capability().unwrap();
    assert_eq!(cap3.channel_count, 2);

    // All queries should return the same capability
    assert_eq!(cap1, cap2);
    assert_eq!(cap2, cap3);
}

/// Test bitrate support checking.
#[test]
fn test_bitrate_support() {
    let backend = MockBackend::new();
    let capability = backend.get_capability().unwrap();

    // Check supported bitrates
    assert!(capability.supports_bitrate(125_000));
    assert!(capability.supports_bitrate(250_000));
    assert!(capability.supports_bitrate(500_000));
    assert!(capability.supports_bitrate(1_000_000));

    // Check unsupported bitrates
    assert!(!capability.supports_bitrate(100_000));
    assert!(!capability.supports_bitrate(2_000_000));
}

/// Test channel validation.
#[test]
fn test_channel_validation() {
    let backend = MockBackend::new();
    let capability = backend.get_capability().unwrap();

    // Valid channels
    assert!(capability.has_channel(0));
    assert!(capability.has_channel(1));

    // Invalid channels
    assert!(!capability.has_channel(2));
    assert!(!capability.has_channel(255));
}

/// Test CAN-FD support detection.
#[test]
fn test_canfd_support_detection() {
    // Backend with CAN-FD support
    let backend_fd = MockBackend::new();
    let cap_fd = backend_fd.get_capability().unwrap();
    assert!(cap_fd.supports_canfd);

    // Backend without CAN-FD support
    let config_20 = MockConfig::can20_only();
    let backend_20 = MockBackend::with_config(config_20);
    let cap_20 = backend_20.get_capability().unwrap();
    assert!(!cap_20.supports_canfd);
}

/// Test timestamp precision detection.
#[test]
fn test_timestamp_precision() {
    // Microsecond precision
    let backend_us = MockBackend::new();
    let cap_us = backend_us.get_capability().unwrap();
    assert_eq!(cap_us.timestamp_precision, TimestampPrecision::Microsecond);
    assert_eq!(cap_us.timestamp_precision.resolution_us(), Some(1));

    // Millisecond precision
    let config_ms = MockConfig::can20_only();
    let backend_ms = MockBackend::with_config(config_ms);
    let cap_ms = backend_ms.get_capability().unwrap();
    assert_eq!(cap_ms.timestamp_precision, TimestampPrecision::Millisecond);
    assert_eq!(cap_ms.timestamp_precision.resolution_us(), Some(1000));
}

/// Test capability query consistency.
#[test]
fn test_capability_consistency() {
    let backend = MockBackend::new();

    // Query multiple times
    let cap1 = backend.get_capability().unwrap();
    let cap2 = backend.get_capability().unwrap();
    let cap3 = backend.get_capability().unwrap();

    // All queries should return identical capabilities
    assert_eq!(cap1, cap2);
    assert_eq!(cap2, cap3);
}

/// Test capability with custom bitrates.
#[test]
fn test_custom_bitrates() {
    let mut config = MockConfig::new();
    config.supported_bitrates = vec![100_000, 200_000, 400_000, 800_000];
    config.max_bitrate = 800_000;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.max_bitrate, 800_000);
    assert!(capability.supports_bitrate(100_000));
    assert!(capability.supports_bitrate(800_000));
    assert!(!capability.supports_bitrate(1_000_000));
}

/// Test capability with multiple channels.
#[test]
fn test_multiple_channels() {
    let mut config = MockConfig::new();
    config.channel_count = 4;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 4);
    assert!(capability.has_channel(0));
    assert!(capability.has_channel(1));
    assert!(capability.has_channel(2));
    assert!(capability.has_channel(3));
    assert!(!capability.has_channel(4));
}

/// Test capability with no timestamp support.
#[test]
fn test_no_timestamp_support() {
    let mut config = MockConfig::new();
    config.timestamp_precision = TimestampPrecision::None;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.timestamp_precision, TimestampPrecision::None);
    assert_eq!(capability.timestamp_precision.resolution_us(), None);
    assert!(!capability.timestamp_precision.is_supported());
}
