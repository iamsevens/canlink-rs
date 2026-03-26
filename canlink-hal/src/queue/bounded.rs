//! Bounded queue implementation (FR-011, FR-017)
//!
//! Provides a bounded message queue with configurable overflow policies.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::QueueError;
use crate::message::CanMessage;

use super::QueueOverflowPolicy;

/// Default queue capacity (FR-017)
pub const DEFAULT_QUEUE_CAPACITY: usize = 1000;

/// Queue statistics
///
/// Tracks queue operations for monitoring and debugging.
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// Total messages enqueued
    pub enqueued: u64,
    /// Total messages dequeued
    pub dequeued: u64,
    /// Total messages dropped due to overflow
    pub dropped: u64,
    /// Number of times the queue was full
    pub overflow_count: u64,
}

/// Internal atomic stats for thread-safe updates
struct AtomicQueueStats {
    enqueued: AtomicU64,
    dequeued: AtomicU64,
    dropped: AtomicU64,
    overflow_count: AtomicU64,
}

impl AtomicQueueStats {
    fn new() -> Self {
        Self {
            enqueued: AtomicU64::new(0),
            dequeued: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
            overflow_count: AtomicU64::new(0),
        }
    }

    fn snapshot(&self) -> QueueStats {
        QueueStats {
            enqueued: self.enqueued.load(Ordering::Relaxed),
            dequeued: self.dequeued.load(Ordering::Relaxed),
            dropped: self.dropped.load(Ordering::Relaxed),
            overflow_count: self.overflow_count.load(Ordering::Relaxed),
        }
    }

    fn inc_enqueued(&self) {
        self.enqueued.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_dequeued(&self) {
        self.dequeued.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_dropped(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }

    fn inc_overflow(&self) {
        self.overflow_count.fetch_add(1, Ordering::Relaxed);
    }
}

/// Bounded message queue
///
/// A queue with a fixed maximum capacity and configurable overflow policy.
///
/// # Example
///
/// ```rust
/// use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
/// use canlink_hal::message::CanMessage;
///
/// // Create queue with default policy (DropOldest)
/// let mut queue = BoundedQueue::new(100);
///
/// // Create queue with custom policy
/// let mut queue = BoundedQueue::with_policy(100, QueueOverflowPolicy::DropNewest);
/// ```
pub struct BoundedQueue {
    buffer: VecDeque<CanMessage>,
    capacity: usize,
    policy: QueueOverflowPolicy,
    stats: AtomicQueueStats,
}

impl BoundedQueue {
    /// Create a new bounded queue with default overflow policy
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of messages the queue can hold
    ///
    /// # Example
    ///
    /// ```rust
    /// use canlink_hal::queue::BoundedQueue;
    ///
    /// let queue = BoundedQueue::new(1000);
    /// assert_eq!(queue.capacity(), 1000);
    /// ```
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self::with_policy(capacity, QueueOverflowPolicy::default())
    }

    /// Create a new bounded queue with specified overflow policy
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of messages the queue can hold
    /// * `policy` - Overflow handling policy
    #[must_use]
    pub fn with_policy(capacity: usize, policy: QueueOverflowPolicy) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            policy,
            stats: AtomicQueueStats::new(),
        }
    }

    /// Get the queue capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current number of messages in the queue
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    /// Get the overflow policy
    pub fn policy(&self) -> QueueOverflowPolicy {
        self.policy
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        self.stats.snapshot()
    }

    /// Push a message to the queue
    ///
    /// Behavior depends on the overflow policy:
    /// - `DropOldest`: Removes the oldest message if full
    /// - `DropNewest`: Rejects the new message if full
    /// - `Block`: Returns `QueueError::QueueFull` (async version handles blocking)
    ///
    /// # Errors
    ///
    /// - Returns `QueueError::QueueFull` if using `Block` policy and queue is full
    /// - Returns `QueueError::MessageDropped` if using `DropNewest` policy and queue is full
    pub fn push(&mut self, message: CanMessage) -> Result<(), QueueError> {
        if self.is_full() {
            self.stats.inc_overflow();

            match self.policy {
                QueueOverflowPolicy::DropOldest => {
                    // Remove oldest message
                    #[allow(unused_variables)]
                    if let Some(dropped) = self.buffer.pop_front() {
                        self.stats.inc_dropped();
                        #[cfg(feature = "tracing")]
                        crate::log_queue_overflow!(self.policy, dropped.id().raw());
                    }
                    // Continue to add new message
                }
                QueueOverflowPolicy::DropNewest => {
                    self.stats.inc_dropped();
                    #[cfg(feature = "tracing")]
                    crate::log_queue_overflow!(self.policy, message.id().raw());
                    return Err(QueueError::MessageDropped {
                        id: message.id().raw(),
                        reason: "Queue full, DropNewest policy".to_string(),
                    });
                }
                QueueOverflowPolicy::Block { .. } => {
                    // Synchronous push cannot block, return error
                    return Err(QueueError::QueueFull {
                        capacity: self.capacity,
                    });
                }
            }
        }

        self.buffer.push_back(message);
        self.stats.inc_enqueued();
        Ok(())
    }

    /// Pop a message from the queue
    ///
    /// Returns `None` if the queue is empty.
    pub fn pop(&mut self) -> Option<CanMessage> {
        let msg = self.buffer.pop_front();
        if msg.is_some() {
            self.stats.inc_dequeued();
        }
        msg
    }

    /// Peek at the next message without removing it
    pub fn peek(&self) -> Option<&CanMessage> {
        self.buffer.front()
    }

    /// Clear all messages from the queue
    pub fn clear(&mut self) {
        self.buffer.clear();
    }

    /// Adjust the queue capacity
    ///
    /// If the new capacity is smaller than the current number of messages,
    /// excess messages are removed according to the overflow policy.
    ///
    /// # Arguments
    ///
    /// * `new_capacity` - New maximum capacity
    pub fn adjust_capacity(&mut self, new_capacity: usize) {
        while self.buffer.len() > new_capacity {
            match self.policy {
                QueueOverflowPolicy::DropOldest | QueueOverflowPolicy::Block { .. } => {
                    if self.buffer.pop_front().is_some() {
                        self.stats.inc_dropped();
                    }
                }
                QueueOverflowPolicy::DropNewest => {
                    if self.buffer.pop_back().is_some() {
                        self.stats.inc_dropped();
                    }
                }
            }
        }
        self.capacity = new_capacity;
    }

    /// Iterate over messages without removing them
    pub fn iter(&self) -> impl Iterator<Item = &CanMessage> {
        self.buffer.iter()
    }
}

impl Default for BoundedQueue {
    fn default() -> Self {
        Self::new(DEFAULT_QUEUE_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CanId;

    fn make_test_message(id: u16) -> CanMessage {
        CanMessage::new_standard(id, &[0u8; 8]).unwrap()
    }

    #[test]
    fn test_new_queue() {
        let queue = BoundedQueue::new(100);
        assert_eq!(queue.capacity(), 100);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
    }

    #[test]
    fn test_push_pop() {
        let mut queue = BoundedQueue::new(10);
        let msg = make_test_message(0x123);

        assert!(queue.push(msg.clone()).is_ok());
        assert_eq!(queue.len(), 1);

        let popped = queue.pop();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().id(), CanId::Standard(0x123));
        assert!(queue.is_empty());
    }

    #[test]
    fn test_drop_oldest_policy() {
        let mut queue = BoundedQueue::with_policy(3, QueueOverflowPolicy::DropOldest);

        // Fill the queue
        queue.push(make_test_message(1)).unwrap();
        queue.push(make_test_message(2)).unwrap();
        queue.push(make_test_message(3)).unwrap();
        assert!(queue.is_full());

        // Push one more - should drop oldest (1)
        queue.push(make_test_message(4)).unwrap();
        assert_eq!(queue.len(), 3);

        // Verify oldest was dropped
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(2));
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(3));
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(4));

        let stats = queue.stats();
        assert_eq!(stats.dropped, 1);
        assert_eq!(stats.overflow_count, 1);
    }

    #[test]
    fn test_drop_newest_policy() {
        let mut queue = BoundedQueue::with_policy(3, QueueOverflowPolicy::DropNewest);

        // Fill the queue
        queue.push(make_test_message(1)).unwrap();
        queue.push(make_test_message(2)).unwrap();
        queue.push(make_test_message(3)).unwrap();

        // Push one more - should reject it
        let result = queue.push(make_test_message(4));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            QueueError::MessageDropped { .. }
        ));

        // Verify queue unchanged
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(1));
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(2));
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(3));
    }

    #[test]
    fn test_block_policy_sync() {
        use std::time::Duration;

        let mut queue = BoundedQueue::with_policy(
            2,
            QueueOverflowPolicy::Block {
                timeout: Duration::from_millis(100),
            },
        );

        queue.push(make_test_message(1)).unwrap();
        queue.push(make_test_message(2)).unwrap();

        // Sync push should return QueueFull error
        let result = queue.push(make_test_message(3));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), QueueError::QueueFull { .. }));
    }

    #[test]
    fn test_adjust_capacity() {
        let mut queue = BoundedQueue::new(10);

        // Add 5 messages
        for i in 0..5u16 {
            queue.push(make_test_message(i)).unwrap();
        }

        // Reduce capacity to 3
        queue.adjust_capacity(3);
        assert_eq!(queue.capacity(), 3);
        assert_eq!(queue.len(), 3);

        // With DropOldest, oldest messages should be removed
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(2));
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(3));
        assert_eq!(queue.pop().unwrap().id(), CanId::Standard(4));
    }

    #[test]
    fn test_stats() {
        let mut queue = BoundedQueue::with_policy(2, QueueOverflowPolicy::DropOldest);

        queue.push(make_test_message(1)).unwrap();
        queue.push(make_test_message(2)).unwrap();
        queue.push(make_test_message(3)).unwrap(); // Causes overflow
        queue.pop();

        let stats = queue.stats();
        assert_eq!(stats.enqueued, 3);
        assert_eq!(stats.dequeued, 1);
        assert_eq!(stats.dropped, 1);
        assert_eq!(stats.overflow_count, 1);
    }

    #[test]
    fn test_default_queue() {
        let queue = BoundedQueue::default();
        assert_eq!(queue.capacity(), DEFAULT_QUEUE_CAPACITY);
    }
}
