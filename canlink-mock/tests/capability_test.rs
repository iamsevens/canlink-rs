//! Mock backend capability configuration tests.
//!
//! Tests for configuring and querying mock backend capabilities.

use canlink_hal::{CanBackend, TimestampPrecision};
use canlink_mock::{MockBackend, MockConfig};

/// Test default capability configuration.
#[test]
fn test_default_capability_config() {
    let config = MockConfig::default();
    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 2);
    assert!(capability.supports_canfd);
    assert_eq!(capability.max_bitrate, 8_000_000);
    assert_eq!(capability.supported_bitrates.len(), 4);
    assert_eq!(capability.filter_count, 16);
    assert_eq!(
        capability.timestamp_precision,
        TimestampPrecision::Microsecond
    );
}

/// Test CAN 2.0 only configuration.
#[test]
fn test_can20_only_config() {
    let config = MockConfig::can20_only();
    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 1);
    assert!(!capability.supports_canfd);
    assert_eq!(capability.max_bitrate, 1_000_000);
    assert_eq!(capability.filter_count, 8);
    assert_eq!(
        capability.timestamp_precision,
        TimestampPrecision::Millisecond
    );
}

/// Test custom channel count configuration.
#[test]
fn test_custom_channel_count() {
    let mut config = MockConfig::new();
    config.channel_count = 8;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 8);
    for i in 0..8 {
        assert!(capability.has_channel(i));
    }
    assert!(!capability.has_channel(8));
}

/// Test custom bitrate configuration.
#[test]
fn test_custom_bitrate_config() {
    let mut config = MockConfig::new();
    config.max_bitrate = 5_000_000;
    config.supported_bitrates = vec![100_000, 250_000, 500_000, 1_000_000, 2_000_000];

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.max_bitrate, 5_000_000);
    assert_eq!(capability.supported_bitrates.len(), 5);
    assert!(capability.supports_bitrate(100_000));
    assert!(capability.supports_bitrate(2_000_000));
    assert!(!capability.supports_bitrate(5_000_000)); // Not in supported list
}

/// Test CAN-FD enable/disable configuration.
#[test]
fn test_canfd_configuration() {
    // CAN-FD enabled
    let mut config_fd = MockConfig::new();
    config_fd.supports_canfd = true;
    let backend_fd = MockBackend::with_config(config_fd);
    let cap_fd = backend_fd.get_capability().unwrap();
    assert!(cap_fd.supports_canfd);

    // CAN-FD disabled
    let mut config_20 = MockConfig::new();
    config_20.supports_canfd = false;
    let backend_20 = MockBackend::with_config(config_20);
    let cap_20 = backend_20.get_capability().unwrap();
    assert!(!cap_20.supports_canfd);
}

/// Test filter count configuration.
#[test]
fn test_filter_count_config() {
    let mut config = MockConfig::new();
    config.filter_count = 32;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.filter_count, 32);
}

/// Test timestamp precision configuration.
#[test]
fn test_timestamp_precision_config() {
    // Microsecond precision
    let mut config_us = MockConfig::new();
    config_us.timestamp_precision = TimestampPrecision::Microsecond;
    let backend_us = MockBackend::with_config(config_us);
    let cap_us = backend_us.get_capability().unwrap();
    assert_eq!(cap_us.timestamp_precision, TimestampPrecision::Microsecond);

    // Millisecond precision
    let mut config_ms = MockConfig::new();
    config_ms.timestamp_precision = TimestampPrecision::Millisecond;
    let backend_ms = MockBackend::with_config(config_ms);
    let cap_ms = backend_ms.get_capability().unwrap();
    assert_eq!(cap_ms.timestamp_precision, TimestampPrecision::Millisecond);

    // No timestamp support
    let mut config_none = MockConfig::new();
    config_none.timestamp_precision = TimestampPrecision::None;
    let backend_none = MockBackend::with_config(config_none);
    let cap_none = backend_none.get_capability().unwrap();
    assert_eq!(cap_none.timestamp_precision, TimestampPrecision::None);
}

/// Test minimal configuration (single channel, CAN 2.0).
#[test]
fn test_minimal_config() {
    let mut config = MockConfig::new();
    config.channel_count = 1;
    config.supports_canfd = false;
    config.max_bitrate = 500_000;
    config.supported_bitrates = vec![500_000];
    config.filter_count = 1;
    config.timestamp_precision = TimestampPrecision::None;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 1);
    assert!(!capability.supports_canfd);
    assert_eq!(capability.max_bitrate, 500_000);
    assert_eq!(capability.supported_bitrates.len(), 1);
    assert_eq!(capability.filter_count, 1);
    assert_eq!(capability.timestamp_precision, TimestampPrecision::None);
}

/// Test maximum configuration (many channels, CAN-FD, high bitrate).
#[test]
fn test_maximum_config() {
    let mut config = MockConfig::new();
    config.channel_count = 16;
    config.supports_canfd = true;
    config.max_bitrate = 10_000_000;
    config.supported_bitrates = vec![
        125_000, 250_000, 500_000, 1_000_000, 2_000_000, 5_000_000, 8_000_000, 10_000_000,
    ];
    config.filter_count = 64;
    config.timestamp_precision = TimestampPrecision::Microsecond;

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 16);
    assert!(capability.supports_canfd);
    assert_eq!(capability.max_bitrate, 10_000_000);
    assert_eq!(capability.supported_bitrates.len(), 8);
    assert_eq!(capability.filter_count, 64);
    assert_eq!(
        capability.timestamp_precision,
        TimestampPrecision::Microsecond
    );
}

/// Test configuration to capability conversion.
#[test]
fn test_config_to_capability_conversion() {
    let config = MockConfig::new();
    let capability = config.to_capability();

    assert_eq!(capability.channel_count, config.channel_count);
    assert_eq!(capability.supports_canfd, config.supports_canfd);
    assert_eq!(capability.max_bitrate, config.max_bitrate);
    assert_eq!(capability.supported_bitrates, config.supported_bitrates);
    assert_eq!(capability.filter_count, config.filter_count as u8);
    assert_eq!(capability.timestamp_precision, config.timestamp_precision);
}

/// Test multiple backends with different configurations.
#[test]
fn test_multiple_backends_different_configs() {
    let config1 = MockConfig::can20_only();
    let backend1 = MockBackend::with_config(config1);

    let config2 = MockConfig::default();
    let backend2 = MockBackend::with_config(config2);

    let cap1 = backend1.get_capability().unwrap();
    let cap2 = backend2.get_capability().unwrap();

    // Verify they have different capabilities
    assert_ne!(cap1.channel_count, cap2.channel_count);
    assert_ne!(cap1.supports_canfd, cap2.supports_canfd);
    assert_ne!(cap1.max_bitrate, cap2.max_bitrate);
}

/// Test configuration immutability after backend creation.
#[test]
fn test_config_immutability() {
    let mut config = MockConfig::new();
    config.channel_count = 4;

    let backend = MockBackend::with_config(config.clone());
    let cap1 = backend.get_capability().unwrap();

    // Modify the original config
    config.channel_count = 8;

    // Backend capability should not change
    let cap2 = backend.get_capability().unwrap();
    assert_eq!(cap1, cap2);
    assert_eq!(cap2.channel_count, 4); // Original value
}

/// Test realistic hardware configurations.
#[test]
fn test_realistic_hardware_configs() {
    // Simulate PEAK PCAN-USB (CAN 2.0)
    let mut peak_config = MockConfig::new();
    peak_config.channel_count = 1;
    peak_config.supports_canfd = false;
    peak_config.max_bitrate = 1_000_000;
    peak_config.supported_bitrates = vec![125_000, 250_000, 500_000, 1_000_000];
    peak_config.filter_count = 11;
    peak_config.timestamp_precision = TimestampPrecision::Microsecond;

    let peak_backend = MockBackend::with_config(peak_config);
    let peak_cap = peak_backend.get_capability().unwrap();
    assert_eq!(peak_cap.channel_count, 1);
    assert!(!peak_cap.supports_canfd);

    // Simulate Kvaser Leaf Pro HS v2 (CAN-FD)
    let mut kvaser_config = MockConfig::new();
    kvaser_config.channel_count = 1;
    kvaser_config.supports_canfd = true;
    kvaser_config.max_bitrate = 8_000_000;
    kvaser_config.supported_bitrates = vec![125_000, 250_000, 500_000, 1_000_000, 2_000_000];
    kvaser_config.filter_count = 16;
    kvaser_config.timestamp_precision = TimestampPrecision::Microsecond;

    let kvaser_backend = MockBackend::with_config(kvaser_config);
    let kvaser_cap = kvaser_backend.get_capability().unwrap();
    assert_eq!(kvaser_cap.channel_count, 1);
    assert!(kvaser_cap.supports_canfd);

    // Simulate multi-channel interface
    let mut multi_config = MockConfig::new();
    multi_config.channel_count = 4;
    multi_config.supports_canfd = true;
    multi_config.max_bitrate = 8_000_000;

    let multi_backend = MockBackend::with_config(multi_config);
    let multi_cap = multi_backend.get_capability().unwrap();
    assert_eq!(multi_cap.channel_count, 4);
}
