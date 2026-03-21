//! Integration tests for periodic message sending.
//!
//! Tests cover:
//! - T010: PeriodicMessage creation and validation
//! - T011: PeriodicStats statistics tracking
//! - T012: PeriodicScheduler basic functionality
//! - T013: Multiple concurrent messages (SC-002)
//! - T013a: Dynamic interval update (Scenario 1.2a)
//! - T013b: Send failure skip (Scenario 1.5)
//! - T013c: Backend disconnect handling (Scenario 1.6)

mod common;
use canlink_hal::periodic::{run_scheduler, PeriodicMessage, PeriodicScheduler, PeriodicStats};
use canlink_hal::{CanId, CanMessage};
use common::{create_initialized_backend, run_local};
use std::time::{Duration, Instant};

// ============================================================================
// T010: PeriodicMessage creation and validation tests
// ============================================================================

#[test]
fn test_periodic_message_creation() {
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04]).unwrap();
    let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

    assert_eq!(periodic.interval(), Duration::from_millis(100));
    assert!(periodic.is_enabled());
}

#[test]
fn test_periodic_message_interval_validation_min() {
    let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();

    // 0ms should fail
    let result = PeriodicMessage::new(msg.clone(), Duration::from_millis(0));
    assert!(result.is_err());

    // 1ms should succeed (minimum)
    let result = PeriodicMessage::new(msg, Duration::from_millis(1));
    assert!(result.is_ok());
}

#[test]
fn test_periodic_message_interval_validation_max() {
    let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();

    // 10001ms should fail
    let result = PeriodicMessage::new(msg.clone(), Duration::from_millis(10_001));
    assert!(result.is_err());

    // 10000ms should succeed (maximum)
    let result = PeriodicMessage::new(msg, Duration::from_millis(10_000));
    assert!(result.is_ok());
}

#[test]
fn test_periodic_message_update_data() {
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02]).unwrap();
    let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

    assert_eq!(periodic.message().data(), &[0x01, 0x02]);

    periodic.update_data(vec![0xAA, 0xBB, 0xCC]).unwrap();
    assert_eq!(periodic.message().data(), &[0xAA, 0xBB, 0xCC]);
}

#[test]
fn test_periodic_message_update_data_standard_rejects_long_payload() {
    let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
    let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

    let result = periodic.update_data(vec![0xAA; 9]);
    assert!(result.is_err());
}

#[test]
fn test_periodic_message_update_data_fd_accepts_long_payload() {
    let msg = CanMessage::new_fd(CanId::Standard(0x123), &[0x01; 12]).unwrap();
    let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

    periodic.update_data(vec![0xBB; 16]).unwrap();
    assert_eq!(periodic.message().data().len(), 16);
    assert!(periodic.message().is_fd());
}

#[test]
fn test_periodic_message_set_interval() {
    let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
    let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

    assert_eq!(periodic.interval(), Duration::from_millis(100));

    periodic.set_interval(Duration::from_millis(200)).unwrap();
    assert_eq!(periodic.interval(), Duration::from_millis(200));

    // Invalid interval should fail
    let result = periodic.set_interval(Duration::from_millis(0));
    assert!(result.is_err());
}

#[test]
fn test_periodic_message_enable_disable() {
    let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
    let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

    assert!(periodic.is_enabled());

    periodic.set_enabled(false);
    assert!(!periodic.is_enabled());

    periodic.set_enabled(true);
    assert!(periodic.is_enabled());
}

// ============================================================================
// T011: PeriodicStats statistics tests
// ============================================================================

#[test]
fn test_periodic_stats_new() {
    let stats = PeriodicStats::new();
    assert_eq!(stats.send_count(), 0);
    assert!(stats.average_interval().is_none());
    assert!(stats.min_interval().is_none());
    assert!(stats.max_interval().is_none());
}

#[test]
fn test_periodic_stats_record_send() {
    let mut stats = PeriodicStats::new();

    let t1 = Instant::now();
    stats.record_send(t1);
    assert_eq!(stats.send_count(), 1);

    // First send doesn't have interval data
    assert!(stats.average_interval().is_none());

    // Simulate second send after 100ms
    let t2 = t1 + Duration::from_millis(100);
    stats.record_send(t2);
    assert_eq!(stats.send_count(), 2);

    // Now we have interval data
    assert!(stats.average_interval().is_some());
    assert!(stats.min_interval().is_some());
    assert!(stats.max_interval().is_some());
}

#[test]
fn test_periodic_stats_min_max_interval() {
    let mut stats = PeriodicStats::new();

    let t1 = Instant::now();
    stats.record_send(t1);

    let t2 = t1 + Duration::from_millis(50);
    stats.record_send(t2);

    let t3 = t2 + Duration::from_millis(150);
    stats.record_send(t3);

    let t4 = t3 + Duration::from_millis(100);
    stats.record_send(t4);

    assert_eq!(stats.send_count(), 4);
    assert_eq!(stats.min_interval(), Some(Duration::from_millis(50)));
    assert_eq!(stats.max_interval(), Some(Duration::from_millis(150)));
}

#[test]
fn test_periodic_stats_reset() {
    let mut stats = PeriodicStats::new();

    let t1 = Instant::now();
    stats.record_send(t1);
    stats.record_send(t1 + Duration::from_millis(100));

    assert_eq!(stats.send_count(), 2);

    stats.reset();

    assert_eq!(stats.send_count(), 0);
    assert!(stats.average_interval().is_none());
}

// ============================================================================
// T012: PeriodicScheduler basic functionality tests
// ============================================================================

#[test]
fn test_scheduler_add_message() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        // Spawn scheduler in background
        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01, 0x02]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();
        assert!(id > 0);

        // Verify message is in the list
        let ids = scheduler.list_ids().await.unwrap();
        assert!(ids.contains(&id));

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

#[test]
fn test_scheduler_remove_message() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();

        // Remove the message
        scheduler.remove(id).await.unwrap();

        // Verify it's gone
        let ids = scheduler.list_ids().await.unwrap();
        assert!(!ids.contains(&id));

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

#[test]
fn test_scheduler_remove_nonexistent() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        // Try to remove non-existent message
        let result = scheduler.remove(999).await;
        assert!(result.is_err());

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

#[test]
fn test_scheduler_update_data() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01, 0x02]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();

        // Update data
        scheduler
            .update_data(id, vec![0xAA, 0xBB, 0xCC])
            .await
            .unwrap();

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

#[test]
fn test_scheduler_set_enabled() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();

        // Disable
        scheduler.set_enabled(id, false).await.unwrap();

        // Enable
        scheduler.set_enabled(id, true).await.unwrap();

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

#[test]
fn test_scheduler_get_stats() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(50)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();

        // Wait for some sends
        tokio::time::sleep(Duration::from_millis(150)).await;

        let stats = scheduler.get_stats(id).await.unwrap();
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert!(stats.send_count() >= 1);

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

// ============================================================================
// T013: Multiple concurrent messages test (SC-002 verification)
// ============================================================================

#[test]
fn test_scheduler_multiple_messages() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        // Add multiple messages with different intervals
        let mut ids = Vec::new();
        for i in 0..5 {
            let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
            let periodic =
                PeriodicMessage::new(msg, Duration::from_millis(50 + i as u64 * 10)).unwrap();
            let id = scheduler.add(periodic).await.unwrap();
            ids.push(id);
        }

        // Verify all messages are registered
        let registered_ids = scheduler.list_ids().await.unwrap();
        assert_eq!(registered_ids.len(), 5);

        for id in &ids {
            assert!(registered_ids.contains(id));
        }

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

#[test]
fn test_scheduler_capacity_32_messages() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        // Add 32 messages (should succeed)
        for i in 0..32 {
            let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
            let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();
            let result = scheduler.add(periodic).await;
            assert!(result.is_ok(), "Failed to add message {}", i);
        }

        // Verify all 32 are registered
        let ids = scheduler.list_ids().await.unwrap();
        assert_eq!(ids.len(), 32);

        // Adding 33rd should fail (capacity exceeded)
        let msg = CanMessage::new_standard(0x200, &[0xFF]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();
        let result = scheduler.add(periodic).await;
        assert!(result.is_err());

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

// ============================================================================
// T013a: Dynamic interval update test (Scenario 1.2a)
// ============================================================================

#[tokio::test(start_paused = true)]
async fn test_scheduler_update_interval_takes_effect_after_next_send() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let backend = create_initialized_backend();
            let (scheduler, command_rx) = PeriodicScheduler::new(64);
            tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

            let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
            let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();
            let id = scheduler.add(periodic).await.unwrap();

            tokio::time::advance(Duration::from_millis(100)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 1);

            scheduler
                .update_interval(id, Duration::from_millis(20))
                .await
                .unwrap();

            tokio::time::advance(Duration::from_millis(80)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 1);

            tokio::time::advance(Duration::from_millis(20)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 2);

            tokio::time::advance(Duration::from_millis(20)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 3);

            tokio::time::advance(Duration::from_millis(20)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 4);

            scheduler.shutdown().await.unwrap();
        })
        .await;
}

#[tokio::test(start_paused = true)]
async fn test_scheduler_enable_disable_reschedules() {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let backend = create_initialized_backend();
            let (scheduler, command_rx) = PeriodicScheduler::new(64);
            tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

            let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
            let periodic = PeriodicMessage::new(msg, Duration::from_millis(50)).unwrap();
            let id = scheduler.add(periodic).await.unwrap();

            scheduler.set_enabled(id, false).await.unwrap();

            tokio::time::advance(Duration::from_millis(100)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 0);

            scheduler.set_enabled(id, true).await.unwrap();

            tokio::time::advance(Duration::from_millis(50)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 1);

            tokio::time::advance(Duration::from_millis(50)).await;
            tokio::task::yield_now().await;
            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert_eq!(stats.send_count(), 2);

            scheduler.shutdown().await.unwrap();
        })
        .await;
}

#[test]
fn test_scheduler_update_interval() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();

        // Update interval
        scheduler
            .update_interval(id, Duration::from_millis(200))
            .await
            .unwrap();

        // Invalid interval should fail
        let result = scheduler
            .update_interval(id, Duration::from_millis(0))
            .await;
        assert!(result.is_err());

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

// ============================================================================
// T013b: Send failure skip test (Scenario 1.5)
// ============================================================================

#[test]
fn test_scheduler_continues_after_send_failure() {
    use canlink_hal::CanError;

    run_local(async {
        let mut backend = create_initialized_backend();

        // Inject a send error for the first send
        backend
            .error_injector_mut()
            .inject_send_error(CanError::SendFailed {
                reason: "Test failure".to_string(),
            });

        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(30)).unwrap();

        let id = scheduler.add(periodic).await.unwrap();

        // Wait for multiple send attempts (first fails, subsequent should succeed)
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Scheduler should still be running and have stats
        let stats = scheduler.get_stats(id).await.unwrap();
        assert!(stats.is_some());

        // Should have attempted multiple sends (some may have succeeded after the injected error)
        let stats = stats.unwrap();
        assert!(stats.send_count() >= 1);

        scheduler.shutdown().await.unwrap();
        let _ = handle.await;
    });
}

// ============================================================================
// T013c: Backend disconnect handling test (Scenario 1.6)
// ============================================================================

// Note: Full backend disconnect testing requires MockBackend enhancements
// to simulate disconnection. For now, we test that the scheduler handles
// shutdown gracefully.

#[test]
fn test_scheduler_graceful_shutdown() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(50)).unwrap();

        let _id = scheduler.add(periodic).await.unwrap();

        // Let it run briefly
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Shutdown should complete without error
        scheduler.shutdown().await.unwrap();

        // Task should complete
        let result = handle.await;
        assert!(result.is_ok());
    });
}

#[test]
fn test_scheduler_operations_after_shutdown() {
    run_local(async {
        let backend = create_initialized_backend();
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        let handle = tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        // Shutdown immediately
        scheduler.shutdown().await.unwrap();
        let _ = handle.await;

        // Operations after shutdown should fail
        let msg = CanMessage::new_standard(0x123, &[0x01]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        let result = scheduler.add(periodic).await;
        assert!(result.is_err());
    });
}

// ============================================================================
// Additional timing verification tests
// ============================================================================

#[cfg(test)]
mod timing_tests {
    use super::*;
    use tokio::task::LocalSet;

    #[test]
    fn test_scheduler_with_local_set() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let local = LocalSet::new();

        local.block_on(&rt, async {
            let backend = create_initialized_backend();
            let (scheduler, command_rx) = PeriodicScheduler::new(64);

            tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

            let msg = CanMessage::new_standard(0x123, &[0x01, 0x02]).unwrap();
            let periodic = PeriodicMessage::new(msg, Duration::from_millis(50)).unwrap();

            let id = scheduler.add(periodic).await.unwrap();
            assert!(id > 0);

            // Wait for some sends
            tokio::time::sleep(Duration::from_millis(200)).await;

            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert!(
                stats.send_count() >= 2,
                "Expected at least 2 sends, got {}",
                stats.send_count()
            );

            scheduler.shutdown().await.unwrap();
        });
    }

    #[test]
    fn test_scheduler_message_actually_sent() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let local = LocalSet::new();

        local.block_on(&rt, async {
            let backend = create_initialized_backend();
            let (scheduler, command_rx) = PeriodicScheduler::new(64);

            // We need to keep a reference to verify sent messages
            // Since backend is moved, we'll verify via stats
            tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

            let msg = CanMessage::new_standard(0x123, &[0xAA, 0xBB]).unwrap();
            let periodic = PeriodicMessage::new(msg, Duration::from_millis(30)).unwrap();

            let id = scheduler.add(periodic).await.unwrap();

            // Wait for sends
            tokio::time::sleep(Duration::from_millis(150)).await;

            let stats = scheduler.get_stats(id).await.unwrap().unwrap();
            assert!(stats.send_count() >= 3, "Expected at least 3 sends");

            scheduler.shutdown().await.unwrap();
        });
    }
}
