//! Automated Testing Example
//!
//! This example demonstrates how to build automated test suites for CAN applications
//! using the Mock backend. It shows patterns for test organization, assertion helpers,
//! and comprehensive test coverage.

use canlink_hal::{BackendConfig, CanBackend, CanError, CanId, CanMessage};
use canlink_mock::{MockBackend, MockConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== CAN Automated Testing Suite ===\n");

    let mut passed = 0;
    let mut failed = 0;

    // Run test suite
    println!("Running test suite...\n");

    run_test(
        "Basic Send/Receive",
        test_basic_send_receive,
        &mut passed,
        &mut failed,
    );
    run_test(
        "Error Handling",
        test_error_handling,
        &mut passed,
        &mut failed,
    );
    run_test(
        "Message Filtering",
        test_message_filtering,
        &mut passed,
        &mut failed,
    );
    run_test(
        "CAN-FD Support",
        test_canfd_support,
        &mut passed,
        &mut failed,
    );
    run_test("Extended IDs", test_extended_ids, &mut passed, &mut failed);
    run_test(
        "Remote Frames",
        test_remote_frames,
        &mut passed,
        &mut failed,
    );
    run_test(
        "Channel Management",
        test_channel_management,
        &mut passed,
        &mut failed,
    );
    run_test(
        "State Transitions",
        test_state_transitions,
        &mut passed,
        &mut failed,
    );
    run_test(
        "Burst Traffic",
        test_burst_traffic,
        &mut passed,
        &mut failed,
    );
    run_test(
        "Error Recovery",
        test_error_recovery,
        &mut passed,
        &mut failed,
    );

    // Print summary
    println!("\n=== Test Summary ===");
    println!("Passed: {}", passed);
    println!("Failed: {}", failed);
    println!("Total:  {}", passed + failed);

    if failed == 0 {
        println!("\n✓ All tests passed!");
        Ok(())
    } else {
        println!("\n✗ Some tests failed");
        Err("Test failures detected".into())
    }
}

/// Run a single test and track results.
fn run_test<F>(name: &str, test_fn: F, passed: &mut u32, failed: &mut u32)
where
    F: FnOnce() -> Result<(), Box<dyn std::error::Error>>,
{
    print!("  [TEST] {} ... ", name);
    match test_fn() {
        Ok(_) => {
            println!("✓ PASS");
            *passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: {}", e);
            *failed += 1;
        }
    }
}

/// Test basic send and receive functionality.
fn test_basic_send_receive() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send a message
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
    backend.send_message(&msg)?;

    // Verify it was recorded
    assert_eq!(
        backend.get_recorded_messages().len(),
        1,
        "Expected 1 message"
    );
    assert!(
        backend.verify_message_sent(CanId::Standard(0x123)),
        "Message not found"
    );

    backend.close()?;
    Ok(())
}

/// Test error handling with error injection.
fn test_error_handling() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Inject send error
    backend
        .error_injector_mut()
        .inject_send_error(CanError::SendFailed {
            reason: "Test error".to_string(),
        });

    // Send should fail
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
    let result = backend.send_message(&msg);
    assert!(result.is_err(), "Expected send to fail");

    // Verify message was not recorded
    assert_eq!(
        backend.get_recorded_messages().len(),
        0,
        "Failed message should not be recorded"
    );

    backend.close()?;
    Ok(())
}

/// Test message filtering by ID.
fn test_message_filtering() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send messages with different IDs
    backend.send_message(&CanMessage::new_standard(0x100, &[1])?)?;
    backend.send_message(&CanMessage::new_standard(0x200, &[2])?)?;
    backend.send_message(&CanMessage::new_standard(0x100, &[3])?)?;
    backend.send_message(&CanMessage::new_standard(0x300, &[4])?)?;

    // Filter by ID
    let messages_100 = backend.get_messages_by_id(CanId::Standard(0x100));
    assert_eq!(messages_100.len(), 2, "Expected 2 messages with ID 0x100");
    assert_eq!(messages_100[0].data()[0], 1, "Wrong data in first message");
    assert_eq!(messages_100[1].data()[0], 3, "Wrong data in second message");

    backend.close()?;
    Ok(())
}

/// Test CAN-FD support.
fn test_canfd_support() -> Result<(), Box<dyn std::error::Error>> {
    // Test with CAN-FD enabled
    let mut backend_fd = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend_fd.initialize(&config)?;
    backend_fd.open_channel(0)?;

    let capability = backend_fd.get_capability()?;
    assert!(capability.supports_canfd, "CAN-FD should be supported");

    // Send CAN-FD message
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let msg = CanMessage::new_fd(CanId::Standard(0x123), &data)?;
    backend_fd.send_message(&msg)?;

    let recorded = backend_fd.get_recorded_messages();
    assert_eq!(recorded.len(), 1, "Expected 1 message");
    assert!(recorded[0].is_fd(), "Message should be CAN-FD");

    backend_fd.close()?;

    // Test with CAN-FD disabled
    let config_20 = MockConfig::can20_only();
    let mut backend_20 = MockBackend::with_config(config_20);
    backend_20.initialize(&config)?;
    backend_20.open_channel(0)?;

    let capability = backend_20.get_capability()?;
    assert!(!capability.supports_canfd, "CAN-FD should not be supported");

    // CAN-FD message should fail
    let result = backend_20.send_message(&msg);
    assert!(
        result.is_err(),
        "CAN-FD message should fail on CAN 2.0 backend"
    );

    backend_20.close()?;
    Ok(())
}

/// Test extended ID support.
fn test_extended_ids() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send messages with extended IDs
    backend.send_message(&CanMessage::new_extended(0x12345678, &[1, 2])?)?;
    backend.send_message(&CanMessage::new_extended(0x1FFFFFFF, &[3, 4])?)?;

    // Verify extended IDs
    assert!(
        backend.verify_message_sent(CanId::Extended(0x12345678)),
        "Extended ID 0x12345678 not found"
    );
    assert!(
        backend.verify_message_sent(CanId::Extended(0x1FFFFFFF)),
        "Extended ID 0x1FFFFFFF not found"
    );

    // Standard and extended IDs with same numeric value are different
    assert!(
        !backend.verify_message_sent(CanId::Standard(0x5678)),
        "Standard ID should not match extended ID"
    );

    backend.close()?;
    Ok(())
}

/// Test remote frame support.
fn test_remote_frames() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send remote frame
    let remote = CanMessage::new_remote(CanId::Standard(0x123), 4)?;
    backend.send_message(&remote)?;

    // Verify remote frame
    let messages = backend.get_messages_by_id(CanId::Standard(0x123));
    assert_eq!(messages.len(), 1, "Expected 1 message");
    assert!(messages[0].is_remote(), "Message should be remote frame");

    backend.close()?;
    Ok(())
}

/// Test channel management.
fn test_channel_management() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;

    // Open multiple channels
    backend.open_channel(0)?;
    backend.open_channel(1)?;

    // Cannot open same channel twice
    let result = backend.open_channel(0);
    assert!(result.is_err(), "Should not be able to open channel twice");

    // Close channel
    backend.close_channel(0)?;

    // Cannot close already closed channel
    let result = backend.close_channel(0);
    assert!(
        result.is_err(),
        "Should not be able to close closed channel"
    );

    // Invalid channel
    let result = backend.open_channel(99);
    assert!(
        result.is_err(),
        "Should not be able to open invalid channel"
    );

    backend.close()?;
    Ok(())
}

/// Test state transitions.
fn test_state_transitions() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    // Cannot send before initialization
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
    let result = backend.send_message(&msg);
    assert!(result.is_err(), "Should not be able to send before init");

    // Initialize
    backend.initialize(&config)?;

    // Cannot send without open channel
    let result = backend.send_message(&msg);
    assert!(
        result.is_err(),
        "Should not be able to send without channel"
    );

    // Open channel and send
    backend.open_channel(0)?;
    backend.send_message(&msg)?;

    // Close and verify cannot send
    backend.close()?;
    let result = backend.send_message(&msg);
    assert!(result.is_err(), "Should not be able to send after close");

    Ok(())
}

/// Test burst traffic handling.
fn test_burst_traffic() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Send burst of messages
    let burst_size = 100;
    for i in 0..burst_size {
        let msg = CanMessage::new_standard((i % 10) as u16, &[i as u8])?;
        backend.send_message(&msg)?;
    }

    // Verify all messages recorded
    assert!(
        backend.verify_message_count(burst_size),
        "Not all messages recorded"
    );

    // Verify distribution across IDs
    for id in 0..10 {
        let messages = backend.get_messages_by_id(CanId::Standard(id));
        assert_eq!(messages.len(), 10, "Expected 10 messages for ID {}", id);
    }

    backend.close()?;
    Ok(())
}

/// Test error recovery scenarios.
fn test_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // Configure intermittent failures
    backend.error_injector_mut().inject_send_error_with_config(
        CanError::SendFailed {
            reason: "Intermittent".to_string(),
        },
        2, // fail twice
        0,
    );

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;

    // First two sends should fail
    assert!(
        backend.send_message(&msg).is_err(),
        "First send should fail"
    );
    assert!(
        backend.send_message(&msg).is_err(),
        "Second send should fail"
    );

    // Third send should succeed
    assert!(
        backend.send_message(&msg).is_ok(),
        "Third send should succeed"
    );

    // Verify only successful message recorded
    assert!(
        backend.verify_message_count(1),
        "Only successful message should be recorded"
    );

    backend.close()?;
    Ok(())
}

/// Helper macro for assertions with custom messages.
#[allow(unused_macros)]
macro_rules! assert_msg {
    ($cond:expr, $msg:expr) => {
        if !$cond {
            return Err($msg.into());
        }
    };
}
