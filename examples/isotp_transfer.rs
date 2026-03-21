//! ISO-TP transfer protocol example.
//!
//! This example demonstrates:
//! - Creating an ISO-TP channel
//! - Sending single-frame and multi-frame messages
//! - Receiving and reassembling multi-frame responses
//! - UDS diagnostic communication patterns
//! - Error handling
//!
//! Run with: `cargo run -p canlink-hal --example isotp_transfer --features "canlink-hal/isotp"`

use canlink_hal::isotp::{AddressingMode, FrameSize, IsoTpChannel, IsoTpConfig, StMin};
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::{MockBackend, MockConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CANLink HAL - ISO-TP Transfer Protocol Example ===\n");

    // Run different examples
    basic_isotp_example().await?;
    println!("\n{}\n", "=".repeat(60));

    uds_diagnostic_example().await?;
    println!("\n{}\n", "=".repeat(60));

    configuration_examples()?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

/// Basic ISO-TP send/receive example
async fn basic_isotp_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Basic ISO-TP Example ---\n");

    // Step 1: Create backend with preset responses
    println!("1. Creating Mock backend with preset ISO-TP responses...");

    // Simulate ECU responses (Single Frame responses)
    let preset_messages = vec![
        // Response to DiagnosticSessionControl (0x10 0x01) -> Positive response
        CanMessage::new_standard(0x7E8, &[0x02, 0x50, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00])?,
        // Response to ReadDataByIdentifier (0x22 0xF1 0x90) -> VIN (multi-frame simulation)
        CanMessage::new_standard(0x7E8, &[0x06, 0x62, 0xF1, 0x90, 0x57, 0x44, 0x42, 0x00])?,
    ];

    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;
    println!("   ✓ Backend created with preset responses\n");

    // Step 2: Configure ISO-TP channel
    println!("2. Configuring ISO-TP channel...");
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0) // Diagnostic request ID
        .rx_id(0x7E8) // Diagnostic response ID
        .block_size(0) // No block size limit
        .st_min(StMin::Milliseconds(10)) // 10ms separation time
        .timeout(Duration::from_millis(1000)) // 1 second timeout
        .build()?;

    println!("   TX ID: 0x{:03X}", config.tx_id);
    println!("   RX ID: 0x{:03X}", config.rx_id);
    println!("   Block Size: {}", config.block_size);
    println!("   STmin: {:?}", config.st_min);
    println!("   Timeout: {:?}", config.rx_timeout);
    println!();

    // Step 3: Create ISO-TP channel
    println!("3. Creating ISO-TP channel...");
    let mut channel = IsoTpChannel::new(backend, config)?;
    println!("   ✓ Channel created\n");

    // Step 4: Send a single-frame message
    println!("4. Sending DiagnosticSessionControl request (Single Frame)...");
    let request = vec![0x10, 0x01]; // DiagnosticSessionControl - Default Session
    println!("   Request: {:02X?}", request);

    channel.send(&request).await?;
    println!("   ✓ Request sent\n");

    // Step 5: Receive response
    println!("5. Receiving response...");
    let response = channel.receive().await?;
    println!("   Response: {:02X?}", response);

    if response.len() >= 2 && response[0] == 0x50 {
        println!("   ✓ Positive response received (Session activated)\n");
    }

    // Step 6: Send another request
    println!("6. Sending ReadDataByIdentifier request...");
    let request2 = vec![0x22, 0xF1, 0x90]; // ReadDataByIdentifier - VIN
    println!("   Request: {:02X?}", request2);

    channel.send(&request2).await?;
    println!("   ✓ Request sent\n");

    // Step 7: Receive VIN response
    println!("7. Receiving VIN response...");
    let response2 = channel.receive().await?;
    println!("   Response: {:02X?}", response2);

    if response2.len() >= 4 && response2[0] == 0x62 {
        println!("   ✓ Positive response received\n");
    }

    println!("--- Basic ISO-TP Example Complete ---");
    Ok(())
}

/// UDS diagnostic communication example
async fn uds_diagnostic_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- UDS Diagnostic Communication Example ---\n");

    // Create backend
    let preset_messages = vec![
        // TesterPresent response
        CanMessage::new_standard(0x7E8, &[0x02, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])?,
        // SecurityAccess seed response
        CanMessage::new_standard(0x7E8, &[0x04, 0x67, 0x01, 0x12, 0x34, 0x00, 0x00, 0x00])?,
        // Negative response (conditions not correct)
        CanMessage::new_standard(0x7E8, &[0x03, 0x7F, 0x27, 0x22, 0x00, 0x00, 0x00, 0x00])?,
    ];

    let mock_config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(mock_config);
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;

    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .timeout(Duration::from_millis(2000))
        .build()?;

    let mut channel = IsoTpChannel::new(backend, config)?;

    // TesterPresent
    println!("1. Sending TesterPresent...");
    let tester_present = vec![0x3E, 0x00];
    channel.send(&tester_present).await?;
    let response = channel.receive().await?;
    print_uds_response("TesterPresent", &response);

    // SecurityAccess - Request Seed
    println!("\n2. Sending SecurityAccess (Request Seed)...");
    let security_seed = vec![0x27, 0x01];
    channel.send(&security_seed).await?;
    let response = channel.receive().await?;
    print_uds_response("SecurityAccess Seed", &response);

    // SecurityAccess - Send Key (will get negative response in mock)
    println!("\n3. Sending SecurityAccess (Send Key)...");
    let security_key = vec![0x27, 0x02, 0xAB, 0xCD];
    channel.send(&security_key).await?;
    let response = channel.receive().await?;
    print_uds_response("SecurityAccess Key", &response);

    println!("\n--- UDS Diagnostic Example Complete ---");
    Ok(())
}

/// Print UDS response with interpretation
fn print_uds_response(service_name: &str, response: &[u8]) {
    print!("   {} Response: {:02X?} - ", service_name, response);

    if response.is_empty() {
        println!("Empty response");
        return;
    }

    match response[0] {
        0x7F => {
            // Negative response
            if response.len() >= 3 {
                let service = response[1];
                let nrc = response[2];
                let nrc_name = match nrc {
                    0x10 => "generalReject",
                    0x12 => "subFunctionNotSupported",
                    0x13 => "incorrectMessageLengthOrInvalidFormat",
                    0x14 => "responseTooLong",
                    0x21 => "busyRepeatRequest",
                    0x22 => "conditionsNotCorrect",
                    0x24 => "requestSequenceError",
                    0x25 => "noResponseFromSubnetComponent",
                    0x26 => "failurePreventsExecutionOfRequestedAction",
                    0x31 => "requestOutOfRange",
                    0x33 => "securityAccessDenied",
                    0x35 => "invalidKey",
                    0x36 => "exceededNumberOfAttempts",
                    0x37 => "requiredTimeDelayNotExpired",
                    0x72 => "generalProgrammingFailure",
                    0x78 => "requestCorrectlyReceivedResponsePending",
                    _ => "unknown",
                };
                println!(
                    "Negative Response (Service=0x{:02X}, NRC=0x{:02X} {})",
                    service, nrc, nrc_name
                );
            } else {
                println!("Negative Response (malformed)");
            }
        }
        sid if sid >= 0x40 => {
            // Positive response (SID + 0x40)
            let original_sid = sid - 0x40;
            println!("Positive Response (Service=0x{:02X})", original_sid);
        }
        _ => {
            println!("Unknown response format");
        }
    }
}

/// Configuration examples showing different ISO-TP settings
fn configuration_examples() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- ISO-TP Configuration Examples ---\n");

    // Example 1: Standard OBD-II configuration
    println!("1. Standard OBD-II Configuration:");
    let obd_config = IsoTpConfig::builder()
        .tx_id(0x7DF) // OBD-II broadcast request
        .rx_id(0x7E8) // ECU response
        .timeout(Duration::from_millis(1000))
        .build()?;
    println!(
        "   TX: 0x{:03X}, RX: 0x{:03X}",
        obd_config.tx_id, obd_config.rx_id
    );

    // Example 2: Extended CAN ID configuration
    println!("\n2. Extended CAN ID Configuration:");
    let extended_config = IsoTpConfig::builder()
        .tx_id(0x18DA00F1) // Extended ID
        .rx_id(0x18DAF100)
        .extended_ids(true)
        .timeout(Duration::from_millis(2000))
        .build()?;
    println!(
        "   TX: 0x{:08X} (extended), RX: 0x{:08X} (extended)",
        extended_config.tx_id, extended_config.rx_id
    );

    // Example 3: High-speed configuration
    println!("\n3. High-Speed Transfer Configuration:");
    let fast_config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .block_size(0) // No limit
        .st_min(StMin::Microseconds(100)) // 100μs separation
        .timeout(Duration::from_millis(5000))
        .build()?;
    println!("   Block Size: {} (unlimited)", fast_config.block_size);
    println!("   STmin: {:?}", fast_config.st_min);

    // Example 4: Conservative/compatible configuration
    println!("\n4. Conservative Configuration (for older ECUs):");
    let slow_config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .block_size(8) // Wait for FC every 8 frames
        .st_min(StMin::Milliseconds(25)) // 25ms separation
        .timeout(Duration::from_millis(10000))
        .max_wait_count(20) // Allow more FC(Wait) responses
        .build()?;
    println!("   Block Size: {}", slow_config.block_size);
    println!("   STmin: {:?}", slow_config.st_min);
    println!("   Max Wait Count: {}", slow_config.max_wait_count);

    // Example 5: CAN-FD configuration
    println!("\n5. CAN-FD Configuration:");
    let fd_config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .frame_size(FrameSize::Fd64) // Force 64-byte frames
        .build()?;
    println!("   Frame Size: {:?}", fd_config.frame_size);

    // Example 6: Extended addressing mode
    println!("\n6. Extended Addressing Mode:");
    let ext_addr_config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .addressing_mode(AddressingMode::Extended {
            target_address: 0x01,
        })
        .build()?;
    println!("   Addressing Mode: {:?}", ext_addr_config.addressing_mode);

    // Example 7: Mixed addressing mode
    println!("\n7. Mixed Addressing Mode:");
    let mixed_config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .addressing_mode(AddressingMode::Mixed {
            address_extension: 0xF1,
        })
        .build()?;
    println!("   Addressing Mode: {:?}", mixed_config.addressing_mode);

    // Example 8: Custom padding
    println!("\n8. Custom Padding Configuration:");
    let padded_config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .padding_enabled(true)
        .padding_byte(0xAA) // Custom padding byte
        .build()?;
    println!("   Padding Enabled: {}", padded_config.padding_enabled);
    println!("   Padding Byte: 0x{:02X}", padded_config.padding_byte);

    println!("\n--- Configuration Examples Complete ---");
    Ok(())
}
