//! Queue overflow policy (FR-011)
//!
//! Defines strategies for handling queue overflow situations.

use std::time::Duration;

/// Queue overflow handling policy
///
/// Determines how the queue behaves when it reaches capacity.
///
/// # Default
///
/// The default policy is `DropOldest`, which is suitable for real-time
/// monitoring scenarios where the latest data is most important.
///
/// # Example
///
/// ```rust
/// use canlink_hal::queue::QueueOverflowPolicy;
/// use std::time::Duration;
///
/// // Default policy
/// let policy = QueueOverflowPolicy::default();
/// assert!(matches!(policy, QueueOverflowPolicy::DropOldest));
///
/// // Block with timeout
/// let policy = QueueOverflowPolicy::Block {
///     timeout: Duration::from_millis(100),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueueOverflowPolicy {
    /// Drop the oldest message in the queue
    ///
    /// This is the default policy, suitable for real-time monitoring
    /// where the latest data is most important.
    #[default]
    DropOldest,

    /// Drop the newest message (the one being added)
    ///
    /// Suitable for data recording scenarios where preserving
    /// complete history is important.
    DropNewest,

    /// Block until space is available or timeout expires
    ///
    /// Suitable for critical messages that should not be lost.
    /// Returns `QueueError::QueueFull` if timeout expires.
    Block {
        /// Maximum time to wait for space
        timeout: Duration,
    },
}

impl QueueOverflowPolicy {
    /// Create a new `DropOldest` policy
    #[must_use]
    pub fn drop_oldest() -> Self {
        Self::DropOldest
    }

    /// Create a new `DropNewest` policy
    #[must_use]
    pub fn drop_newest() -> Self {
        Self::DropNewest
    }

    /// Create a new Block policy with the given timeout
    #[must_use]
    pub fn block(timeout: Duration) -> Self {
        Self::Block { timeout }
    }

    /// Check if this policy may block
    #[must_use]
    pub fn may_block(&self) -> bool {
        matches!(self, Self::Block { .. })
    }

    /// Get the timeout if this is a Block policy
    #[must_use]
    pub fn timeout(&self) -> Option<Duration> {
        match self {
            Self::Block { timeout } => Some(*timeout),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = QueueOverflowPolicy::default();
        assert!(matches!(policy, QueueOverflowPolicy::DropOldest));
    }

    #[test]
    fn test_constructors() {
        assert!(matches!(
            QueueOverflowPolicy::drop_oldest(),
            QueueOverflowPolicy::DropOldest
        ));
        assert!(matches!(
            QueueOverflowPolicy::drop_newest(),
            QueueOverflowPolicy::DropNewest
        ));

        let timeout = Duration::from_millis(100);
        let policy = QueueOverflowPolicy::block(timeout);
        assert!(matches!(policy, QueueOverflowPolicy::Block { .. }));
        assert_eq!(policy.timeout(), Some(timeout));
    }

    #[test]
    fn test_may_block() {
        assert!(!QueueOverflowPolicy::DropOldest.may_block());
        assert!(!QueueOverflowPolicy::DropNewest.may_block());
        assert!(QueueOverflowPolicy::block(Duration::from_millis(100)).may_block());
    }
}
