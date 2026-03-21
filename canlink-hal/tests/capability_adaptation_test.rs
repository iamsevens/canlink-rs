//! Capability adaptation integration tests.
//!
//! Tests for adapting application behavior based on hardware capabilities.

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

/// Test adaptive message type selection based on CAN-FD support.
#[test]
fn test_adaptive_message_type() {
    // Test with CAN-FD backend
    let backend_fd = MockBackend::new();
    let capability_fd = backend_fd.get_capability().unwrap();

    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let message = if capability_fd.supports_canfd {
        // Use CAN-FD for larger data
        CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap()
    } else {
        // Fall back to CAN 2.0 with truncated data
        CanMessage::new_standard(0x123, &data[..8]).unwrap()
    };

    assert_eq!(message.data().len(), 12); // CAN-FD allows 12 bytes

    // Test with CAN 2.0 only backend
    let config_20 = MockConfig::can20_only();
    let backend_20 = MockBackend::with_config(config_20);
    let capability_20 = backend_20.get_capability().unwrap();

    let message_20 = if capability_20.supports_canfd {
        CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap()
    } else {
        CanMessage::new_standard(0x123, &data[..8]).unwrap()
    };

    assert_eq!(message_20.data().len(), 8); // CAN 2.0 limited to 8 bytes
}

/// Test channel validation before opening.
#[test]
fn test_channel_validation_before_open() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    let capability = backend.get_capability().unwrap();

    // Try to open valid channels
    for channel in 0..capability.channel_count {
        if capability.has_channel(channel) {
            assert!(backend.open_channel(channel).is_ok());
        }
    }

    // Try to open invalid channel
    let invalid_channel = capability.channel_count;
    let result = backend.open_channel(invalid_channel);
    assert!(result.is_err());
}

/// Test bitrate selection based on supported bitrates.
#[test]
fn test_bitrate_selection() {
    let backend = MockBackend::new();
    let capability = backend.get_capability().unwrap();

    // Preferred bitrates in order of preference
    let preferred_bitrates = [1_000_000, 500_000, 250_000, 125_000];

    // Select the first supported bitrate
    let selected_bitrate = preferred_bitrates
        .iter()
        .find(|&&bitrate| capability.supports_bitrate(bitrate))
        .copied();

    assert_eq!(selected_bitrate, Some(1_000_000));

    // Test with limited bitrate support
    let mut config = MockConfig::new();
    config.supported_bitrates = vec![125_000, 250_000];
    let backend_limited = MockBackend::with_config(config);
    let capability_limited = backend_limited.get_capability().unwrap();

    let selected_limited = preferred_bitrates
        .iter()
        .find(|&&bitrate| capability_limited.supports_bitrate(bitrate))
        .copied();

    assert_eq!(selected_limited, Some(250_000));
}

/// Test adaptive data length based on CAN-FD support.
#[test]
fn test_adaptive_data_length() {
    fn get_max_data_length(backend: &MockBackend) -> usize {
        let capability = backend.get_capability().unwrap();
        if capability.supports_canfd {
            64 // CAN-FD max
        } else {
            8 // CAN 2.0 max
        }
    }

    // CAN-FD backend
    let backend_fd = MockBackend::new();
    assert_eq!(get_max_data_length(&backend_fd), 64);

    // CAN 2.0 backend
    let config_20 = MockConfig::can20_only();
    let backend_20 = MockBackend::with_config(config_20);
    assert_eq!(get_max_data_length(&backend_20), 8);
}

/// Test multi-channel application adaptation.
#[test]
fn test_multi_channel_adaptation() {
    // Single channel backend
    let config_single = MockConfig::can20_only();
    let mut backend_single = MockBackend::with_config(config_single);
    let config = BackendConfig::new("mock");
    backend_single.initialize(&config).unwrap();

    let capability_single = backend_single.get_capability().unwrap();
    assert_eq!(capability_single.channel_count, 1);

    // Open all available channels
    for channel in 0..capability_single.channel_count {
        backend_single.open_channel(channel).unwrap();
    }

    // Multi-channel backend
    let mut config_multi = MockConfig::new();
    config_multi.channel_count = 4;
    let mut backend_multi = MockBackend::with_config(config_multi);
    backend_multi.initialize(&config).unwrap();

    let capability_multi = backend_multi.get_capability().unwrap();
    assert_eq!(capability_multi.channel_count, 4);

    // Open all available channels
    for channel in 0..capability_multi.channel_count {
        backend_multi.open_channel(channel).unwrap();
    }
}

/// Test timestamp handling based on precision.
#[test]
fn test_timestamp_handling_adaptation() {
    fn should_use_timestamps(backend: &MockBackend) -> bool {
        let capability = backend.get_capability().unwrap();
        capability.timestamp_precision.is_supported()
    }

    // Backend with timestamp support
    let backend_ts = MockBackend::new();
    assert!(should_use_timestamps(&backend_ts));

    // Backend without timestamp support
    let mut config_no_ts = MockConfig::new();
    config_no_ts.timestamp_precision = canlink_hal::TimestampPrecision::None;
    let backend_no_ts = MockBackend::with_config(config_no_ts);
    assert!(!should_use_timestamps(&backend_no_ts));
}

/// Test filter allocation based on available filters.
#[test]
fn test_filter_allocation() {
    fn allocate_filters(backend: &MockBackend, requested: u8) -> u8 {
        let capability = backend.get_capability().unwrap();
        requested.min(capability.filter_count)
    }

    // Backend with many filters
    let backend_many = MockBackend::new();
    assert_eq!(allocate_filters(&backend_many, 20), 16); // Limited by hardware

    // Backend with few filters
    let config_few = MockConfig::can20_only();
    let backend_few = MockBackend::with_config(config_few);
    assert_eq!(allocate_filters(&backend_few, 20), 8); // Limited by hardware
    assert_eq!(allocate_filters(&backend_few, 5), 5); // Request fits
}

/// Test complete application adaptation workflow.
#[test]
fn test_complete_adaptation_workflow() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Step 1: Query capabilities
    let capability = backend.get_capability().unwrap();

    // Step 2: Validate and open channels
    let desired_channel = 0;
    assert!(capability.has_channel(desired_channel));
    backend.open_channel(desired_channel).unwrap();

    // Step 3: Select appropriate message type
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let message = if capability.supports_canfd {
        CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap()
    } else {
        CanMessage::new_standard(0x123, &data[..8]).unwrap()
    };

    // Step 4: Send message
    assert!(backend.send_message(&message).is_ok());

    // Step 5: Verify with recorder
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].id(), message.id());
}

/// Test graceful degradation when features are unavailable.
#[test]
fn test_graceful_degradation() {
    // Application wants CAN-FD but only CAN 2.0 is available
    let config_20 = MockConfig::can20_only();
    let mut backend = MockBackend::with_config(config_20);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    let capability = backend.get_capability().unwrap();

    // Large data that would fit in CAN-FD
    let large_data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

    // Gracefully degrade to CAN 2.0
    let message = if capability.supports_canfd {
        CanMessage::new_fd(CanId::Standard(0x123), &large_data).unwrap()
    } else {
        // Split into multiple CAN 2.0 messages or truncate
        CanMessage::new_standard(0x123, &large_data[..8]).unwrap()
    };

    assert!(backend.send_message(&message).is_ok());
    assert_eq!(message.data().len(), 8); // Truncated to CAN 2.0 limit
}

/// Test capability-based feature flags.
#[test]
fn test_capability_feature_flags() {
    struct AppConfig {
        use_canfd: bool,
        use_timestamps: bool,
        max_channels: u8,
    }

    fn create_app_config(backend: &MockBackend) -> AppConfig {
        let capability = backend.get_capability().unwrap();
        AppConfig {
            use_canfd: capability.supports_canfd,
            use_timestamps: capability.timestamp_precision.is_supported(),
            max_channels: capability.channel_count,
        }
    }

    // CAN-FD backend
    let backend_fd = MockBackend::new();
    let config_fd = create_app_config(&backend_fd);
    assert!(config_fd.use_canfd);
    assert!(config_fd.use_timestamps);
    assert_eq!(config_fd.max_channels, 2);

    // CAN 2.0 backend
    let mock_config_20 = MockConfig::can20_only();
    let backend_20 = MockBackend::with_config(mock_config_20);
    let config_20 = create_app_config(&backend_20);
    assert!(!config_20.use_canfd);
    assert!(config_20.use_timestamps); // Still has timestamps
    assert_eq!(config_20.max_channels, 1);
}

/// Test runtime capability switching between backends.
#[test]
fn test_runtime_backend_switching() {
    // Start with CAN 2.0 backend
    let config_20 = MockConfig::can20_only();
    let backend_20 = MockBackend::with_config(config_20);
    let cap_20 = backend_20.get_capability().unwrap();
    assert!(!cap_20.supports_canfd);

    // Switch to CAN-FD backend
    let backend_fd = MockBackend::new();
    let cap_fd = backend_fd.get_capability().unwrap();
    assert!(cap_fd.supports_canfd);

    // Application adapts to new capabilities
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // With CAN 2.0
    let msg_20 = if cap_20.supports_canfd {
        CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap()
    } else {
        CanMessage::new_standard(0x123, &data[..8]).unwrap()
    };
    assert_eq!(msg_20.data().len(), 8);

    // With CAN-FD
    let msg_fd = if cap_fd.supports_canfd {
        CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap()
    } else {
        CanMessage::new_standard(0x123, &data[..8]).unwrap()
    };
    assert_eq!(msg_fd.data().len(), 10);
}
