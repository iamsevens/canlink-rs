//! Mock backend integration tests.
//!
//! These tests verify the Mock backend's specific features like message recording,
//! preset messages, and error injection.

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

/// Test message recording functionality.
#[test]
fn test_mock_message_recording() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send multiple messages
    let messages = vec![
        CanMessage::new_standard(0x100, &[1, 2, 3]).unwrap(),
        CanMessage::new_standard(0x200, &[4, 5, 6]).unwrap(),
        CanMessage::new_extended(0x12345678, &[7, 8, 9]).unwrap(),
    ];

    for msg in &messages {
        backend.send_message(msg).unwrap();
    }

    // Verify all messages were recorded
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 3);

    // Verify message IDs
    assert_eq!(recorded[0].id(), CanId::Standard(0x100));
    assert_eq!(recorded[1].id(), CanId::Standard(0x200));
    assert_eq!(recorded[2].id(), CanId::Extended(0x12345678));

    // Verify message data
    assert_eq!(recorded[0].data(), &[1, 2, 3]);
    assert_eq!(recorded[1].data(), &[4, 5, 6]);
    assert_eq!(recorded[2].data(), &[7, 8, 9]);
}

/// Test clearing recorded messages.
#[test]
fn test_mock_clear_recorded_messages() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send messages
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    backend.send_message(&msg).unwrap();
    backend.send_message(&msg).unwrap();

    assert_eq!(backend.get_recorded_messages().len(), 2);

    // Clear messages
    backend.clear_recorded_messages();
    assert_eq!(backend.get_recorded_messages().len(), 0);

    // Send more messages
    backend.send_message(&msg).unwrap();
    assert_eq!(backend.get_recorded_messages().len(), 1);
}

/// Test preset messages functionality.
#[test]
fn test_mock_preset_messages() {
    let preset = vec![
        CanMessage::new_standard(0x111, &[0x11]).unwrap(),
        CanMessage::new_standard(0x222, &[0x22]).unwrap(),
        CanMessage::new_standard(0x333, &[0x33]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Receive all preset messages
    let msg1 = backend.receive_message().unwrap();
    assert!(msg1.is_some());
    assert_eq!(msg1.unwrap().id(), CanId::Standard(0x111));

    let msg2 = backend.receive_message().unwrap();
    assert!(msg2.is_some());
    assert_eq!(msg2.unwrap().id(), CanId::Standard(0x222));

    let msg3 = backend.receive_message().unwrap();
    assert!(msg3.is_some());
    assert_eq!(msg3.unwrap().id(), CanId::Standard(0x333));

    // No more messages
    let msg4 = backend.receive_message().unwrap();
    assert!(msg4.is_none());
}

/// Test error injection - initialization failure.
#[test]
fn test_mock_initialization_failure() {
    let config = MockConfig {
        fail_initialization: true,
        ..Default::default()
    };

    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");

    let result = backend.initialize(&backend_config);
    assert!(result.is_err());
}

/// Test error injection - send failure.
#[test]
fn test_mock_send_failure() {
    let config = MockConfig {
        fail_send: true,
        ..Default::default()
    };

    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    let result = backend.send_message(&msg);
    assert!(result.is_err());
}

/// Test error injection - receive failure.
#[test]
fn test_mock_receive_failure() {
    let config = MockConfig {
        fail_receive: true,
        ..Default::default()
    };

    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    let result = backend.receive_message();
    assert!(result.is_err());
}

/// Test CAN 2.0 only configuration.
#[test]
fn test_mock_can20_only() {
    let config = MockConfig::can20_only();
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();

    // Check capability
    let capability = backend.get_capability().unwrap();
    assert!(!capability.supports_canfd);
    assert_eq!(capability.channel_count, 1);

    backend.open_channel(0).unwrap();

    // Standard message should work
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // CAN-FD message should fail
    let msg_fd = CanMessage::new_fd(CanId::Standard(0x123), &[1, 2, 3]).unwrap();
    let result = backend.send_message(&msg_fd);
    assert!(result.is_err());
}

/// Test custom configuration.
#[test]
fn test_mock_custom_configuration() {
    let config = MockConfig {
        channel_count: 4,
        max_bitrate: 5_000_000,
        filter_count: 32,
        ..Default::default()
    };

    let backend = MockBackend::with_config(config);
    let capability = backend.get_capability().unwrap();

    assert_eq!(capability.channel_count, 4);
    assert_eq!(capability.max_bitrate, 5_000_000);
    assert_eq!(capability.filter_count, 32);
}

/// Test message recording with capacity limit.
#[test]
fn test_mock_recording_capacity_limit() {
    let config = MockConfig {
        max_recorded_messages: 3,
        ..Default::default()
    };

    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Send 5 messages
    for i in 0..5 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        backend.send_message(&msg).unwrap();
    }

    // Should only keep last 3 messages
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 3);

    // Verify it's the last 3 messages (FIFO)
    assert_eq!(recorded[0].id(), CanId::Standard(0x102));
    assert_eq!(recorded[1].id(), CanId::Standard(0x103));
    assert_eq!(recorded[2].id(), CanId::Standard(0x104));
}

/// Test that Mock backend works with multiple channels.
#[test]
fn test_mock_multiple_channels() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Open both channels
    backend.open_channel(0).unwrap();
    backend.open_channel(1).unwrap();

    // Send messages (should work with any channel open)
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // Close one channel
    backend.close_channel(0).unwrap();

    // Should still work with channel 1 open
    assert!(backend.send_message(&msg).is_ok());

    // Close last channel
    backend.close_channel(1).unwrap();

    // Should fail with no channels open
    assert!(backend.send_message(&msg).is_err());
}

/// Test Mock backend state management.
#[test]
fn test_mock_state_management() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    // Check initial state
    assert_eq!(
        backend.get_state(),
        canlink_hal::BackendState::Uninitialized
    );

    // Initialize
    backend.initialize(&config).unwrap();
    assert_eq!(backend.get_state(), canlink_hal::BackendState::Ready);

    // Close
    backend.close().unwrap();
    assert_eq!(backend.get_state(), canlink_hal::BackendState::Closed);
}

/// Test that Mock backend properly validates operations based on state.
#[test]
fn test_mock_state_validation() {
    let mut backend = MockBackend::new();

    // Operations should fail before initialization
    assert!(backend.open_channel(0).is_err());

    // Initialize
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Operations should work now
    assert!(backend.open_channel(0).is_ok());

    // Close
    backend.close().unwrap();

    // Operations should fail after close
    assert!(backend.open_channel(0).is_err());
}

/// Test Mock backend with preset messages and recording simultaneously.
#[test]
fn test_mock_preset_and_recording() {
    let preset = vec![CanMessage::new_standard(0x111, &[0x11]).unwrap()];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Send a message (should be recorded)
    let sent_msg = CanMessage::new_standard(0x222, &[0x22]).unwrap();
    backend.send_message(&sent_msg).unwrap();

    // Receive preset message
    let received = backend.receive_message().unwrap();
    assert!(received.is_some());
    assert_eq!(received.unwrap().id(), CanId::Standard(0x111));

    // Check recorded messages (should only have sent message, not received)
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].id(), CanId::Standard(0x222));
}

/// Test Mock backend configuration persistence.
#[test]
fn test_mock_config_persistence() {
    let config = MockConfig::can20_only();
    let backend = MockBackend::with_config(config.clone());

    // Configuration should be accessible
    let backend_config = backend.get_config();
    assert_eq!(backend_config.channel_count, config.channel_count);
    assert_eq!(backend_config.supports_canfd, config.supports_canfd);
}

/// Test Mock backend with empty preset messages.
#[test]
fn test_mock_empty_preset_messages() {
    let config = MockConfig::with_preset_messages(vec![]);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Should return None immediately
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_none());
}

/// Test Mock backend version and name.
#[test]
fn test_mock_metadata() {
    let backend = MockBackend::new();

    assert_eq!(backend.name(), "mock");

    let version = backend.version();
    assert_eq!(version.major(), 0);
    assert_eq!(version.minor(), 1);
    assert_eq!(version.patch(), 0);
}
