//! BoundedQueue unit tests (T048)
//!
//! Tests for the bounded queue implementation.

use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
use canlink_hal::CanMessage;
use std::time::Duration;

#[test]
fn test_new_queue_default_capacity() {
    let queue = BoundedQueue::new(100);
    assert_eq!(queue.capacity(), 100);
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
    assert!(!queue.is_full());
    assert_eq!(queue.policy(), QueueOverflowPolicy::DropOldest);
}

#[test]
fn test_with_policy() {
    let queue = BoundedQueue::with_policy(50, QueueOverflowPolicy::DropNewest);
    assert_eq!(queue.capacity(), 50);
    assert_eq!(queue.policy(), QueueOverflowPolicy::DropNewest);
}

#[test]
fn test_basic_push_pop() {
    let mut queue = BoundedQueue::new(10);

    let msg1 = CanMessage::new_standard(0x100, &[1, 2, 3]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[4, 5, 6]).unwrap();

    assert!(queue.push(msg1.clone()).is_ok());
    assert_eq!(queue.len(), 1);
    assert!(!queue.is_empty());

    assert!(queue.push(msg2.clone()).is_ok());
    assert_eq!(queue.len(), 2);

    // Pop in FIFO order
    let popped1 = queue.pop().unwrap();
    assert_eq!(popped1.data(), msg1.data());
    assert_eq!(queue.len(), 1);

    let popped2 = queue.pop().unwrap();
    assert_eq!(popped2.data(), msg2.data());
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());

    // Pop from empty queue
    assert!(queue.pop().is_none());
}

#[test]
fn test_peek() {
    let mut queue = BoundedQueue::new(10);

    // Peek empty queue
    assert!(queue.peek().is_none());

    let msg = CanMessage::new_standard(0x100, &[1, 2, 3]).unwrap();
    queue.push(msg.clone()).unwrap();

    // Peek should return reference without removing
    let peeked = queue.peek().unwrap();
    assert_eq!(peeked.data(), msg.data());
    assert_eq!(queue.len(), 1); // Still in queue

    // Peek again should return same message
    let peeked2 = queue.peek().unwrap();
    assert_eq!(peeked2.data(), msg.data());
    assert_eq!(queue.len(), 1);
}

#[test]
fn test_capacity_limit() {
    let mut queue = BoundedQueue::new(3);

    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let msg3 = CanMessage::new_standard(0x300, &[3]).unwrap();

    queue.push(msg1).unwrap();
    queue.push(msg2).unwrap();
    queue.push(msg3).unwrap();

    assert_eq!(queue.len(), 3);
    assert!(queue.is_full());
}

#[test]
fn test_clear() {
    let mut queue = BoundedQueue::new(10);

    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    assert_eq!(queue.len(), 5);

    queue.clear();

    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
    assert!(queue.pop().is_none());
}

#[test]
fn test_statistics_enqueue_dequeue() {
    let mut queue = BoundedQueue::new(10);

    // Push 5 messages
    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    let stats = queue.stats();
    assert_eq!(stats.enqueued, 5);
    assert_eq!(stats.dequeued, 0);
    assert_eq!(stats.dropped, 0);
    assert_eq!(stats.overflow_count, 0);

    // Pop 3 messages
    for _ in 0..3 {
        queue.pop();
    }

    let stats = queue.stats();
    assert_eq!(stats.enqueued, 5);
    assert_eq!(stats.dequeued, 3);
}

#[test]
fn test_statistics_overflow() {
    let mut queue = BoundedQueue::with_policy(2, QueueOverflowPolicy::DropOldest);

    // Push 5 messages into capacity-2 queue
    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    let stats = queue.stats();
    assert_eq!(stats.enqueued, 5);
    assert_eq!(stats.dropped, 3); // 3 oldest dropped
    assert_eq!(stats.overflow_count, 3);
}

#[test]
fn test_adjust_capacity_increase() {
    let mut queue = BoundedQueue::new(3);

    // Fill the queue
    for i in 0..3u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    assert!(queue.is_full());

    // Increase capacity
    queue.adjust_capacity(5);
    assert_eq!(queue.capacity(), 5);
    assert!(!queue.is_full());
    assert_eq!(queue.len(), 3); // Messages preserved

    // Can now add more
    let msg = CanMessage::new_standard(0x400, &[4]).unwrap();
    queue.push(msg).unwrap();
    assert_eq!(queue.len(), 4);
}

#[test]
fn test_adjust_capacity_decrease() {
    let mut queue = BoundedQueue::new(5);

    // Add 5 messages
    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    // Decrease capacity to 3 - should drop oldest 2
    queue.adjust_capacity(3);
    assert_eq!(queue.capacity(), 3);
    assert_eq!(queue.len(), 3);
    assert!(queue.is_full());

    // Verify oldest were dropped
    let msg = queue.pop().unwrap();
    assert_eq!(msg.data()[0], 2); // Third message (0, 1 were dropped)
}

#[test]
fn test_adjust_capacity_to_zero() {
    let mut queue = BoundedQueue::new(5);

    for i in 0..3u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    // Adjust to 0 should clear the queue
    queue.adjust_capacity(0);
    assert_eq!(queue.capacity(), 0);
    assert_eq!(queue.len(), 0);
    assert!(queue.is_empty());
}

#[test]
fn test_iter() {
    let mut queue = BoundedQueue::new(10);

    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    // Iterate without consuming
    let ids: Vec<u32> = queue.iter().map(|m| m.id().raw()).collect();
    assert_eq!(ids, vec![0x100, 0x101, 0x102, 0x103, 0x104]);

    // Queue should still have all messages
    assert_eq!(queue.len(), 5);
}

#[test]
fn test_fifo_order() {
    let mut queue = BoundedQueue::new(100);

    // Push messages in order
    for i in 0..50u16 {
        let msg = CanMessage::new_standard(i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    // Pop should return in same order
    for i in 0..50u32 {
        let msg = queue.pop().unwrap();
        assert_eq!(msg.id().raw(), i);
        assert_eq!(msg.data()[0], i as u8);
    }
}

#[test]
fn test_block_policy_with_timeout() {
    let mut queue = BoundedQueue::with_policy(
        1,
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(50),
        },
    );

    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    queue.push(msg1).unwrap();

    // Second push should fail (sync version can't block)
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    let result = queue.push(msg2);
    assert!(result.is_err());
}

#[test]
fn test_statistics_preserved_after_clear() {
    let mut queue = BoundedQueue::new(10);

    for i in 0..5u16 {
        let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    let stats_before = queue.stats();
    assert_eq!(stats_before.enqueued, 5);

    queue.clear();

    // Stats should be preserved after clear (they track lifetime stats)
    let stats_after = queue.stats();
    assert_eq!(stats_after.enqueued, 5);
}

#[test]
fn test_single_capacity_queue() {
    let mut queue = BoundedQueue::new(1);

    let msg1 = CanMessage::new_standard(0x100, &[1]).unwrap();
    queue.push(msg1).unwrap();

    assert!(queue.is_full());
    assert_eq!(queue.len(), 1);

    // With DropOldest, pushing replaces the message
    let msg2 = CanMessage::new_standard(0x200, &[2]).unwrap();
    queue.push(msg2).unwrap();

    assert_eq!(queue.len(), 1);
    let popped = queue.pop().unwrap();
    assert_eq!(popped.data()[0], 2);
}

#[test]
fn test_large_queue() {
    let mut queue = BoundedQueue::new(10000);

    // Fill with many messages (use extended IDs to allow > 2047)
    for i in 0..10000u32 {
        let msg = CanMessage::new_extended(i, &[(i % 256) as u8]).unwrap();
        queue.push(msg).unwrap();
    }

    assert_eq!(queue.len(), 10000);
    assert!(queue.is_full());

    // Verify first and last
    let first = queue.pop().unwrap();
    assert_eq!(first.id().raw(), 0);

    // Skip to near end (pop 9998 more to leave just the last one)
    for _ in 0..9998 {
        queue.pop();
    }

    let last = queue.pop().unwrap();
    assert_eq!(last.id().raw(), 9999);
    assert!(queue.is_empty());
}
