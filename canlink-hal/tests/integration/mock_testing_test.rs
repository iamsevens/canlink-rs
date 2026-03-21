//! Mock testing integration tests.
//!
//! These tests demonstrate the complete mock testing workflow including
//! error injection, preset messages, and message verification.

use canlink_hal::{BackendConfig, CanBackend, CanError, CanId, CanMessage};
use canlink_mock::MockBackend;

/// Test complete error injection workflow.
#[test]
fn test_error_injection_workflow() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Configure error injection to fail the 3rd send
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Simulated bus-off".to_string(),
        },
        1, // inject once
        2, // skip first 2 calls
    );

    // First two sends should succeed
    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    assert!(backend.send_message(&msg1).is_ok());

    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    assert!(backend.send_message(&msg2).is_ok());

    // Third send should fail
    let msg3 = CanMessage::new_standard(0x300, &[3]).unwrap();
    let result = backend.send_message(&msg3);
    assert!(result.is_err());
    match result {
        Err(CanError::SendFailed { reason }) => {
            assert_eq!(reason, "Simulated bus-off");
        }
        _ => panic!("Expected SendFailed error"),
    }

    // Fourth send should succeed (error injection exhausted)
    let msg4 = CanMessage::new_standard(0x400, &[4]).unwrap();
    assert!(backend.send_message(&msg4).is_ok());

    // Verify only successful messages were recorded
    assert!(backend.verify_message_count(3));
    assert!(backend.verify_message_sent(CanId::Standard(0x100)));
    assert!(backend.verify_message_sent(CanId::Standard(0x200)));
    assert!(!backend.verify_message_sent(CanId::Standard(0x300)));
    assert!(backend.verify_message_sent(CanId::Standard(0x400)));
}

/// Test preset messages with verification.
#[test]
fn test_preset_messages_workflow() {
    // Create backend with preset messages
    let preset_messages = vec![
        CanMessage::new_standard(0x111, &[0x11, 0x22]).unwrap(),
        CanMessage::new_standard(0x222, &[0x33, 0x44, 0x55]).unwrap(),
        CanMessage::new_extended(0x12345678, &[0x66]).unwrap(),
    ];

    let config = canlink_mock::MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(config);

    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Receive and verify first message
    let msg1 = backend.receive_message().unwrap();
    assert!(msg1.is_some());
    let msg1 = msg1.unwrap();
    assert_eq!(msg1.id(), CanId::Standard(0x111));
    assert_eq!(msg1.data(), &[0x11, 0x22]);

    // Receive and verify second message
    let msg2 = backend.receive_message().unwrap();
    assert!(msg2.is_some());
    let msg2 = msg2.unwrap();
    assert_eq!(msg2.id(), CanId::Standard(0x222));
    assert_eq!(msg2.data(), &[0x33, 0x44, 0x55]);

    // Receive and verify third message (extended ID)
    let msg3 = backend.receive_message().unwrap();
    assert!(msg3.is_some());
    let msg3 = msg3.unwrap();
    assert_eq!(msg3.id(), CanId::Extended(0x12345678));
    assert_eq!(msg3.data(), &[0x66]);

    // No more messages
    let msg4 = backend.receive_message().unwrap();
    assert!(msg4.is_none());
}

/// Test message verification with filtering.
#[test]
fn test_message_verification_workflow() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send multiple messages with different IDs
    backend
        .send_message(&CanMessage::new_standard(0x100, &[1, 2]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x200, &[3, 4]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x100, &[5, 6]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x300, &[7, 8]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x100, &[9, 10]).unwrap())
        .unwrap();

    // Verify total count
    assert!(backend.verify_message_count(5));

    // Verify specific IDs were sent
    assert!(backend.verify_message_sent(CanId::Standard(0x100)));
    assert!(backend.verify_message_sent(CanId::Standard(0x200)));
    assert!(backend.verify_message_sent(CanId::Standard(0x300)));
    assert!(!backend.verify_message_sent(CanId::Standard(0x400)));

    // Get messages by ID and verify
    let messages_100 = backend.get_messages_by_id(CanId::Standard(0x100));
    assert_eq!(messages_100.len(), 3);
    assert_eq!(messages_100[0].data(), &[1, 2]);
    assert_eq!(messages_100[1].data(), &[5, 6]);
    assert_eq!(messages_100[2].data(), &[9, 10]);

    let messages_200 = backend.get_messages_by_id(CanId::Standard(0x200));
    assert_eq!(messages_200.len(), 1);
    assert_eq!(messages_200[0].data(), &[3, 4]);

    let messages_300 = backend.get_messages_by_id(CanId::Standard(0x300));
    assert_eq!(messages_300.len(), 1);
    assert_eq!(messages_300[0].data(), &[7, 8]);
}

/// Test combined error injection and verification.
#[test]
fn test_combined_error_injection_and_verification() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Inject error after first 2 successful sends, then fail 3 times
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Intermittent failure".to_string(),
        },
        3, // inject 3 times
        2, // skip first 2 calls
    );

    // Send 6 messages
    for i in 0..6 {
        let msg = CanMessage::new_standard(0x100 + i as u16, &[i]).unwrap();
        let result = backend.send_message(&msg);

        if i < 2 {
            // First 2 should succeed (skip)
            assert!(result.is_ok(), "Message {} should succeed (skip)", i);
        } else if i < 5 {
            // Next 3 should fail (inject)
            assert!(result.is_err(), "Message {} should fail (inject)", i);
        } else {
            // Last one should succeed (injection exhausted)
            assert!(result.is_ok(), "Message {} should succeed (exhausted)", i);
        }
    }

    // Verify only successful messages were recorded
    assert!(backend.verify_message_count(3));
    assert!(backend.verify_message_sent(CanId::Standard(0x100))); // i=0 (skip)
    assert!(backend.verify_message_sent(CanId::Standard(0x101))); // i=1 (skip)
    assert!(!backend.verify_message_sent(CanId::Standard(0x102))); // i=2 (failed)
    assert!(!backend.verify_message_sent(CanId::Standard(0x103))); // i=3 (failed)
    assert!(!backend.verify_message_sent(CanId::Standard(0x104))); // i=4 (failed)
    assert!(backend.verify_message_sent(CanId::Standard(0x105))); // i=5 (exhausted)
}

/// Test clearing and resetting mock state.
#[test]
fn test_mock_state_reset() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send some messages
    backend
        .send_message(&CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x456, &[4, 5, 6]).unwrap())
        .unwrap();

    assert!(backend.verify_message_count(2));

    // Clear recorded messages
    backend.clear_recorded_messages();
    assert!(backend.verify_message_count(0));
    assert!(!backend.verify_message_sent(CanId::Standard(0x123)));

    // Send new messages
    backend
        .send_message(&CanMessage::new_standard(0x789, &[7, 8, 9]).unwrap())
        .unwrap();

    assert!(backend.verify_message_count(1));
    assert!(backend.verify_message_sent(CanId::Standard(0x789)));
    assert!(!backend.verify_message_sent(CanId::Standard(0x123)));
}

/// Test error injection on different operations.
#[test]
fn test_multi_operation_error_injection() {
    let mut backend = MockBackend::new();

    // Inject initialization error
    backend
        .error_injector_mut()
        .inject_init_error(CanError::InitializationFailed {
            reason: "Hardware not found".to_string(),
        });

    let config = BackendConfig::new("mock");
    let result = backend.initialize(&config);
    assert!(result.is_err());

    // Clear errors and initialize successfully
    backend.error_injector_mut().clear();
    assert!(backend.initialize(&config).is_ok());

    // Inject open_channel error
    backend
        .error_injector_mut()
        .inject_open_channel_error(CanError::ChannelNotFound { channel: 0, max: 1 });

    let result = backend.open_channel(0);
    assert!(result.is_err());

    // Clear and open successfully
    backend.error_injector_mut().clear();
    assert!(backend.open_channel(0).is_ok());

    // Inject receive error
    backend
        .error_injector_mut()
        .inject_receive_error(CanError::ReceiveFailed {
            reason: "Buffer overflow".to_string(),
        });

    let result = backend.receive_message();
    assert!(result.is_err());

    // Clear errors
    backend.error_injector_mut().clear();
}

/// Test realistic testing scenario: protocol implementation.
#[test]
fn test_realistic_protocol_testing() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Simulate a request-response protocol
    // Step 1: Send request
    let request = CanMessage::new_standard(0x7E0, &[0x02, 0x01, 0x00]).unwrap();
    backend.send_message(&request).unwrap();

    // Verify request was sent
    assert!(backend.verify_message_sent(CanId::Standard(0x7E0)));
    let requests = backend.get_messages_by_id(CanId::Standard(0x7E0));
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].data(), &[0x02, 0x01, 0x00]);

    // Step 2: Simulate error on second request
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Bus busy".to_string(),
        });

    let request2 = CanMessage::new_standard(0x7E0, &[0x02, 0x01, 0x01]).unwrap();
    let result = backend.send_message(&request2);
    assert!(result.is_err());

    // Step 3: Retry should succeed
    backend.error_injector_mut().clear();
    backend.send_message(&request2).unwrap();

    // Verify both successful requests
    let all_requests = backend.get_messages_by_id(CanId::Standard(0x7E0));
    assert_eq!(all_requests.len(), 2);
    assert_eq!(all_requests[0].data(), &[0x02, 0x01, 0x00]);
    assert_eq!(all_requests[1].data(), &[0x02, 0x01, 0x01]);
}

/// Test CAN-FD capability adaptation with mock.
#[test]
fn test_canfd_capability_adaptation() {
    // Test with CAN-FD support
    let mut backend_fd = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend_fd.initialize(&config).unwrap();
    backend_fd.open_channel(0).unwrap();

    let capability = backend_fd.get_capability().unwrap();
    assert!(capability.supports_canfd);

    // Send CAN-FD message
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let msg_fd = CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap();
    assert!(backend_fd.send_message(&msg_fd).is_ok());

    // Test with CAN 2.0 only
    let config_20 = canlink_mock::MockConfig::can20_only();
    let mut backend_20 = MockBackend::with_config(config_20);
    backend_20.initialize(&config).unwrap();
    backend_20.open_channel(0).unwrap();

    let capability = backend_20.get_capability().unwrap();
    assert!(!capability.supports_canfd);

    // CAN-FD message should fail
    let result = backend_20.send_message(&msg_fd);
    assert!(result.is_err());
    match result {
        Err(CanError::UnsupportedFeature { feature }) => {
            assert_eq!(feature, "CAN-FD");
        }
        _ => panic!("Expected UnsupportedFeature error"),
    }

    // Regular CAN 2.0 message should work
    let msg_20 = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(backend_20.send_message(&msg_20).is_ok());
}

/// Test injection counter tracking.
#[test]
fn test_injection_counter_tracking() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Initial count should be 0
    assert_eq!(backend.error_injector().injection_count(), 0);

    // Inject send error 3 times
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Test".to_string(),
        },
        3,
        0,
    );

    // Trigger 3 errors
    for _ in 0..3 {
        let msg = CanMessage::new_standard(0x100, &[1]).unwrap();
        let _ = backend.send_message(&msg);
    }

    assert_eq!(backend.error_injector().injection_count(), 3);

    // Clear should reset counter
    backend.error_injector_mut().clear();
    assert_eq!(backend.error_injector().injection_count(), 0);
}
