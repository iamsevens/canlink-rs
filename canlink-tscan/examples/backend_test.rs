//! Integration test for TSCanBackend with real hardware.
//!
//! This example demonstrates using the TSCanBackend through the CanBackend trait.
//! It requires a connected `TSMaster` device to run successfully.

use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_tscan::TSCanBackend;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TSCanBackend Integration Test\n");
    println!("=====================================\n");

    // Create backend
    println!("1. Creating TSCanBackend...");
    let mut backend = TSCanBackend::new();
    println!("   ✓ Backend created: {}", backend.name());
    println!("   ✓ Version: {}\n", backend.version());

    // Initialize
    println!("2. Initializing backend...");
    let config = BackendConfig::new("tscan");
    backend.initialize(&config)?;
    println!("   ✓ Backend initialized\n");

    // Query capabilities
    println!("3. Querying hardware capabilities...");
    let capability = backend.get_capability()?;
    println!("   ✓ Channel count: {}", capability.channel_count);
    println!("   ✓ CAN-FD support: {}", capability.supports_canfd);
    println!("   ✓ Max bitrate: {} bps", capability.max_bitrate);
    println!(
        "   ✓ Supported bitrates: {:?}",
        capability.supported_bitrates
    );
    println!("   ✓ Filter count: {}", capability.filter_count);
    println!(
        "   ✓ Timestamp precision: {:?}\n",
        capability.timestamp_precision
    );

    // Open channel
    println!("4. Opening channel 0...");
    backend.open_channel(0)?;
    println!("   ✓ Channel 0 opened (configured at 500 kbps)\n");

    // Send test message
    println!("5. Sending test message...");
    let test_msg =
        CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])?;
    backend.send_message(&test_msg)?;
    println!(
        "   ✓ Message sent: ID=0x{:03X}, Data={:02X?}\n",
        test_msg.id().raw(),
        test_msg.data()
    );

    // Receive messages
    println!("6. Receiving messages (5 seconds)...");
    let start = std::time::Instant::now();
    let mut count = 0;
    let mut last_print = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(5) {
        if let Some(msg) = backend.receive_message()? {
            count += 1;

            // Print first 5 messages and then every 100th message
            if count <= 5 || count % 100 == 0 {
                println!(
                    "   [{}] ID=0x{:03X}, DLC={}, Data={:02X?}, Time={:?}",
                    count,
                    msg.id().raw(),
                    msg.data().len(),
                    msg.data(),
                    msg.timestamp()
                );
            }
        } else {
            // No message available, sleep briefly
            thread::sleep(Duration::from_millis(10));
        }

        // Print progress every second
        if last_print.elapsed() >= Duration::from_secs(1) {
            println!("   ... received {} messages so far", count);
            last_print = std::time::Instant::now();
        }
    }

    println!("\n   ✓ Total received: {} messages\n", count);

    // Close channel
    println!("7. Closing channel 0...");
    backend.close_channel(0)?;
    println!("   ✓ Channel closed\n");

    // Close backend
    println!("8. Closing backend...");
    backend.close()?;
    println!("   ✓ Backend closed\n");

    println!("=====================================");
    println!("✅ All tests passed!\n");

    if count == 0 {
        println!("⚠️  Note: No messages received. This is normal if:");
        println!("   - The CAN bus is idle");
        println!("   - No other devices are transmitting");
        println!("   - The bus is not properly terminated");
    }

    Ok(())
}
