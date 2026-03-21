//! Capability Query Example
//!
//! This example demonstrates how to query and display hardware capabilities
//! from different CAN backends.

use canlink_hal::CanBackend;
use canlink_mock::{MockBackend, MockConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CAN Hardware Capability Query Example ===\n");

    // Query capabilities for different backend configurations

    // Backend 1: CAN-FD capable backend
    query_backend_capability("CAN-FD Capable", MockBackend::new())?;

    // Backend 2: CAN 2.0 only backend
    let config_20 = MockConfig::can20_only();
    query_backend_capability("CAN 2.0 Only", MockBackend::with_config(config_20))?;

    // Backend 3: Multi-channel backend
    let mut config_multi = MockConfig::new();
    config_multi.channel_count = 4;
    query_backend_capability("Multi-Channel", MockBackend::with_config(config_multi))?;

    // Demonstrate capability-based decision making
    println!("\n=== Capability-Based Decisions ===\n");
    demonstrate_capability_decisions()?;

    Ok(())
}

/// Query and display capability information for a backend.
fn query_backend_capability(
    name: &str,
    backend: MockBackend,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Backend: {} ---", name);

    let capability = backend.get_capability()?;

    // Display basic information
    println!("Channels:           {}", capability.channel_count);
    println!("CAN-FD Support:     {}", capability.supports_canfd);
    println!("Max Bitrate:        {} bps", capability.max_bitrate);
    println!("Hardware Filters:   {}", capability.filter_count);

    // Display timestamp precision
    print!("Timestamp Precision: ");
    match capability.timestamp_precision {
        canlink_hal::TimestampPrecision::Microsecond => {
            println!("Microsecond (1 µs)");
        }
        canlink_hal::TimestampPrecision::Millisecond => {
            println!("Millisecond (1 ms)");
        }
        canlink_hal::TimestampPrecision::None => {
            println!("Not supported");
        }
    }

    // Display supported bitrates
    println!("Supported Bitrates:");
    for bitrate in &capability.supported_bitrates {
        println!("  - {} bps ({} kbps)", bitrate, bitrate / 1000);
    }

    // Display channel information
    println!("Available Channels:");
    for channel in 0..capability.channel_count {
        println!("  - Channel {}", channel);
    }

    println!();
    Ok(())
}

/// Demonstrate capability-based decision making.
fn demonstrate_capability_decisions() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MockBackend::new();
    let capability = backend.get_capability()?;

    // Decision 1: Message type selection
    println!("Decision 1: Message Type Selection");
    if capability.supports_canfd {
        println!("  ✓ CAN-FD is supported");
        println!("  → Can send messages up to 64 bytes");
    } else {
        println!("  ✗ CAN-FD not supported");
        println!("  → Limited to 8 bytes per message");
    }
    println!();

    // Decision 2: Bitrate selection
    println!("Decision 2: Bitrate Selection");
    let desired_bitrates = vec![1_000_000, 500_000, 250_000];
    println!("  Desired bitrates: {:?}", desired_bitrates);

    for bitrate in desired_bitrates {
        if capability.supports_bitrate(bitrate) {
            println!("  ✓ {} bps is supported", bitrate);
        } else {
            println!("  ✗ {} bps is NOT supported", bitrate);
        }
    }
    println!();

    // Decision 3: Channel allocation
    println!("Decision 3: Channel Allocation");
    println!("  Available channels: {}", capability.channel_count);
    let requested_channels = 3;
    if requested_channels <= capability.channel_count {
        println!("  ✓ Can allocate {} channels", requested_channels);
    } else {
        println!(
            "  ✗ Cannot allocate {} channels (only {} available)",
            requested_channels, capability.channel_count
        );
    }
    println!();

    // Decision 4: Filter usage
    println!("Decision 4: Hardware Filter Usage");
    println!("  Available filters: {}", capability.filter_count);
    let requested_filters = 10;
    let allocated_filters = requested_filters.min(capability.filter_count);
    println!("  Requested: {}", requested_filters);
    println!("  Allocated: {}", allocated_filters);
    println!();

    // Decision 5: Timestamp handling
    println!("Decision 5: Timestamp Handling");
    if capability.timestamp_precision.is_supported() {
        println!("  ✓ Timestamps are supported");
        if let Some(resolution) = capability.timestamp_precision.resolution_us() {
            println!("  → Resolution: {} µs", resolution);
        }
    } else {
        println!("  ✗ Timestamps not supported");
        println!("  → Will use system time instead");
    }
    println!();

    Ok(())
}
