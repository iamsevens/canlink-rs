//! Basic usage example for CANLink HAL with Mock backend.
//!
//! This example demonstrates:
//! - Creating and initializing a Mock backend
//! - Opening a CAN channel
//! - Sending CAN messages
//! - Receiving preset messages
//! - Verifying recorded messages
//! - Proper cleanup

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - Basic Usage Example ===\n");

    // Step 1: Create a Mock backend with preset messages
    println!("1. Creating Mock backend with preset messages...");
    let preset_messages = vec![
        CanMessage::new_standard(0x100, &[0x01, 0x02, 0x03, 0x04])?,
        CanMessage::new_standard(0x200, &[0x05, 0x06, 0x07, 0x08])?,
    ];
    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    println!("   ✓ Backend created: {}", backend.name());
    println!("   ✓ Version: {}\n", backend.version());

    // Step 2: Initialize the backend
    println!("2. Initializing backend...");
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    println!("   ✓ Backend initialized\n");

    // Step 3: Query hardware capabilities
    println!("3. Querying hardware capabilities...");
    let capability = backend.get_capability()?;
    println!("   ✓ Channels: {}", capability.channel_count);
    println!("   ✓ CAN-FD support: {}", capability.supports_canfd);
    println!("   ✓ Max bitrate: {} bps", capability.max_bitrate);
    println!(
        "   ✓ Supported bitrates: {:?}\n",
        capability.supported_bitrates
    );

    // Step 4: Open a CAN channel
    println!("4. Opening CAN channel 0...");
    backend.open_channel(0)?;
    println!("   ✓ Channel 0 opened\n");

    // Step 5: Send CAN messages
    println!("5. Sending CAN messages...");
    let messages_to_send = [
        CanMessage::new_standard(0x123, &[0x11, 0x22, 0x33])?,
        CanMessage::new_extended(0x12345678, &[0x44, 0x55, 0x66, 0x77])?,
        CanMessage::new_standard(0x456, &[0x88, 0x99, 0xAA, 0xBB, 0xCC])?,
    ];

    for (i, msg) in messages_to_send.iter().enumerate() {
        backend.send_message(msg)?;
        println!("   ✓ Sent message {}: ID={:?}", i + 1, msg.id());
    }
    println!();

    // Step 6: Receive preset messages
    println!("6. Receiving preset messages...");
    let mut received_count = 0;
    while let Some(msg) = backend.receive_message()? {
        received_count += 1;
        println!(
            "   ✓ Received message {}: ID={:?}, Data={:?}",
            received_count,
            msg.id(),
            msg.data()
        );
    }
    println!("   ✓ Total received: {} messages\n", received_count);

    // Step 7: Verify recorded messages
    println!("7. Verifying recorded messages...");
    let recorded = backend.get_recorded_messages();
    println!("   ✓ Total recorded: {} messages", recorded.len());
    for (i, msg) in recorded.iter().enumerate() {
        println!(
            "   ✓ Recorded message {}: ID={:?}, Data={:?}",
            i + 1,
            msg.id(),
            msg.data()
        );
    }
    println!();

    // Step 8: Verify specific messages
    println!("8. Checking for specific message IDs...");
    if recorded
        .iter()
        .any(|msg| msg.id() == CanId::Standard(0x123))
    {
        println!("   ✓ Found message with ID 0x123");
    }
    if recorded
        .iter()
        .any(|msg| msg.id() == CanId::Extended(0x12345678))
    {
        println!("   ✓ Found message with ID 0x12345678");
    }
    println!();

    // Step 9: Close the channel
    println!("9. Closing CAN channel...");
    backend.close_channel(0)?;
    println!("   ✓ Channel 0 closed\n");

    // Step 10: Close the backend
    println!("10. Closing backend...");
    backend.close()?;
    println!("   ✓ Backend closed\n");

    println!("=== Example completed successfully! ===");
    Ok(())
}
