//! Async API tests for CAN hardware abstraction layer.
//!
//! These tests verify the async functionality when the `async` feature is enabled.
//! Run with: `cargo test --features async-tokio`

#![cfg(feature = "async")]

use canlink_hal::{BackendConfig, CanBackend, CanBackendAsync, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use std::time::Duration;

/// Helper to create and initialize a mock backend
fn create_initialized_backend() -> MockBackend {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();
    backend
}

/// Helper to create a backend with preset messages
fn create_backend_with_messages(messages: Vec<CanMessage>) -> MockBackend {
    let config = MockConfig::with_preset_messages(messages);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();
    backend
}

#[tokio::test]
async fn test_send_message_async() {
    let mut backend = create_initialized_backend();

    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04]).unwrap();
    let result = backend.send_message_async(&msg).await;

    assert!(result.is_ok());
    assert!(backend.verify_message_sent(canlink_hal::CanId::Standard(0x123)));
}

#[tokio::test]
async fn test_send_multiple_messages_async() {
    let mut backend = create_initialized_backend();

    for i in 0..10u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        backend.send_message_async(&msg).await.unwrap();
    }

    assert!(backend.verify_message_count(10));
}

#[tokio::test]
async fn test_receive_message_async_no_timeout() {
    let preset = vec![
        CanMessage::new_standard(0x111, &[0x11]).unwrap(),
        CanMessage::new_standard(0x222, &[0x22]).unwrap(),
    ];
    let mut backend = create_backend_with_messages(preset);

    // Receive without timeout (non-blocking)
    let msg1 = backend.receive_message_async(None).await.unwrap();
    assert!(msg1.is_some());
    assert_eq!(msg1.unwrap().id(), canlink_hal::CanId::Standard(0x111));

    let msg2 = backend.receive_message_async(None).await.unwrap();
    assert!(msg2.is_some());
    assert_eq!(msg2.unwrap().id(), canlink_hal::CanId::Standard(0x222));

    // No more messages
    let msg3 = backend.receive_message_async(None).await.unwrap();
    assert!(msg3.is_none());
}

#[tokio::test]
async fn test_receive_message_async_with_timeout_success() {
    let preset = vec![CanMessage::new_standard(0x333, &[0x33]).unwrap()];
    let mut backend = create_backend_with_messages(preset);

    // Receive with timeout - should succeed immediately
    let result = backend
        .receive_message_async(Some(Duration::from_millis(100)))
        .await
        .unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().id(), canlink_hal::CanId::Standard(0x333));
}

#[tokio::test]
async fn test_receive_message_async_with_timeout_expires() {
    let mut backend = create_initialized_backend();
    // No preset messages, so receive should timeout

    let start = std::time::Instant::now();
    let result = backend
        .receive_message_async(Some(Duration::from_millis(50)))
        .await
        .unwrap();
    let elapsed = start.elapsed();

    assert!(result.is_none());
    // Should have waited approximately the timeout duration
    assert!(elapsed >= Duration::from_millis(45)); // Allow some tolerance
    assert!(elapsed < Duration::from_millis(200)); // But not too long
}

#[tokio::test]
async fn test_async_send_receive_roundtrip() {
    let mut backend = create_initialized_backend();

    // Send a message
    let sent_msg = CanMessage::new_standard(0x456, &[0x44, 0x55, 0x66]).unwrap();
    backend.send_message_async(&sent_msg).await.unwrap();

    // Verify it was recorded
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].id(), canlink_hal::CanId::Standard(0x456));
    assert_eq!(recorded[0].data(), &[0x44, 0x55, 0x66]);
}

#[tokio::test]
async fn test_async_canfd_message() {
    let mut backend = create_initialized_backend();

    // Send CAN-FD message with 64 bytes
    let fd_data: Vec<u8> = (0..64).collect();
    let fd_msg = CanMessage::new_fd(canlink_hal::CanId::Standard(0x789), &fd_data).unwrap();

    let result = backend.send_message_async(&fd_msg).await;
    assert!(result.is_ok());

    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].data().len(), 64);
}

#[tokio::test]
async fn test_async_extended_id() {
    let mut backend = create_initialized_backend();

    let msg = CanMessage::new_extended(0x1234_5678, &[0xAA, 0xBB]).unwrap();
    backend.send_message_async(&msg).await.unwrap();

    assert!(backend.verify_message_sent(canlink_hal::CanId::Extended(0x1234_5678)));
}

#[tokio::test]
async fn test_async_concurrent_operations() {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let backend = Arc::new(Mutex::new(create_initialized_backend()));

    // Spawn multiple tasks that send messages
    let mut handles = vec![];

    for i in 0..5u16 {
        let backend_clone = Arc::clone(&backend);
        let handle = tokio::spawn(async move {
            let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
            let mut backend = backend_clone.lock().await;
            backend.send_message_async(&msg).await
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    // Verify all messages were sent
    let backend = backend.lock().await;
    assert!(backend.verify_message_count(5));
}
