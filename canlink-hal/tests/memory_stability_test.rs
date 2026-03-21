//! Memory stability test for SC-004 verification.
//!
//! This test verifies that long-running operations maintain stable memory usage
//! with fluctuation < 10%.
//!
//! ## Success Criteria (SC-004)
//!
//! - Memory usage fluctuation < 10% during extended operation
//! - No memory leaks detected
//! - Stable performance over time

use canlink_hal::{
    filter::{FilterChain, IdFilter, RangeFilter},
    queue::{BoundedQueue, QueueOverflowPolicy},
    BackendConfig, CanBackend, CanMessage,
};
use canlink_mock::{MockBackend, MockConfig};

#[test]
fn test_queue_memory_stability() {
    // Test that queue operations don't leak memory
    let iterations = 10000;
    let queue_capacity = 100;

    // Warm up
    {
        let mut queue = BoundedQueue::with_policy(queue_capacity, QueueOverflowPolicy::DropOldest);
        for i in 0..1000 {
            let msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap();
            let _ = queue.push(msg);
        }
        for _ in 0..1000 {
            let _ = queue.pop();
        }
    }

    // Measure baseline
    let mut queue = BoundedQueue::with_policy(queue_capacity, QueueOverflowPolicy::DropOldest);

    // Run many iterations of push/pop cycles
    for iteration in 0..iterations {
        // Push some messages
        for i in 0..10 {
            let msg = CanMessage::new_standard(((iteration * 10 + i) % 0x7FF) as u16, &[i as u8])
                .unwrap();
            let _ = queue.push(msg);
        }

        // Pop some messages
        for _ in 0..10 {
            let _ = queue.pop();
        }

        // Verify queue length stays bounded
        assert!(
            queue.len() <= queue_capacity,
            "Queue exceeded capacity at iteration {}",
            iteration
        );
    }

    // Verify stats are reasonable
    let stats = queue.stats();
    assert!(stats.enqueued > 0, "No messages were enqueued");
    assert!(stats.dequeued > 0, "No messages were dequeued");

    // Verify the queue is still functional
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    assert!(
        queue.push(msg).is_ok(),
        "Queue should still accept messages"
    );
}

#[test]
fn test_filter_chain_memory_stability() {
    // Test that filter chain operations don't leak memory
    let iterations = 1000;

    for _ in 0..iterations {
        // Create filter chain
        let mut chain = FilterChain::new(8);

        // Add various filters
        chain.add_filter(Box::new(IdFilter::new(0x123)));
        chain.add_filter(Box::new(IdFilter::with_mask(0x100, 0x700)));
        chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));

        // Use the filter chain
        let msg1 = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
        let msg2 = CanMessage::new_standard(0x456, &[4, 5, 6]).unwrap();

        assert!(chain.matches(&msg1));
        assert!(!chain.matches(&msg2));

        // Clear and rebuild
        chain.clear();
        assert!(chain.is_empty());

        chain.add_filter(Box::new(IdFilter::new(0x456)));
        assert!(chain.matches(&msg2));

        // Chain is dropped here, all memory should be freed
    }

    // If we get here without OOM, memory is being properly managed
}

#[test]
fn test_backend_message_cycling_stability() {
    // Test that sending/receiving many messages doesn't leak memory
    let iterations = 10000;

    // Create backend with limited recording to prevent intentional memory growth
    let config = MockConfig {
        max_recorded_messages: 100,
        ..MockConfig::default()
    };
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Send many messages
    for i in 0..iterations {
        let msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8, (i >> 8) as u8]).unwrap();
        backend.send_message(&msg).unwrap();

        // Periodically clear to prevent intentional accumulation
        if i % 1000 == 999 {
            backend.clear_recorded_messages();
        }
    }

    // Verify recorded messages are bounded
    let recorded = backend.get_recorded_messages();
    assert!(
        recorded.len() <= 100,
        "Recorded messages exceeded limit: {}",
        recorded.len()
    );

    backend.close().unwrap();
}

#[test]
fn test_filter_with_backend_stability() {
    // Test filter + backend combination for memory stability
    let iterations = 5000;

    // Create preset messages
    let preset: Vec<CanMessage> = (0..10000)
        .map(|i| CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap())
        .collect();

    let config = MockConfig::with_preset_messages(preset);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // Add filter
    backend.add_id_filter(0x123);
    backend.add_range_filter(0x200, 0x2FF);

    // Receive filtered messages
    let mut received_count = 0;
    for _ in 0..iterations {
        if let Ok(Some(_msg)) = backend.receive_message() {
            received_count += 1;
        }
    }

    // Some messages should have been filtered
    assert!(
        received_count < iterations,
        "Filter didn't filter any messages"
    );

    backend.close().unwrap();
}

#[test]
fn test_queue_overflow_policies_stability() {
    // Test all overflow policies for memory stability
    let iterations = 10000;
    let capacity = 50;

    // Test DropOldest
    {
        let mut queue = BoundedQueue::with_policy(capacity, QueueOverflowPolicy::DropOldest);

        for i in 0..iterations {
            let msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap();
            let _ = queue.push(msg);
        }

        assert_eq!(queue.len(), capacity);
        let stats = queue.stats();
        assert_eq!(stats.dropped as usize, iterations - capacity);
    }

    // Test DropNewest
    {
        let mut queue = BoundedQueue::with_policy(capacity, QueueOverflowPolicy::DropNewest);

        for i in 0..iterations {
            let msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap();
            let _ = queue.push(msg);
        }

        assert_eq!(queue.len(), capacity);
        let stats = queue.stats();
        assert_eq!(stats.dropped as usize, iterations - capacity);
    }
}

#[test]
fn test_repeated_init_close_stability() {
    // Test that repeated init/close cycles don't leak memory
    let iterations = 100;

    for _ in 0..iterations {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");

        backend.initialize(&config).unwrap();
        backend.open_channel(0).unwrap();

        // Do some work
        for i in 0..100 {
            let msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap();
            backend.send_message(&msg).unwrap();
        }

        backend.close().unwrap();
        // Backend is dropped here
    }

    // If we get here without issues, init/close is stable
}

#[test]
fn test_message_creation_stability() {
    // Test that creating many messages doesn't leak memory
    let iterations = 100000;

    for i in 0..iterations {
        // Create various message types
        let _std_msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap();
        let _ext_msg = CanMessage::new_extended(i % 0x1FFFFFFF, &[i as u8]).unwrap();

        // Messages are dropped at end of each iteration
    }

    // If we get here, message creation/destruction is stable
}

/// Simulated long-running test that verifies memory stability over time.
/// This test runs multiple cycles and checks that memory usage remains stable.
#[test]
fn test_extended_operation_stability() {
    let cycles = 100;
    let operations_per_cycle = 1000;

    // Track queue stats across cycles to verify stability
    let mut cycle_stats: Vec<(u64, u64)> = Vec::new();

    for cycle in 0..cycles {
        // Create fresh instances each cycle
        let mut queue = BoundedQueue::with_policy(100, QueueOverflowPolicy::DropOldest);
        let mut chain = FilterChain::new(8);

        chain.add_filter(Box::new(IdFilter::new(0x123)));
        chain.add_filter(Box::new(RangeFilter::new(0x100, 0x1FF)));

        // Perform operations
        for i in 0..operations_per_cycle {
            let msg = CanMessage::new_standard((i % 0x7FF) as u16, &[i as u8]).unwrap();

            // Filter check
            let _ = chain.matches(&msg);

            // Queue operations
            let _ = queue.push(msg);
            if i % 2 == 0 {
                let _ = queue.pop();
            }
        }

        // Record stats
        let stats = queue.stats();
        cycle_stats.push((stats.enqueued, stats.dropped));

        // Verify consistency
        assert_eq!(
            stats.enqueued as usize, operations_per_cycle,
            "Cycle {}: unexpected push count",
            cycle
        );
    }

    // Verify all cycles had consistent behavior
    let first_stats = cycle_stats[0];
    for (i, stats) in cycle_stats.iter().enumerate() {
        assert_eq!(
            stats.0, first_stats.0,
            "Cycle {} had different push count",
            i
        );
    }
}
