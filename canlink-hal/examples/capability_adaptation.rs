//! Capability Adaptation Example
//!
//! This example demonstrates how to write portable CAN applications that
//! automatically adapt to different hardware capabilities.

use canlink_hal::{BackendConfig, CanBackend, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CAN Hardware Capability Adaptation Example ===\n");

    // Scenario 1: Adaptive message sending with CAN-FD backend
    println!("--- Scenario 1: CAN-FD Backend ---");
    let backend_fd = MockBackend::new();
    demonstrate_adaptive_messaging(&backend_fd)?;
    println!();

    // Scenario 2: Adaptive message sending with CAN 2.0 backend
    println!("--- Scenario 2: CAN 2.0 Backend ---");
    let config_20 = MockConfig::can20_only();
    let backend_20 = MockBackend::with_config(config_20);
    demonstrate_adaptive_messaging(&backend_20)?;
    println!();

    // Scenario 3: Multi-channel adaptation
    println!("--- Scenario 3: Multi-Channel Adaptation ---");
    demonstrate_multi_channel_adaptation()?;
    println!();

    // Scenario 4: Bitrate selection
    println!("--- Scenario 4: Bitrate Selection ---");
    demonstrate_bitrate_selection()?;
    println!();

    // Scenario 5: Complete application workflow
    println!("--- Scenario 5: Complete Application Workflow ---");
    demonstrate_complete_workflow()?;

    Ok(())
}

/// Demonstrate adaptive message sending based on CAN-FD support.
fn demonstrate_adaptive_messaging(backend: &MockBackend) -> Result<(), Box<dyn std::error::Error>> {
    let capability = backend.get_capability()?;

    println!("Hardware capabilities:");
    println!("  CAN-FD: {}", capability.supports_canfd);
    println!("  Channels: {}", capability.channel_count);

    // Prepare data to send (12 bytes)
    let data = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
    ];

    // Adapt message type based on capability
    let message = if capability.supports_canfd {
        println!("\n✓ Using CAN-FD for 12-byte message");
        CanMessage::new_fd(CanId::Standard(0x123), &data)?
    } else {
        println!("\n✗ CAN-FD not available, splitting into CAN 2.0 messages");
        println!("  Sending first 8 bytes only (or split into multiple messages)");
        CanMessage::new_standard(0x123, &data[..8])?
    };

    println!("Message created:");
    println!("  ID: 0x{:03X}", message.id().raw());
    println!("  Data length: {} bytes", message.data().len());
    println!("  Data: {:02X?}", message.data());

    Ok(())
}

/// Demonstrate multi-channel adaptation.
fn demonstrate_multi_channel_adaptation() -> Result<(), Box<dyn std::error::Error>> {
    // Create backends with different channel counts
    let mut config_single = MockConfig::can20_only();
    config_single.channel_count = 1;
    let mut backend_single = MockBackend::with_config(config_single);

    let mut config_multi = MockConfig::new();
    config_multi.channel_count = 4;
    let mut backend_multi = MockBackend::with_config(config_multi);

    // Initialize both backends
    let config = BackendConfig::new("mock");
    backend_single.initialize(&config)?;
    backend_multi.initialize(&config)?;

    // Adapt to single-channel backend
    println!("Single-channel backend:");
    let cap_single = backend_single.get_capability()?;
    println!("  Available channels: {}", cap_single.channel_count);

    for channel in 0..cap_single.channel_count {
        backend_single.open_channel(channel)?;
        println!("  ✓ Opened channel {}", channel);
    }

    // Adapt to multi-channel backend
    println!("\nMulti-channel backend:");
    let cap_multi = backend_multi.get_capability()?;
    println!("  Available channels: {}", cap_multi.channel_count);

    for channel in 0..cap_multi.channel_count {
        backend_multi.open_channel(channel)?;
        println!("  ✓ Opened channel {}", channel);
    }

    Ok(())
}

/// Demonstrate bitrate selection based on supported bitrates.
fn demonstrate_bitrate_selection() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MockBackend::new();
    let capability = backend.get_capability()?;

    println!("Supported bitrates:");
    for bitrate in &capability.supported_bitrates {
        println!("  - {} bps", bitrate);
    }

    // Application's preferred bitrates (in order of preference)
    let preferred_bitrates = vec![1_000_000, 800_000, 500_000, 250_000, 125_000];

    println!("\nPreferred bitrates: {:?}", preferred_bitrates);

    // Select the first supported bitrate
    let selected_bitrate = preferred_bitrates
        .iter()
        .find(|&&bitrate| capability.supports_bitrate(bitrate))
        .copied();

    match selected_bitrate {
        Some(bitrate) => {
            println!("✓ Selected bitrate: {} bps", bitrate);
        }
        None => {
            println!("✗ No preferred bitrate is supported!");
            println!(
                "  Falling back to first available: {} bps",
                capability.supported_bitrates[0]
            );
        }
    }

    Ok(())
}

/// Demonstrate a complete application workflow with capability adaptation.
fn demonstrate_complete_workflow() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting CAN application with capability adaptation...\n");

    // Step 1: Create and initialize backend
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    println!("✓ Backend initialized");

    // Step 2: Query capabilities
    let capability = backend.get_capability()?;
    println!("✓ Capabilities queried");
    println!("  - Channels: {}", capability.channel_count);
    println!("  - CAN-FD: {}", capability.supports_canfd);
    println!("  - Max bitrate: {} bps", capability.max_bitrate);

    // Step 3: Validate and open channel
    let desired_channel = 0;
    if !capability.has_channel(desired_channel) {
        return Err(format!("Channel {} not available", desired_channel).into());
    }
    backend.open_channel(desired_channel)?;
    println!("✓ Channel {} opened", desired_channel);

    // Step 4: Prepare messages with adaptive data length
    let max_data_len = if capability.supports_canfd { 64 } else { 8 };
    println!("✓ Max data length: {} bytes", max_data_len);

    // Step 5: Send messages
    let test_data = vec![0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA];

    for chunk in test_data.chunks(max_data_len) {
        let message = if capability.supports_canfd && chunk.len() > 8 {
            CanMessage::new_fd(CanId::Standard(0x100), chunk)?
        } else {
            CanMessage::new_standard(0x100, chunk)?
        };

        backend.send_message(&message)?;
        println!("✓ Sent message: {} bytes", chunk.len());
    }

    // Step 6: Verify with message recorder
    let recorded = backend.get_recorded_messages();
    println!("✓ Recorded {} messages", recorded.len());

    // Step 7: Handle timestamps if supported
    if capability.timestamp_precision.is_supported() {
        println!(
            "✓ Timestamps available (precision: {:?})",
            capability.timestamp_precision
        );
        for (i, msg) in recorded.iter().enumerate() {
            if let Some(ts) = msg.timestamp() {
                println!("  Message {}: timestamp = {} µs", i, ts.as_micros());
            }
        }
    } else {
        println!("✗ Timestamps not supported, using system time");
    }

    // Step 8: Cleanup
    backend.close()?;
    println!("✓ Backend closed");

    println!("\n=== Application completed successfully ===");

    Ok(())
}
