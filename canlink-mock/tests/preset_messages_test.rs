//! Preset messages tests.
//!
//! Tests for preset message functionality in the Mock backend.

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

/// Test basic preset message functionality.
#[test]
fn test_preset_messages_basic() {
    let preset = vec![
        CanMessage::new_standard(0x111, &[1, 2, 3]).unwrap(),
        CanMessage::new_standard(0x222, &[4, 5, 6]).unwrap(),
        CanMessage::new_standard(0x333, &[7, 8, 9]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Receive first message
    let msg1 = backend.receive_message().unwrap();
    assert!(msg1.is_some());
    assert_eq!(msg1.unwrap().id(), CanId::Standard(0x111));

    // Receive second message
    let msg2 = backend.receive_message().unwrap();
    assert!(msg2.is_some());
    assert_eq!(msg2.unwrap().id(), CanId::Standard(0x222));

    // Receive third message
    let msg3 = backend.receive_message().unwrap();
    assert!(msg3.is_some());
    assert_eq!(msg3.unwrap().id(), CanId::Standard(0x333));

    // No more messages
    let msg4 = backend.receive_message().unwrap();
    assert!(msg4.is_none());
}

/// Test preset messages with different data lengths.
#[test]
fn test_preset_messages_different_lengths() {
    let preset = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[1, 2, 3, 4]).unwrap(),
        CanMessage::new_standard(0x300, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Verify data lengths
    let msg1 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg1.data().len(), 1);

    let msg2 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg2.data().len(), 4);

    let msg3 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg3.data().len(), 8);
}

/// Test preset messages with CAN-FD frames.
#[test]
fn test_preset_messages_canfd() {
    let preset = vec![
        CanMessage::new_standard(0x100, &[1, 2, 3]).unwrap(),
        CanMessage::new_fd(
            CanId::Standard(0x200),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        )
        .unwrap(),
        CanMessage::new_standard(0x300, &[7, 8]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // First message is CAN 2.0
    let msg1 = backend.receive_message().unwrap().unwrap();
    assert!(!msg1.is_fd());
    assert_eq!(msg1.data().len(), 3);

    // Second message is CAN-FD
    let msg2 = backend.receive_message().unwrap().unwrap();
    assert!(msg2.is_fd());
    assert_eq!(msg2.data().len(), 12);

    // Third message is CAN 2.0
    let msg3 = backend.receive_message().unwrap().unwrap();
    assert!(!msg3.is_fd());
    assert_eq!(msg3.data().len(), 2);
}

/// Test preset messages with extended IDs.
#[test]
fn test_preset_messages_extended_ids() {
    let preset = vec![
        CanMessage::new_standard(0x123, &[1]).unwrap(),
        CanMessage::new_extended(0x12345678, &[2]).unwrap(),
        CanMessage::new_extended(0x1FFFFFFF, &[3]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Standard ID
    let msg1 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg1.id(), CanId::Standard(0x123));

    // Extended IDs
    let msg2 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg2.id(), CanId::Extended(0x12345678));

    let msg3 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg3.id(), CanId::Extended(0x1FFFFFFF));
}

/// Test empty preset messages list.
#[test]
fn test_preset_messages_empty() {
    let config = MockConfig::with_preset_messages(vec![]);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Should return None immediately
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_none());
}

/// Test preset messages with single message.
#[test]
fn test_preset_messages_single() {
    let preset = vec![CanMessage::new_standard(0x100, &[0xAA, 0xBB]).unwrap()];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Receive the single message
    let msg = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg.id(), CanId::Standard(0x100));
    assert_eq!(msg.data(), &[0xAA, 0xBB]);

    // No more messages
    let msg = backend.receive_message().unwrap();
    assert!(msg.is_none());
}

/// Test preset messages order is preserved.
#[test]
fn test_preset_messages_order() {
    let preset = vec![
        CanMessage::new_standard(0x001, &[1]).unwrap(),
        CanMessage::new_standard(0x002, &[2]).unwrap(),
        CanMessage::new_standard(0x003, &[3]).unwrap(),
        CanMessage::new_standard(0x004, &[4]).unwrap(),
        CanMessage::new_standard(0x005, &[5]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Verify messages are received in order
    for i in 1..=5 {
        let msg = backend.receive_message().unwrap().unwrap();
        assert_eq!(msg.id(), CanId::Standard(i));
        assert_eq!(msg.data()[0], i as u8);
    }

    // No more messages
    assert!(backend.receive_message().unwrap().is_none());
}

/// Test preset messages with remote frames.
#[test]
fn test_preset_messages_remote_frames() {
    let preset = vec![
        CanMessage::new_standard(0x100, &[1, 2, 3]).unwrap(),
        CanMessage::new_remote(CanId::Standard(0x200), 4).unwrap(),
        CanMessage::new_standard(0x300, &[5, 6]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Data frame
    let msg1 = backend.receive_message().unwrap().unwrap();
    assert!(!msg1.is_remote());
    assert_eq!(msg1.data().len(), 3);

    // Remote frame
    let msg2 = backend.receive_message().unwrap().unwrap();
    assert!(msg2.is_remote());
    assert_eq!(msg2.data().len(), 0);

    // Data frame
    let msg3 = backend.receive_message().unwrap().unwrap();
    assert!(!msg3.is_remote());
    assert_eq!(msg3.data().len(), 2);
}

/// Test preset messages with large dataset.
#[test]
fn test_preset_messages_large_dataset() {
    // Create 100 preset messages
    let mut preset = Vec::new();
    for i in 0..100 {
        let id = (i % 0x7FF) as u16;
        let data = vec![i as u8];
        preset.push(CanMessage::new_standard(id, &data).unwrap());
    }

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Receive all 100 messages
    for i in 0..100 {
        let msg = backend.receive_message().unwrap();
        assert!(msg.is_some());
        let msg = msg.unwrap();
        assert_eq!(msg.data()[0], i as u8);
    }

    // No more messages
    assert!(backend.receive_message().unwrap().is_none());
}

/// Test preset messages combined with send/record.
#[test]
fn test_preset_messages_with_send() {
    let preset = vec![
        CanMessage::new_standard(0x100, &[1]).unwrap(),
        CanMessage::new_standard(0x200, &[2]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Send some messages
    backend
        .send_message(&CanMessage::new_standard(0x300, &[3]).unwrap())
        .unwrap();
    backend
        .send_message(&CanMessage::new_standard(0x400, &[4]).unwrap())
        .unwrap();

    // Receive preset messages
    let msg1 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg1.id(), CanId::Standard(0x100));

    let msg2 = backend.receive_message().unwrap().unwrap();
    assert_eq!(msg2.id(), CanId::Standard(0x200));

    // Verify sent messages were recorded
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].id(), CanId::Standard(0x300));
    assert_eq!(recorded[1].id(), CanId::Standard(0x400));
}
