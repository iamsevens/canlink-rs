//! Backend trait contract tests.
//!
//! These tests verify that backend implementations correctly follow the CanBackend trait contract.
//! All backends (Mock, `TSMaster`, PEAK, etc.) must pass these tests.

use canlink_hal::{BackendConfig, BackendState, CanBackend, CanError, CanId, CanMessage};
use canlink_mock::MockBackend;

/// Test that a backend can be initialized and closed.
#[test]
fn test_backend_lifecycle() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    // Should start uninitialized
    assert_eq!(backend.get_state(), BackendState::Uninitialized);

    // Initialize should succeed
    assert!(backend.initialize(&config).is_ok());
    assert_eq!(backend.get_state(), BackendState::Ready);

    // Close should succeed
    assert!(backend.close().is_ok());
    assert_eq!(backend.get_state(), BackendState::Closed);
}

/// Test that double initialization fails.
#[test]
fn test_backend_double_initialization() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    backend.initialize(&config).unwrap();

    // Second initialization should fail
    let result = backend.initialize(&config);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), CanError::InvalidState { .. }));
}

/// Test that operations fail when backend is not initialized.
#[test]
fn test_backend_operations_require_initialization() {
    let mut backend = MockBackend::new();

    // Operations should fail before initialization
    assert!(backend.open_channel(0).is_err());
    assert!(backend.close_channel(0).is_err());

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_err());
    assert!(backend.receive_message().is_err());
}

/// Test that capability query works.
#[test]
fn test_backend_capability_query() {
    let backend = MockBackend::new();

    // Capability query should work even before initialization
    let capability = backend.get_capability();
    assert!(capability.is_ok());

    let cap = capability.unwrap();
    assert!(cap.channel_count > 0);
    assert!(!cap.supported_bitrates.is_empty());
}

/// Test channel management.
#[test]
fn test_backend_channel_management() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Open channel should succeed
    assert!(backend.open_channel(0).is_ok());

    // Opening same channel again should fail
    let result = backend.open_channel(0);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CanError::ChannelAlreadyOpen { .. }
    ));

    // Close channel should succeed
    assert!(backend.close_channel(0).is_ok());

    // Closing same channel again should fail
    let result = backend.close_channel(0);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CanError::ChannelNotOpen { .. }
    ));
}

/// Test opening invalid channel.
#[test]
fn test_backend_invalid_channel() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    let capability = backend.get_capability().unwrap();
    let invalid_channel = capability.channel_count + 10;

    // Opening invalid channel should fail
    let result = backend.open_channel(invalid_channel);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CanError::ChannelNotFound { .. }
    ));
}

/// Test message sending.
#[test]
fn test_backend_send_message() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send standard message
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // Send extended message
    let msg = CanMessage::new_extended(0x12345678, &[5, 6, 7, 8]).unwrap();
    assert!(backend.send_message(&msg).is_ok());
}

/// Test message sending without open channel.
#[test]
fn test_backend_send_without_channel() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    let result = backend.send_message(&msg);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        CanError::ChannelNotOpen { .. }
    ));
}

/// Test message receiving.
#[test]
fn test_backend_receive_message() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Receive should return Ok(None) when no messages available
    let result = backend.receive_message();
    assert!(result.is_ok());
    // Mock backend with no preset messages returns None
}

/// Test backend version and name.
#[test]
fn test_backend_metadata() {
    let backend = MockBackend::new();

    // Name should be non-empty
    assert!(!backend.name().is_empty());

    // Version should be valid (all components are u8, so always >= 0)
    let _version = backend.version();
}

/// Test that backend properly validates message data length.
#[test]
fn test_backend_message_validation() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Valid CAN 2.0 message (0-8 bytes)
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // CAN-FD message should work if supported
    let capability = backend.get_capability().unwrap();
    if capability.supports_canfd {
        let data = vec![0u8; 64]; // Max CAN-FD data
        let msg = CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap();
        assert!(backend.send_message(&msg).is_ok());
    }
}

/// Test backend state transitions.
#[test]
fn test_backend_state_transitions() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    // Uninitialized -> Ready
    assert_eq!(backend.get_state(), BackendState::Uninitialized);
    backend.initialize(&config).unwrap();
    assert_eq!(backend.get_state(), BackendState::Ready);

    // Ready -> Closed
    backend.close().unwrap();
    assert_eq!(backend.get_state(), BackendState::Closed);

    // Operations should fail after close
    assert!(backend.open_channel(0).is_err());
}

/// Test that closing backend closes all channels.
#[test]
fn test_backend_close_closes_channels() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Open multiple channels
    backend.open_channel(0).unwrap();
    backend.open_channel(1).unwrap();

    // Close backend
    backend.close().unwrap();

    // Reinitialize
    backend.initialize(&config).unwrap();

    // Channels should be closed (can open again)
    assert!(backend.open_channel(0).is_ok());
    assert!(backend.open_channel(1).is_ok());
}

/// Test backend with multiple channels.
#[test]
fn test_backend_multiple_channels() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    let capability = backend.get_capability().unwrap();

    // Open all available channels
    for channel in 0..capability.channel_count {
        assert!(backend.open_channel(channel).is_ok());
    }

    // Send message on each channel (should work with any channel open)
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // Close all channels
    for channel in 0..capability.channel_count {
        assert!(backend.close_channel(channel).is_ok());
    }
}

/// Test that backend handles standard and extended IDs correctly.
#[test]
fn test_backend_id_types() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Standard ID (11-bit)
    let msg = CanMessage::new_standard(0x7FF, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // Extended ID (29-bit)
    let msg = CanMessage::new_extended(0x1FFFFFFF, &[4, 5, 6]).unwrap();
    assert!(backend.send_message(&msg).is_ok());
}
