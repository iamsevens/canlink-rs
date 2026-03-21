//! Async API usage example for CANLink.
//!
//! This example demonstrates how to use the async API for CAN communication.
//!
//! Run with: `cargo run -p canlink-hal --example async_usage --features "canlink-hal/async-tokio"`

use canlink_hal::{BackendConfig, CanBackend, CanBackendAsync, CanMessage, CanResult};
use canlink_mock::{MockBackend, MockConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> CanResult<()> {
    println!("=== CANLink Async API Example ===\n");

    // Example 1: Basic async send/receive
    basic_async_example().await?;

    // Example 2: Async receive with timeout
    timeout_example().await?;

    // Example 3: Concurrent message sending
    concurrent_example().await?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

/// Basic async send and receive operations
async fn basic_async_example() -> CanResult<()> {
    println!("--- Example 1: Basic Async Send/Receive ---");

    // Create backend with preset messages for receiving
    let preset_messages = vec![
        CanMessage::new_standard(0x100, &[0x01, 0x02, 0x03, 0x04])?,
        CanMessage::new_standard(0x200, &[0xAA, 0xBB, 0xCC, 0xDD])?,
    ];
    let config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(config);

    // Initialize
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config)?;
    backend.open_channel(0)?;

    // Send messages asynchronously
    let msg1 = CanMessage::new_standard(0x123, &[0x11, 0x22, 0x33])?;
    backend.send_message_async(&msg1).await?;
    println!("Sent: ID=0x123, Data=[0x11, 0x22, 0x33]");

    let msg2 = CanMessage::new_extended(0x1234_5678, &[0x44, 0x55])?;
    backend.send_message_async(&msg2).await?;
    println!("Sent: ID=0x12345678 (extended), Data=[0x44, 0x55]");

    // Receive messages asynchronously (non-blocking)
    while let Some(msg) = backend.receive_message_async(None).await? {
        println!("Received: ID={:?}, Data={:?}", msg.id(), msg.data());
    }

    // Verify sent messages
    let recorded = backend.get_recorded_messages();
    println!("Total messages sent: {}", recorded.len());

    backend.close()?;
    println!();
    Ok(())
}

/// Async receive with timeout
async fn timeout_example() -> CanResult<()> {
    println!("--- Example 2: Async Receive with Timeout ---");

    // Create backend with one preset message
    let preset_messages = vec![CanMessage::new_standard(0x300, &[0x30])?];
    let config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(config);

    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config)?;
    backend.open_channel(0)?;

    // Receive with timeout - should succeed immediately
    println!("Waiting for message with 100ms timeout...");
    match backend
        .receive_message_async(Some(Duration::from_millis(100)))
        .await?
    {
        Some(msg) => println!("Received: ID={:?}, Data={:?}", msg.id(), msg.data()),
        None => println!("Timeout - no message received"),
    }

    // Receive again - should timeout since no more messages
    println!("Waiting for another message with 50ms timeout...");
    let start = std::time::Instant::now();
    match backend
        .receive_message_async(Some(Duration::from_millis(50)))
        .await?
    {
        Some(msg) => println!("Received: ID={:?}", msg.id()),
        None => println!("Timeout after {:?} - no message available", start.elapsed()),
    }

    backend.close()?;
    println!();
    Ok(())
}

/// Concurrent message sending from multiple tasks
async fn concurrent_example() -> CanResult<()> {
    println!("--- Example 3: Concurrent Message Sending ---");

    use std::sync::Arc;
    use tokio::sync::Mutex;

    let mut backend = MockBackend::new();
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config)?;
    backend.open_channel(0)?;

    let backend = Arc::new(Mutex::new(backend));

    // Spawn multiple tasks to send messages concurrently
    let mut handles = vec![];

    for task_id in 0..4u16 {
        let backend_clone = Arc::clone(&backend);
        let handle = tokio::spawn(async move {
            for msg_id in 0..5u16 {
                let can_id = 0x100 + task_id * 0x10 + msg_id;
                let msg = CanMessage::new_standard(can_id, &[task_id as u8, msg_id as u8]).unwrap();

                let mut backend = backend_clone.lock().await;
                backend.send_message_async(&msg).await.unwrap();
                println!("Task {} sent message with ID 0x{:03X}", task_id, can_id);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // Verify all messages were sent
    let backend = backend.lock().await;
    let total = backend.get_recorded_messages().len();
    println!("\nTotal messages sent concurrently: {}", total);
    assert_eq!(total, 20, "Expected 20 messages (4 tasks × 5 messages)");

    println!();
    Ok(())
}
