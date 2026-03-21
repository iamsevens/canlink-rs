//! QueueOverflowPolicy unit tests (T047)
//!
//! Tests for the queue overflow policy enum and its behavior.

use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
use canlink_hal::{CanMessage, QueueError};
use std::time::Duration;

#[test]
fn test_drop_oldest_policy() {
    // Create a queue with capacity 3 and DropOldest policy
    let mut queue = BoundedQueue::with_policy(3, QueueOverflowPolicy::DropOldest);

    // Fill the queue
    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let msg3 = CanMessage::new_standard(0x300, &[3]).unwrap();

    assert!(queue.push(msg1).is_ok());
    assert!(queue.push(msg2).is_ok());
    assert!(queue.push(msg3).is_ok());
    assert_eq!(queue.len(), 3);

    // Push another message - should drop oldest (0x100)
    let msg4 = CanMessage::new_standard(0x400, &[4]).unwrap();
    assert!(queue.push(msg4).is_ok());
    assert_eq!(queue.len(), 3);

    // Verify oldest was dropped
    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 2); // msg2 (0x200)

    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 3); // msg3 (0x300)

    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 4); // msg4 (0x400)

    assert!(queue.pop().is_none());
}

#[test]
fn test_drop_newest_policy() {
    // Create a queue with capacity 3 and DropNewest policy
    let mut queue = BoundedQueue::with_policy(3, QueueOverflowPolicy::DropNewest);

    // Fill the queue
    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let msg3 = CanMessage::new_standard(0x300, &[3]).unwrap();

    assert!(queue.push(msg1).is_ok());
    assert!(queue.push(msg2).is_ok());
    assert!(queue.push(msg3).is_ok());
    assert_eq!(queue.len(), 3);

    // Push another message - should return error (message dropped)
    let msg4 = CanMessage::new_standard(0x400, &[4]).unwrap();
    let result = queue.push(msg4);
    assert!(result.is_err());

    // Verify it's a MessageDropped error
    match result {
        Err(QueueError::MessageDropped { id, .. }) => {
            assert_eq!(id, 0x400);
        }
        _ => panic!("Expected MessageDropped error"),
    }

    assert_eq!(queue.len(), 3);

    // Verify original messages are preserved
    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 1); // msg1 (0x100)

    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 2); // msg2 (0x200)

    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 3); // msg3 (0x300)

    assert!(queue.pop().is_none());
}

#[test]
fn test_block_policy_immediate_space() {
    // Create a queue with capacity 3 and Block policy
    let mut queue = BoundedQueue::with_policy(
        3,
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100),
        },
    );

    // Queue has space, should succeed immediately
    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    assert!(queue.push(msg1).is_ok());
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_block_policy_returns_queue_full() {
    // Create a queue with capacity 1 and Block policy
    // Note: Synchronous push cannot actually block, it returns QueueFull immediately
    let mut queue = BoundedQueue::with_policy(
        1,
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100),
        },
    );

    // Fill the queue
    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    assert!(queue.push(msg1).is_ok());
    assert_eq!(queue.len(), 1);

    // Try to push another - should return QueueFull error immediately
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let result = queue.push(msg2);

    // Should return QueueFull error (sync version can't block)
    match result {
        Err(QueueError::QueueFull { capacity }) => {
            assert_eq!(capacity, 1);
        }
        _ => panic!("Expected QueueFull error"),
    }
}

#[test]
fn test_policy_default_is_drop_oldest() {
    let queue = BoundedQueue::new(10);
    assert_eq!(queue.policy(), QueueOverflowPolicy::DropOldest);
}

#[test]
fn test_policy_statistics_tracking() {
    let mut queue = BoundedQueue::with_policy(2, QueueOverflowPolicy::DropOldest);

    // Fill and overflow
    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let msg3 = CanMessage::new_standard(0x300, &[3]).unwrap();
    let msg4 = CanMessage::new_standard(0x400, &[4]).unwrap();

    queue.push(msg1).unwrap();
    queue.push(msg2).unwrap();
    queue.push(msg3).unwrap(); // Overflow - drops msg1
    queue.push(msg4).unwrap(); // Overflow - drops msg2

    let stats = queue.stats();
    assert_eq!(stats.enqueued, 4);
    assert_eq!(stats.dropped, 2);
    assert_eq!(stats.overflow_count, 2);
}

#[test]
fn test_drop_oldest_with_single_capacity() {
    let mut queue = BoundedQueue::with_policy(1, QueueOverflowPolicy::DropOldest);

    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();

    queue.push(msg1).unwrap();
    queue.push(msg2).unwrap();

    assert_eq!(queue.len(), 1);
    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 2); // Only msg2 remains
}

#[test]
fn test_drop_newest_with_single_capacity() {
    let mut queue = BoundedQueue::with_policy(1, QueueOverflowPolicy::DropNewest);

    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();

    queue.push(msg1).unwrap();

    // Second push should fail with MessageDropped
    let result = queue.push(msg2);
    assert!(result.is_err());

    assert_eq!(queue.len(), 1);
    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 1); // Only msg1 remains (msg2 was dropped)
}

#[test]
fn test_drop_newest_statistics() {
    let mut queue = BoundedQueue::with_policy(2, QueueOverflowPolicy::DropNewest);

    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let msg3 = CanMessage::new_standard(0x300, &[3]).unwrap();

    queue.push(msg1).unwrap();
    queue.push(msg2).unwrap();
    let _ = queue.push(msg3); // This will fail but still counts as dropped

    let stats = queue.stats();
    assert_eq!(stats.enqueued, 2); // Only 2 successfully enqueued
    assert_eq!(stats.dropped, 1);
    assert_eq!(stats.overflow_count, 1);
}

#[test]
fn test_policy_equality() {
    assert_eq!(
        QueueOverflowPolicy::DropOldest,
        QueueOverflowPolicy::DropOldest
    );
    assert_eq!(
        QueueOverflowPolicy::DropNewest,
        QueueOverflowPolicy::DropNewest
    );
    assert_eq!(
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100)
        },
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100)
        }
    );
    assert_ne!(
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100)
        },
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(200)
        }
    );
    assert_ne!(
        QueueOverflowPolicy::DropOldest,
        QueueOverflowPolicy::DropNewest
    );
}

#[test]
fn test_policy_helper_methods() {
    let drop_oldest = QueueOverflowPolicy::drop_oldest();
    assert_eq!(drop_oldest, QueueOverflowPolicy::DropOldest);
    assert!(!drop_oldest.may_block());
    assert!(drop_oldest.timeout().is_none());

    let drop_newest = QueueOverflowPolicy::drop_newest();
    assert_eq!(drop_newest, QueueOverflowPolicy::DropNewest);
    assert!(!drop_newest.may_block());
    assert!(drop_newest.timeout().is_none());

    let block = QueueOverflowPolicy::block(Duration::from_millis(500));
    assert!(block.may_block());
    assert_eq!(block.timeout(), Some(Duration::from_millis(500)));
}
