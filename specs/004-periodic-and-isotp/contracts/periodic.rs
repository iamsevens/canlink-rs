//! Periodic Message Sending API Contract
//!
//! This file defines the public API contract for the periodic message sending feature.
//! Implementation must conform to these signatures and behaviors.
//!
//! # Feature: `periodic`
//!
//! Enable with:
//! ```toml
//! [dependencies]
//! canlink-hal = { version = "0.3", features = ["periodic"] }
//! ```

use crate::{CanBackendAsync, CanError, CanMessage};
use std::time::Duration;

// ============================================================================
// Constants
// ============================================================================

/// Minimum allowed periodic interval
pub const MIN_INTERVAL: Duration = Duration::from_millis(1);

/// Maximum allowed periodic interval
pub const MAX_INTERVAL: Duration = Duration::from_millis(10000);

/// Default scheduler capacity (maximum number of periodic messages)
pub const DEFAULT_CAPACITY: usize = 32;

// ============================================================================
// PeriodicMessage
// ============================================================================

/// A message configured for periodic transmission.
///
/// # Example
///
/// ```rust
/// use canlink_hal::{CanMessage, periodic::PeriodicMessage};
/// use std::time::Duration;
///
/// let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
/// let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();
///
/// assert_eq!(periodic.interval(), Duration::from_millis(100));
/// assert!(periodic.is_enabled());
/// ```
#[derive(Debug, Clone)]
pub struct PeriodicMessage {
    // Internal fields omitted from contract
}

impl PeriodicMessage {
    /// Creates a new periodic message.
    ///
    /// # Arguments
    ///
    /// * `message` - The CAN message to send periodically
    /// * `interval` - Send interval (1ms to 10000ms)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if:
    /// - `interval` < 1ms
    /// - `interval` > 10000ms
    ///
    /// # Example
    ///
    /// ```rust
    /// use canlink_hal::{CanMessage, periodic::PeriodicMessage};
    /// use std::time::Duration;
    ///
    /// // Valid interval
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// let periodic = PeriodicMessage::new(msg, Duration::from_millis(100));
    /// assert!(periodic.is_ok());
    ///
    /// // Invalid interval (too small)
    /// let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    /// let periodic = PeriodicMessage::new(msg, Duration::from_micros(500));
    /// assert!(periodic.is_err());
    /// ```
    pub fn new(message: CanMessage, interval: Duration) -> Result<Self, CanError>;

    /// Returns the unique identifier for this periodic message.
    pub fn id(&self) -> u32;

    /// Returns a reference to the CAN message.
    pub fn message(&self) -> &CanMessage;

    /// Returns the send interval.
    pub fn interval(&self) -> Duration;

    /// Returns whether this message is enabled for sending.
    pub fn is_enabled(&self) -> bool;

    /// Updates the message data without changing the CAN ID.
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if data length exceeds message capacity.
    pub fn update_data(&mut self, data: Vec<u8>) -> Result<(), CanError>;

    /// Updates the send interval.
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if interval is out of range.
    pub fn set_interval(&mut self, interval: Duration) -> Result<(), CanError>;

    /// Enables or disables periodic sending.
    pub fn set_enabled(&mut self, enabled: bool);
}

// ============================================================================
// PeriodicStats
// ============================================================================

/// Statistics for a periodic message.
///
/// # Example
///
/// ```rust
/// use canlink_hal::periodic::PeriodicStats;
///
/// let stats = PeriodicStats::new();
/// assert_eq!(stats.send_count(), 0);
/// assert!(stats.average_interval().is_none());
/// ```
#[derive(Debug, Clone, Default)]
pub struct PeriodicStats {
    // Internal fields omitted from contract
}

impl PeriodicStats {
    /// Creates a new statistics instance.
    pub fn new() -> Self;

    /// Returns the total number of messages sent.
    pub fn send_count(&self) -> u64;

    /// Returns the time of the last send, if any.
    pub fn last_send_time(&self) -> Option<std::time::Instant>;

    /// Returns the average actual interval between sends.
    ///
    /// Returns `None` if fewer than 2 messages have been sent.
    pub fn average_interval(&self) -> Option<Duration>;

    /// Returns the minimum observed interval.
    pub fn min_interval(&self) -> Option<Duration>;

    /// Returns the maximum observed interval.
    pub fn max_interval(&self) -> Option<Duration>;

    /// Resets all statistics to zero.
    pub fn reset(&mut self);
}

// ============================================================================
// PeriodicScheduler
// ============================================================================

/// Scheduler for managing periodic message transmission.
///
/// The scheduler runs as an async task and manages multiple periodic messages
/// using a priority queue for efficient scheduling.
///
/// # Example
///
/// ```rust,no_run
/// use canlink_hal::{CanMessage, CanBackendAsync};
/// use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let backend = /* create backend */;
///
///     // Create scheduler
///     let scheduler = PeriodicScheduler::new(backend, 32).await?;
///
///     // Add a periodic message
///     let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
///     let periodic = PeriodicMessage::new(msg, Duration::from_millis(100))?;
///     let id = scheduler.add(periodic).await?;
///
///     // Update data dynamically
///     scheduler.update_data(id, vec![5, 6, 7, 8]).await?;
///
///     // Get statistics
///     if let Some(stats) = scheduler.get_stats(id).await? {
///         println!("Sent {} messages", stats.send_count());
///     }
///
///     // Shutdown
///     scheduler.shutdown().await?;
///
///     Ok(())
/// }
/// ```
pub struct PeriodicScheduler {
    // Internal fields omitted from contract
}

impl PeriodicScheduler {
    /// Creates and starts a new periodic scheduler.
    ///
    /// # Arguments
    ///
    /// * `backend` - The CAN backend to use for sending messages
    /// * `capacity` - Maximum number of periodic messages (default: 32)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if capacity is 0.
    pub async fn new<B: CanBackendAsync + 'static>(
        backend: B,
        capacity: usize,
    ) -> Result<Self, CanError>;

    /// Adds a periodic message to the scheduler.
    ///
    /// # Returns
    ///
    /// The unique ID assigned to this periodic message.
    ///
    /// # Errors
    ///
    /// Returns `CanError::QueueError` if the scheduler is at capacity.
    pub async fn add(&self, message: PeriodicMessage) -> Result<u32, CanError>;

    /// Removes a periodic message from the scheduler.
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if the ID is not found.
    pub async fn remove(&self, id: u32) -> Result<(), CanError>;

    /// Updates the data of a periodic message.
    ///
    /// The update takes effect on the next scheduled send.
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if the ID is not found or data is invalid.
    pub async fn update_data(&self, id: u32, data: Vec<u8>) -> Result<(), CanError>;

    /// Updates the interval of a periodic message.
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if the ID is not found or interval is invalid.
    pub async fn update_interval(&self, id: u32, interval: Duration) -> Result<(), CanError>;

    /// Enables or disables a periodic message.
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if the ID is not found.
    pub async fn set_enabled(&self, id: u32, enabled: bool) -> Result<(), CanError>;

    /// Gets statistics for a periodic message.
    ///
    /// # Returns
    ///
    /// `Some(stats)` if the message exists, `None` otherwise.
    pub async fn get_stats(&self, id: u32) -> Result<Option<PeriodicStats>, CanError>;

    /// Lists all periodic message IDs.
    pub async fn list_ids(&self) -> Result<Vec<u32>, CanError>;

    /// Returns the current number of periodic messages.
    pub async fn len(&self) -> Result<usize, CanError>;

    /// Returns whether the scheduler is empty.
    pub async fn is_empty(&self) -> Result<bool, CanError>;

    /// Shuts down the scheduler and releases resources.
    ///
    /// All periodic messages are stopped. This method consumes the scheduler.
    pub async fn shutdown(self) -> Result<(), CanError>;
}

// ============================================================================
// Tests (Contract Verification)
// ============================================================================

#[cfg(test)]
mod contract_tests {
    use super::*;

    /// FR-001: Interval range validation
    #[test]
    fn test_interval_range() {
        // Valid: 1ms
        assert!(PeriodicMessage::new(
            CanMessage::new_standard(0x123, &[1]).unwrap(),
            Duration::from_millis(1)
        ).is_ok());

        // Valid: 10000ms
        assert!(PeriodicMessage::new(
            CanMessage::new_standard(0x123, &[1]).unwrap(),
            Duration::from_millis(10000)
        ).is_ok());

        // Invalid: 0ms
        assert!(PeriodicMessage::new(
            CanMessage::new_standard(0x123, &[1]).unwrap(),
            Duration::ZERO
        ).is_err());

        // Invalid: > 10000ms
        assert!(PeriodicMessage::new(
            CanMessage::new_standard(0x123, &[1]).unwrap(),
            Duration::from_millis(10001)
        ).is_err());
    }

    /// FR-002: Dynamic data update
    #[tokio::test]
    async fn test_dynamic_data_update() {
        // Data update should not interrupt sending cycle
        // Implementation test will verify timing
    }

    /// FR-003: Start/stop individual messages
    #[tokio::test]
    async fn test_enable_disable() {
        // set_enabled(false) should stop sending
        // set_enabled(true) should resume sending
    }

    /// FR-004: Multiple concurrent messages
    #[tokio::test]
    async fn test_capacity() {
        // Should support at least 32 concurrent periodic messages
    }

    /// FR-005: Statistics tracking
    #[test]
    fn test_statistics() {
        let mut stats = PeriodicStats::new();
        assert_eq!(stats.send_count(), 0);
        assert!(stats.average_interval().is_none());
    }
}
