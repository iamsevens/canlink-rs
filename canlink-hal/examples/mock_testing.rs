//! Mock Testing Example
//!
//! This example demonstrates how to use the Mock backend for testing CAN applications
//! without physical hardware. It covers error injection, preset messages, and message
//! verification.

use canlink_hal::{BackendConfig, CanBackend, CanError, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CAN Mock Testing Example ===\n");

    // Scenario 1: Basic message recording and verification
    println!("--- Scenario 1: Message Recording ---");
    basic_message_recording()?;
    println!();

    // Scenario 2: Error injection
    println!("--- Scenario 2: Error Injection ---");
    error_injection_testing()?;
    println!();

    // Scenario 3: Preset messages
    println!("--- Scenario 3: Preset Messages ---");
    preset_message_testing()?;
    println!();

    // Scenario 4: Message verification
    println!("--- Scenario 4: Message Verification ---");
    message_verification_testing()?;
    println!();

    // Scenario 5: Protocol testing
    println!("--- Scenario 5: Protocol Testing ---");
    protocol_testing()?;
    println!();

    // Scenario 6: Error recovery testing
    println!("--- Scenario 6: Error Recovery Testing ---");
    error_recovery_testing()?;
    println!();

    println!("=== All scenarios completed successfully ===");
    Ok(())
}

/// Demonstrate basic message recording and verification.
fn basic_message_recording() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating mock backend and sending messages...");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send some messages
    let messages = vec![
        CanMessage::new_standard(0x100, &[0x11, 0x22, 0x33])?,
        CanMessage::new_standard(0x200, &[0x44, 0x55])?,
        CanMessage::new_extended(0x12345678, &[0x66, 0x77, 0x88, 0x99])?,
    ];

    for msg in &messages {
        backend.send_message(msg)?;
        println!(
            "  Sent: ID=0x{:X}, Data={:02X?}",
            msg.id().raw(),
            msg.data()
        );
    }

    // Verify messages were recorded
    let recorded = backend.get_recorded_messages();
    println!("\n✓ Recorded {} messages", recorded.len());

    for (i, msg) in recorded.iter().enumerate() {
        println!(
            "  Message {}: ID=0x{:X}, Data={:02X?}",
            i + 1,
            msg.id().raw(),
            msg.data()
        );
    }

    backend.close()?;
    Ok(())
}

/// Demonstrate error injection for testing error handling.
fn error_injection_testing() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing error injection...");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Configure error injection: fail the 3rd send attempt
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Simulated bus-off condition".to_string(),
        },
        1, // inject once
        2, // skip first 2 calls
    );

    println!("  Configured to fail 3rd send attempt");

    // Attempt to send 5 messages
    for i in 1..=5 {
        let msg = CanMessage::new_standard(0x100, &[i])?;
        match backend.send_message(&msg) {
            Ok(_) => println!("  ✓ Send #{} succeeded", i),
            Err(e) => println!("  ✗ Send #{} failed: {}", i, e),
        }
    }

    // Verify only successful messages were recorded
    let recorded = backend.get_recorded_messages();
    println!(
        "\n✓ Recorded {} successful messages (expected 4)",
        recorded.len()
    );

    backend.close()?;
    Ok(())
}

/// Demonstrate preset messages for testing receive functionality.
fn preset_message_testing() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing preset messages...");

    // Create backend with preset messages
    let preset_messages = vec![
        CanMessage::new_standard(0x111, &[0x01, 0x02])?,
        CanMessage::new_standard(0x222, &[0x03, 0x04, 0x05])?,
        CanMessage::new_standard(0x333, &[0x06])?,
    ];

    println!("  Configured {} preset messages", preset_messages.len());

    let config = MockConfig::with_preset_messages(preset_messages);
    let mut backend = MockBackend::with_config(config);

    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config)?;
    backend.open_channel(0)?;

    // Receive all preset messages
    let mut received_count = 0;
    loop {
        match backend.receive_message()? {
            Some(msg) => {
                received_count += 1;
                println!(
                    "  ✓ Received message {}: ID=0x{:X}, Data={:02X?}",
                    received_count,
                    msg.id().raw(),
                    msg.data()
                );
            }
            None => {
                println!("\n✓ All preset messages received ({})", received_count);
                break;
            }
        }
    }

    backend.close()?;
    Ok(())
}

/// Demonstrate message verification capabilities.
fn message_verification_testing() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing message verification...");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send multiple messages with different IDs
    println!("  Sending messages...");
    backend.send_message(&CanMessage::new_standard(0x100, &[1, 2])?)?;
    backend.send_message(&CanMessage::new_standard(0x200, &[3, 4])?)?;
    backend.send_message(&CanMessage::new_standard(0x100, &[5, 6])?)?;
    backend.send_message(&CanMessage::new_standard(0x300, &[7, 8])?)?;
    backend.send_message(&CanMessage::new_standard(0x100, &[9, 10])?)?;

    // Verify total count
    println!("\n  Verifying message count...");
    if backend.verify_message_count(5) {
        println!("  ✓ Correct total count: 5 messages");
    }

    // Verify specific IDs
    println!("\n  Verifying specific IDs...");
    let ids_to_check = vec![0x100, 0x200, 0x300, 0x400];
    for id in ids_to_check {
        let sent = backend.verify_message_sent(CanId::Standard(id));
        if sent {
            println!("  ✓ ID 0x{:03X} was sent", id);
        } else {
            println!("  ✗ ID 0x{:03X} was not sent", id);
        }
    }

    // Get messages by specific ID
    println!("\n  Getting messages by ID 0x100...");
    let messages_100 = backend.get_messages_by_id(CanId::Standard(0x100));
    println!("  ✓ Found {} messages with ID 0x100", messages_100.len());
    for (i, msg) in messages_100.iter().enumerate() {
        println!("    Message {}: Data={:02X?}", i + 1, msg.data());
    }

    backend.close()?;
    Ok(())
}

/// Demonstrate testing a request-response protocol.
fn protocol_testing() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing OBD-II-like request-response protocol...");

    // Create backend with preset responses
    let responses = vec![
        // Response to PID 0x00 (supported PIDs)
        CanMessage::new_standard(0x7E8, &[0x06, 0x41, 0x00, 0xBE, 0x1F, 0xA8, 0x13])?,
        // Response to PID 0x0C (engine RPM)
        CanMessage::new_standard(0x7E8, &[0x04, 0x41, 0x0C, 0x1A, 0xF8])?,
        // Response to PID 0x0D (vehicle speed)
        CanMessage::new_standard(0x7E8, &[0x03, 0x41, 0x0D, 0x3C])?,
    ];

    let config = MockConfig::with_preset_messages(responses);
    let mut backend = MockBackend::with_config(config);

    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config)?;
    backend.open_channel(0)?;

    // Send requests and receive responses
    println!("\n  Request 1: Supported PIDs (0x00)");
    let request1 = CanMessage::new_standard(0x7DF, &[0x02, 0x01, 0x00])?;
    backend.send_message(&request1)?;
    println!("    Sent: {:02X?}", request1.data());

    if let Some(response) = backend.receive_message()? {
        println!("    Received: {:02X?}", response.data());
        println!("    ✓ Supported PIDs response received");
    }

    println!("\n  Request 2: Engine RPM (0x0C)");
    let request2 = CanMessage::new_standard(0x7DF, &[0x02, 0x01, 0x0C])?;
    backend.send_message(&request2)?;
    println!("    Sent: {:02X?}", request2.data());

    if let Some(response) = backend.receive_message()? {
        println!("    Received: {:02X?}", response.data());
        // Parse RPM: ((A*256)+B)/4
        let rpm = ((response.data()[3] as u16 * 256) + response.data()[4] as u16) / 4;
        println!("    ✓ Engine RPM: {} RPM", rpm);
    }

    println!("\n  Request 3: Vehicle Speed (0x0D)");
    let request3 = CanMessage::new_standard(0x7DF, &[0x02, 0x01, 0x0D])?;
    backend.send_message(&request3)?;
    println!("    Sent: {:02X?}", request3.data());

    if let Some(response) = backend.receive_message()? {
        println!("    Received: {:02X?}", response.data());
        let speed = response.data()[3];
        println!("    ✓ Vehicle Speed: {} km/h", speed);
    }

    // Verify all requests were sent
    println!("\n  Verifying requests...");
    let requests = backend.get_messages_by_id(CanId::Standard(0x7DF));
    println!("  ✓ Sent {} requests", requests.len());

    backend.close()?;
    Ok(())
}

/// Demonstrate testing error recovery scenarios.
fn error_recovery_testing() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing error recovery scenarios...");

    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Simulate intermittent failures
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Bus busy".to_string(),
        },
        3, // fail 3 times
        0, // no skip
    );

    println!("  Configured to fail first 3 send attempts");
    println!("\n  Testing retry logic...");

    let msg = CanMessage::new_standard(0x123, &[0xAA, 0xBB, 0xCC])?;
    let max_retries = 5;
    let mut attempt = 0;
    let mut success = false;

    while attempt < max_retries && !success {
        attempt += 1;
        match backend.send_message(&msg) {
            Ok(_) => {
                println!("  ✓ Attempt {} succeeded", attempt);
                success = true;
            }
            Err(e) => {
                println!("  ✗ Attempt {} failed: {}", attempt, e);
                println!("    Retrying...");
            }
        }
    }

    if success {
        println!("\n✓ Message sent successfully after {} attempts", attempt);
    } else {
        println!("\n✗ Failed to send message after {} attempts", max_retries);
    }

    // Verify message was recorded only once
    if backend.verify_message_count(1) {
        println!("✓ Message recorded exactly once (no duplicates)");
    }

    // Test error clearing
    println!("\n  Testing error clearing...");
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Test error".to_string(),
        });

    let result = backend.send_message(&msg);
    if result.is_err() {
        println!("  ✓ Error injection active");
    }

    backend.error_injector_mut().clear();
    let result = backend.send_message(&msg);
    if result.is_ok() {
        println!("  ✓ Error injection cleared, send succeeded");
    }

    backend.close()?;
    Ok(())
}
