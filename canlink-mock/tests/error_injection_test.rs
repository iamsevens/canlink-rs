//! Error injection tests.
//!
//! Tests for error injection functionality in the Mock backend.

use canlink_hal::{BackendConfig, CanBackend, CanError, CanMessage};
use canlink_mock::MockBackend;

/// Test basic send error injection.
#[test]
fn test_inject_send_error() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject a send error
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Test error".to_string(),
        });

    // First send should fail
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    let result = backend.send_message(&msg);
    assert!(result.is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Second send should succeed
    let result = backend.send_message(&msg);
    assert!(result.is_ok());
    assert_eq!(backend.error_injector().injection_count(), 1);
}

/// Test send error injection with skip.
#[test]
fn test_inject_send_error_with_skip() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject error on 3rd call (skip first 2)
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Test error".to_string(),
        },
        1, // inject once
        2, // skip 2 calls
    );

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();

    // First two sends should succeed
    assert!(backend.send_message(&msg).is_ok());
    assert!(backend.send_message(&msg).is_ok());
    assert_eq!(backend.error_injector().injection_count(), 0);

    // Third send should fail
    assert!(backend.send_message(&msg).is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Fourth send should succeed
    assert!(backend.send_message(&msg).is_ok());
}

/// Test multiple error injections.
#[test]
fn test_inject_multiple_send_errors() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject error 3 times
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Test error".to_string(),
        },
        3, // inject 3 times
        0, // no skip
    );

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();

    // First three sends should fail
    assert!(backend.send_message(&msg).is_err());
    assert!(backend.send_message(&msg).is_err());
    assert!(backend.send_message(&msg).is_err());
    assert_eq!(backend.error_injector().injection_count(), 3);

    // Fourth send should succeed
    assert!(backend.send_message(&msg).is_ok());
    assert_eq!(backend.error_injector().injection_count(), 3);
}

/// Test receive error injection.
#[test]
fn test_inject_receive_error() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject a receive error
    backend
        .error_injector_mut()
        .inject_receive_error(CanError::ReceiveFailed {
            reason: "Test error".to_string(),
        });

    // First receive should fail
    let result = backend.receive_message();
    assert!(result.is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Second receive should succeed
    let result = backend.receive_message();
    assert!(result.is_ok());
}

/// Test initialization error injection.
#[test]
fn test_inject_init_error() {
    let mut backend = MockBackend::new();

    // Inject an initialization error
    backend
        .error_injector_mut()
        .inject_init_error(CanError::InitializationFailed {
            reason: "Test error".to_string(),
        });

    // Initialization should fail
    let config = BackendConfig::new("mock");
    let result = backend.initialize(&config);
    assert!(result.is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);
}

/// Test open channel error injection.
#[test]
fn test_inject_open_channel_error() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Inject an open channel error
    backend
        .error_injector_mut()
        .inject_open_channel_error(CanError::ChannelNotFound { channel: 0, max: 1 });

    // Opening channel should fail
    let result = backend.open_channel(0);
    assert!(result.is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Second attempt should succeed
    let result = backend.open_channel(0);
    assert!(result.is_ok());
}

/// Test close channel error injection.
#[test]
fn test_inject_close_channel_error() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject a close channel error
    backend
        .error_injector_mut()
        .inject_close_channel_error(CanError::ChannelNotOpen { channel: 0 });

    // Closing channel should fail
    let result = backend.close_channel(0);
    assert!(result.is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Second attempt should succeed
    let result = backend.close_channel(0);
    assert!(result.is_ok());
}

/// Test clearing injected errors.
#[test]
fn test_clear_injected_errors() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject a send error
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Test error".to_string(),
        });

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();

    // First send should fail
    assert!(backend.send_message(&msg).is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Clear errors
    backend.error_injector_mut().clear();
    assert_eq!(backend.error_injector().injection_count(), 0);

    // Now send should succeed
    assert!(backend.send_message(&msg).is_ok());
}

/// Test infinite error injection.
#[test]
fn test_infinite_error_injection() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject error infinitely (count = 0)
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Test error".to_string(),
        },
        0, // infinite
        0, // no skip
    );

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();

    // All sends should fail
    for i in 1..=10 {
        assert!(backend.send_message(&msg).is_err());
        assert_eq!(backend.error_injector().injection_count(), i);
    }
}

/// Test multiple error types simultaneously.
#[test]
fn test_multiple_error_types() {
    let mut backend = MockBackend::new();

    // Inject errors for different operations
    backend
        .error_injector_mut()
        .inject_init_error(CanError::InitializationFailed {
            reason: "Init error".to_string(),
        });
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Send error".to_string(),
        });
    backend
        .error_injector_mut()
        .inject_receive_error(CanError::ReceiveFailed {
            reason: "Receive error".to_string(),
        });

    // Init should fail
    let config = BackendConfig::new("mock");
    assert!(backend.initialize(&config).is_err());
    assert_eq!(backend.error_injector().injection_count(), 1);

    // Clear and reinitialize
    backend.error_injector_mut().clear();
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Re-inject send and receive errors
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Send error".to_string(),
        });
    backend
        .error_injector_mut()
        .inject_receive_error(CanError::ReceiveFailed {
            reason: "Receive error".to_string(),
        });

    // Both should fail
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_err());
    assert!(backend.receive_message().is_err());
    assert_eq!(backend.error_injector().injection_count(), 2);
}

/// Test error injection with different error types.
#[test]
fn test_different_error_types() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Test timeout error
    backend
        .error_injector_mut()
        .inject_send_error(CanError::Timeout { timeout_ms: 1000 });
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    let result = backend.send_message(&msg);
    assert!(result.is_err());
    if let Err(CanError::Timeout { timeout_ms }) = result {
        assert_eq!(timeout_ms, 1000);
    } else {
        panic!("Expected Timeout error");
    }

    // Test unsupported feature error
    backend
        .error_injector_mut()
        .inject_send_error(CanError::UnsupportedFeature {
            feature: "Test feature".to_string(),
        });
    let result = backend.send_message(&msg);
    assert!(result.is_err());
}
