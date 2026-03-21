//! Message verification tests.
//!
//! Tests for message verification functionality in the Mock backend.

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::MockBackend;

/// Test basic message verification by ID.
#[test]
fn test_verify_message_sent_basic() {
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

    // Verify messages were sent
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    assert!(backend.verify_message_sent(CanId::Standard(0x456)));
    assert!(!backend.verify_message_sent(CanId::Standard(0x789)));
}

/// Test message verification with extended IDs.
#[test]
fn test_verify_message_sent_extended() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send messages with extended IDs
    backend
        .send_message(&CanMessage::new_extended(0x12345678, &[1]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_extended(0x1FFFFFFF, &[2]).unwrap())
        .unwrap();

    // Verify extended IDs
    assert!(backend.verify_message_sent(CanId::Extended(0x12345678)));
    assert!(backend.verify_message_sent(CanId::Extended(0x1FFFFFFF)));
    assert!(!backend.verify_message_sent(CanId::Extended(0x11111111)));
}

/// Test message verification with mixed ID types.
#[test]
fn test_verify_message_sent_mixed_ids() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send both standard and extended IDs
    backend
        .send_message(&CanMessage::new_standard(0x123, &[1]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_extended(0x12345678, &[2]).unwrap())
        .unwrap();

    // Verify both types
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    assert!(backend.verify_message_sent(CanId::Extended(0x12345678)));

    // Standard and extended with same numeric value are different
    assert!(!backend.verify_message_sent(CanId::Extended(0x123)));
    // Extended ID truncated to standard range should not match
    assert!(!backend.verify_message_sent(CanId::Standard(0x5678)));
}

/// Test getting messages by ID.
#[test]
fn test_get_messages_by_id() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send multiple messages with same ID
    backend
        .send_message(&CanMessage::new_standard(0x123, &[1]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x456, &[2]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x123, &[3]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x123, &[4]).unwrap())
        .unwrap();

    // Get messages by ID
    let messages_123 = backend.get_messages_by_id(CanId::Standard(0x123));
    assert_eq!(messages_123.len(), 3);
    assert_eq!(messages_123[0].data(), &[1]);
    assert_eq!(messages_123[1].data(), &[3]);
    assert_eq!(messages_123[2].data(), &[4]);

    let messages_456 = backend.get_messages_by_id(CanId::Standard(0x456));
    assert_eq!(messages_456.len(), 1);
    assert_eq!(messages_456[0].data(), &[2]);

    let messages_789 = backend.get_messages_by_id(CanId::Standard(0x789));
    assert_eq!(messages_789.len(), 0);
}

/// Test message count verification.
#[test]
fn test_verify_message_count() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Initially no messages
    assert!(backend.verify_message_count(0));
    assert!(!backend.verify_message_count(1));

    // Send one message
    backend
        .send_message(&CanMessage::new_standard(0x123, &[1]).unwrap())
        .unwrap();
    assert!(backend.verify_message_count(1));
    assert!(!backend.verify_message_count(0));
    assert!(!backend.verify_message_count(2));

    // Send more messages
    backend
        .send_message(&CanMessage::new_standard(0x456, &[2]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x789, &[3]).unwrap())
        .unwrap();
    assert!(backend.verify_message_count(3));
    assert!(!backend.verify_message_count(2));
    assert!(!backend.verify_message_count(4));
}

/// Test message verification after clearing.
#[test]
fn test_verify_after_clear() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send messages
    backend
        .send_message(&CanMessage::new_standard(0x123, &[1]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x456, &[2]).unwrap())
        .unwrap();

    // Verify messages exist
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    assert!(backend.verify_message_count(2));

    // Clear messages
    backend.clear_recorded_messages();

    // Verify messages are gone
    assert!(!backend.verify_message_sent(CanId::Standard(0x123)));
    assert!(backend.verify_message_count(0));
}

/// Test message verification with CAN-FD frames.
#[test]
fn test_verify_canfd_messages() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send CAN-FD messages
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    backend
        .send_message(&CanMessage::new_fd(CanId::Standard(0x123), &data).unwrap())
        .unwrap();

    // Verify CAN-FD message
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    let messages = backend.get_messages_by_id(CanId::Standard(0x123));
    assert_eq!(messages.len(), 1);
    assert!(messages[0].is_fd());
    assert_eq!(messages[0].data().len(), 12);
}

/// Test message verification with remote frames.
#[test]
fn test_verify_remote_frames() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send remote frame
    backend
        .send_message(&CanMessage::new_remote(CanId::Standard(0x123), 4).unwrap())
        .unwrap();

    // Verify remote frame
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));
    let messages = backend.get_messages_by_id(CanId::Standard(0x123));
    assert_eq!(messages.len(), 1);
    assert!(messages[0].is_remote());
}

/// Test message verification with large number of messages.
#[test]
fn test_verify_large_message_count() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send 100 messages
    for i in 0..100 {
        let id = (i % 10) as u16;
        backend
            .send_message(&CanMessage::new_standard(id, &[i as u8]).unwrap())
            .unwrap();
    }

    // Verify total count
    assert!(backend.verify_message_count(100));

    // Verify each ID has 10 messages
    for i in 0..10 {
        let messages = backend.get_messages_by_id(CanId::Standard(i));
        assert_eq!(messages.len(), 10);
    }
}

/// Test message verification order preservation.
#[test]
fn test_verify_message_order() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send messages in specific order
    backend
        .send_message(&CanMessage::new_standard(0x100, &[1]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x100, &[2]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x100, &[3]).unwrap())
        .unwrap();

    // Verify order is preserved
    let messages = backend.get_messages_by_id(CanId::Standard(0x100));
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].data()[0], 1);
    assert_eq!(messages[1].data()[0], 2);
    assert_eq!(messages[2].data()[0], 3);
}

/// Test message verification with no messages sent.
#[test]
fn test_verify_no_messages() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // No messages sent
    assert!(!backend.verify_message_sent(CanId::Standard(0x123)));
    assert!(backend.verify_message_count(0));
    assert_eq!(backend.get_messages_by_id(CanId::Standard(0x123)).len(), 0);
}

/// Test message verification across multiple channels.
#[test]
fn test_verify_multiple_channels() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();

    // Open multiple channels
    backend.open_channel(0).unwrap();
    backend.open_channel(1).unwrap();

    // Send messages (all recorded regardless of channel)
    backend
        .send_message(&CanMessage::new_standard(0x100, &[1]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x200, &[2]).unwrap())
        .unwrap();

    // Verify messages
    assert!(backend.verify_message_sent(CanId::Standard(0x100)));
    assert!(backend.verify_message_sent(CanId::Standard(0x200)));
    assert!(backend.verify_message_count(2));
}

/// Test message verification with duplicate IDs and different data.
#[test]
fn test_verify_duplicate_ids_different_data() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // Send same ID with different data
    backend
        .send_message(&CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x123, &[4, 5, 6, 7, 8]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x123, &[9]).unwrap())
        .unwrap();

    // Verify all messages are recorded
    let messages = backend.get_messages_by_id(CanId::Standard(0x123));
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].data(), &[1, 2, 3]);
    assert_eq!(messages[1].data(), &[4, 5, 6, 7, 8]);
    assert_eq!(messages[2].data(), &[9]);
}
