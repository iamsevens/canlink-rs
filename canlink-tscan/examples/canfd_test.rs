//! CAN-FD hardware verification test.
//!
//! This example tests CAN-FD message transmission and reception with real `LibTSCAN`-backed hardware.
//! It requires a connected `TSMaster` device that supports CAN-FD.
//!
//! # Test Scenarios
//!
//! 1. Standard CAN 2.0 messages (baseline)
//! 2. CAN-FD messages with various data lengths (12, 16, 24, 32, 48, 64 bytes)
//! 3. CAN-FD with BRS (Bit Rate Switch) flag
//! 4. Mixed CAN 2.0 and CAN-FD traffic
//!
//! # Hardware Requirements
//!
//! - `LibTSCAN`-compatible CAN-FD capable device (validated in this repository on TOSUN-related devices)
//! - Properly terminated CAN bus
//! - Optional: Second CAN device for loopback testing

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_tscan::TSCanBackend;
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 CAN-FD Hardware Verification Test\n");
    println!("=====================================\n");

    // Create and initialize backend
    println!("1. Initializing TSCanBackend...");
    let mut backend = TSCanBackend::new();
    let config = BackendConfig::new("tscan");
    backend.initialize(&config)?;
    println!("   ✓ Backend initialized\n");

    // Query capabilities
    println!("2. Checking CAN-FD support...");
    let capability = backend.get_capability()?;
    println!("   ✓ Channel count: {}", capability.channel_count);
    println!("   ✓ CAN-FD support: {}", capability.supports_canfd);
    println!(
        "   ✓ Timestamp precision: {:?}\n",
        capability.timestamp_precision
    );

    if !capability.supports_canfd {
        println!("❌ ERROR: Hardware does not support CAN-FD");
        println!("   This test requires CAN-FD capable hardware.\n");
        backend.close()?;
        return Ok(());
    }

    // Open channel
    println!("3. Opening channel 0...");
    backend.open_channel(0)?;
    println!("   ✓ Channel 0 opened\n");

    // Test 1: Standard CAN 2.0 message (baseline)
    println!("4. Test 1: Standard CAN 2.0 message (8 bytes)");
    let can20_msg =
        CanMessage::new_standard(0x100, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])?;
    backend.send_message(&can20_msg)?;
    println!(
        "   ✓ Sent CAN 2.0: ID=0x{:03X}, DLC={}, Data={:02X?}\n",
        can20_msg.id().raw(),
        can20_msg.data().len(),
        can20_msg.data()
    );

    // Test 2: CAN-FD messages with various data lengths
    println!("5. Test 2: CAN-FD messages with various data lengths");

    let test_lengths = vec![
        (12, "12 bytes (DLC=9)"),
        (16, "16 bytes (DLC=10)"),
        (24, "24 bytes (DLC=12)"),
        (32, "32 bytes (DLC=13)"),
        (48, "48 bytes (DLC=14)"),
        (64, "64 bytes (DLC=15)"),
    ];

    for (len, _desc) in test_lengths {
        let data: Vec<u8> = (0..len).map(|i| (i % 256) as u8).collect();
        let canfd_msg = CanMessage::new_fd(CanId::Standard(0x200), &data)?;

        backend.send_message(&canfd_msg)?;
        println!(
            "   ✓ Sent CAN-FD: ID=0x{:03X}, {} bytes, Data={:02X?}...",
            canfd_msg.id().raw(),
            canfd_msg.data().len(),
            &canfd_msg.data()[..8.min(canfd_msg.data().len())]
        );

        thread::sleep(Duration::from_millis(10));
    }
    println!();

    // Test 3: CAN-FD with BRS flag
    println!("6. Test 3: CAN-FD with BRS (Bit Rate Switch)");
    let data_32: Vec<u8> = (0..32).map(|i| i as u8).collect();
    let canfd_brs_msg = CanMessage::new_fd(CanId::Standard(0x300), &data_32)?;
    // Note: BRS flag is automatically set by new_fd()

    backend.send_message(&canfd_brs_msg)?;
    println!(
        "   ✓ Sent CAN-FD+BRS: ID=0x{:03X}, {} bytes, Flags={:?}\n",
        canfd_brs_msg.id().raw(),
        canfd_brs_msg.data().len(),
        canfd_brs_msg.flags()
    );

    // Test 4: Extended ID CAN-FD message
    println!("7. Test 4: Extended ID CAN-FD message");
    let data_48: Vec<u8> = (0..48).map(|i| (i * 2) as u8).collect();
    let canfd_ext_msg = CanMessage::new_fd(CanId::Extended(0x12345678), &data_48)?;

    backend.send_message(&canfd_ext_msg)?;
    println!(
        "   ✓ Sent CAN-FD Extended: ID=0x{:08X}, {} bytes\n",
        canfd_ext_msg.id().raw(),
        canfd_ext_msg.data().len()
    );

    // Test 5: Receive messages (both CAN 2.0 and CAN-FD)
    println!("8. Test 5: Receiving messages (10 seconds)...");
    println!("   Listening for both CAN 2.0 and CAN-FD messages...\n");

    let start = std::time::Instant::now();
    let mut can20_count = 0;
    let mut canfd_count = 0;
    let mut last_print = std::time::Instant::now();

    while start.elapsed() < Duration::from_secs(10) {
        if let Some(msg) = backend.receive_message()? {
            if msg.is_fd() {
                canfd_count += 1;
                if canfd_count <= 5 {
                    println!(
                        "   [CAN-FD #{}] ID=0x{:03X}, DLC={}, Flags={:?}, Data={:02X?}...",
                        canfd_count,
                        msg.id().raw(),
                        msg.data().len(),
                        msg.flags(),
                        &msg.data()[..8.min(msg.data().len())]
                    );
                }
            } else {
                can20_count += 1;
                if can20_count <= 5 {
                    println!(
                        "   [CAN 2.0 #{}] ID=0x{:03X}, DLC={}, Data={:02X?}",
                        can20_count,
                        msg.id().raw(),
                        msg.data().len(),
                        msg.data()
                    );
                }
            }
        } else {
            thread::sleep(Duration::from_millis(10));
        }

        // Print progress every 2 seconds
        if last_print.elapsed() >= Duration::from_secs(2) {
            println!(
                "   ... CAN 2.0: {}, CAN-FD: {} messages",
                can20_count, canfd_count
            );
            last_print = std::time::Instant::now();
        }
    }

    println!("\n   ✓ Total received:");
    println!("     - CAN 2.0: {} messages", can20_count);
    println!("     - CAN-FD: {} messages\n", canfd_count);

    // Close channel and backend
    println!("9. Cleaning up...");
    backend.close_channel(0)?;
    backend.close()?;
    println!("   ✓ Backend closed\n");

    // Summary
    println!("=====================================");
    println!("✅ CAN-FD Hardware Test Complete!\n");

    println!("📊 Test Summary:");
    println!("   ✓ CAN 2.0 baseline: PASSED");
    println!("   ✓ CAN-FD data lengths (12-64 bytes): PASSED");
    println!("   ✓ CAN-FD with BRS flag: PASSED");
    println!("   ✓ Extended ID CAN-FD: PASSED");
    println!(
        "   ✓ Message reception: {} CAN 2.0, {} CAN-FD\n",
        can20_count, canfd_count
    );

    if can20_count == 0 && canfd_count == 0 {
        println!("⚠️  Note: No messages received. This is normal if:");
        println!("   - The CAN bus is idle");
        println!("   - No other devices are transmitting");
        println!("   - The bus is not properly terminated");
        println!("   - You need a loopback device for self-testing\n");
    }

    println!("💡 Next Steps:");
    println!("   1. Verify messages on CAN analyzer");
    println!("   2. Test with loopback device");
    println!("   3. Measure throughput with continuous traffic");
    println!("   4. Test error handling scenarios\n");

    Ok(())
}
