//! System robustness integration tests (T050)
//!
//! Tests for hardware disconnect detection, queue overflow handling,
//! and long-running stability.

use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;

#[test]
fn test_hardware_disconnect_detection() {
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();

    // Backend should be operational
    let msg = CanMessage::new_standard(0x100, &[1, 2, 3]).unwrap();
    assert!(backend.send_message(&msg).is_ok());

    // Simulate hardware disconnect
    backend.simulate_disconnect();
    assert!(backend.is_disconnected());

    // Operations should fail
    let msg = CanMessage::new_standard(0x200, &[4, 5, 6]).unwrap();
    let result = backend.send_message(&msg);
    assert!(result.is_err());

    let result = backend.receive_message();
    assert!(result.is_err());

    // Simulate reconnection
    backend.simulate_reconnect();
    assert!(!backend.is_disconnected());

    // Operations should work again
    let msg = CanMessage::new_standard(0x300, &[7, 8, 9]).unwrap();
    assert!(backend.send_message(&msg).is_ok());
}

#[test]
fn test_disconnect_reconnect_cycle() {
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();

    // Multiple disconnect/reconnect cycles
    for i in 0..10 {
        // Send a message
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        assert!(backend.send_message(&msg).is_ok());

        // Disconnect
        backend.simulate_disconnect();
        assert!(backend.is_disconnected());

        // Reconnect
        backend.simulate_reconnect();
        assert!(!backend.is_disconnected());
    }

    // Verify all messages were recorded
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 10);
}

#[test]
fn test_queue_overflow_drop_oldest() {
    use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};

    let mut queue = BoundedQueue::with_policy(5, QueueOverflowPolicy::DropOldest);

    // Fill beyond capacity
    for i in 0..10u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    // Should only have last 5 messages
    assert_eq!(queue.len(), 5);

    // Verify oldest were dropped
    let first = queue.pop().unwrap();
    assert_eq!(first.id().raw(), 0x105); // 6th message (0-4 dropped)

    let stats = queue.stats();
    assert_eq!(stats.dropped, 5);
    assert_eq!(stats.overflow_count, 5);
}

#[test]
fn test_queue_overflow_drop_newest() {
    use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};

    let mut queue = BoundedQueue::with_policy(5, QueueOverflowPolicy::DropNewest);

    // Fill beyond capacity
    for i in 0..10u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        let _ = queue.push(msg); // May return error for dropped messages
    }

    // Should only have first 5 messages
    assert_eq!(queue.len(), 5);

    // Verify newest were dropped (first 5 preserved)
    let first = queue.pop().unwrap();
    assert_eq!(first.id().raw(), 0x100); // First message preserved

    let stats = queue.stats();
    assert_eq!(stats.dropped, 5);
}

#[test]
fn test_repeated_open_close_no_leak() {
    // This test verifies that repeated open/close cycles don't leak resources
    // (actual leak detection requires valgrind/miri, but we can verify behavior)

    for _ in 0..100 {
        let mut backend = MockBackend::new();
        backend.initialize(&BackendConfig::new("mock")).unwrap();
        backend.open_channel(0).unwrap();

        // Send some messages
        for i in 0..10u16 {
            let msg = CanMessage::new_standard(i, &[i as u8]).unwrap();
            backend.send_message(&msg).unwrap();
        }

        backend.close_channel(0).unwrap();
        backend.close().unwrap();
    }
    // If we get here without OOM, basic resource management is working
}

#[test]
fn test_filter_with_disconnect() {
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();

    // Add filters
    backend.add_id_filter(0x100);
    backend.add_id_filter(0x200);
    assert_eq!(backend.filter_count(), 2);

    // Disconnect
    backend.simulate_disconnect();

    // Reconnect
    backend.simulate_reconnect();

    // Filters should still be configured
    assert_eq!(backend.filter_count(), 2);
}

#[test]
fn test_backend_state_after_error() {
    use canlink_hal::BackendState;

    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();

    assert_eq!(backend.get_state(), BackendState::Ready);

    // Simulate disconnect (puts backend in Error state)
    backend.simulate_disconnect();
    assert_eq!(backend.get_state(), BackendState::Error);

    // Reconnect restores Ready state
    backend.simulate_reconnect();
    assert_eq!(backend.get_state(), BackendState::Ready);
}

#[test]
fn test_message_rate_monitor() {
    use canlink_hal::MessageRateMonitor;

    let mut monitor = MessageRateMonitor::new(100); // 100 msg/s threshold

    // Record some messages (won't exceed threshold in < 1 second)
    for _ in 0..50 {
        let exceeded = monitor.record_message();
        assert!(!exceeded); // Won't exceed until 1 second passes
    }

    assert_eq!(monitor.current_count(), 50);
    assert_eq!(monitor.threshold(), 100);
}

#[test]
fn test_switch_backend() {
    use canlink_hal::switch_backend;

    let mut old_backend = MockBackend::new();
    old_backend.initialize(&BackendConfig::new("mock")).unwrap();
    old_backend.open_channel(0).unwrap();

    // Send some messages on old backend
    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        old_backend.send_message(&msg).unwrap();
    }

    let mut new_backend = MockBackend::new();

    // Switch backends
    let result = switch_backend(
        &mut old_backend,
        &mut new_backend,
        &BackendConfig::new("mock"),
    );
    assert!(result.is_ok());

    // New backend should be initialized
    new_backend.open_channel(0).unwrap();
    let msg = CanMessage::new_standard(0x200, &[1]).unwrap();
    assert!(new_backend.send_message(&msg).is_ok());
}

#[test]
fn test_long_running_simulation() {
    // Simulate a long-running scenario with many operations
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock")).unwrap();
    backend.open_channel(0).unwrap();

    // Simulate 1000 message cycles
    for cycle in 0..1000u32 {
        let msg = CanMessage::new_extended(cycle, &[(cycle % 256) as u8]).unwrap();
        backend.send_message(&msg).unwrap();

        // Occasionally simulate disconnect/reconnect
        if cycle % 100 == 99 {
            backend.simulate_disconnect();
            backend.simulate_reconnect();
        }
    }

    // Verify all messages were recorded
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1000);

    backend.close().unwrap();
}
