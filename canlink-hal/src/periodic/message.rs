//! Periodic message configuration.

use crate::{CanError, CanMessage};
use std::time::Duration;

/// Minimum allowed interval (1ms)
pub const MIN_INTERVAL_MS: u64 = 1;

/// Maximum allowed interval (10000ms = 10s)
pub const MAX_INTERVAL_MS: u64 = 10_000;

/// Periodic message configuration.
///
/// Represents a CAN message that should be sent at a fixed time interval.
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::periodic::PeriodicMessage;
/// use canlink_hal::CanMessage;
/// use std::time::Duration;
///
/// let msg = CanMessage::new_standard(0x123, &[0x01, 0x02])?;
/// let periodic = PeriodicMessage::new(msg, Duration::from_millis(100))?;
/// ```
#[derive(Debug, Clone)]
pub struct PeriodicMessage {
    /// Unique identifier for this periodic message
    id: u32,
    /// The CAN message to send
    message: CanMessage,
    /// Send interval
    interval: Duration,
    /// Whether sending is enabled
    enabled: bool,
}

impl PeriodicMessage {
    /// Create a new periodic message.
    ///
    /// # Arguments
    ///
    /// * `message` - The CAN message to send periodically
    /// * `interval` - Send interval (must be between 1ms and 10000ms)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if the interval is out of range.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let msg = CanMessage::new_standard(0x123, &[0x01])?;
    /// let periodic = PeriodicMessage::new(msg, Duration::from_millis(100))?;
    /// ```
    pub fn new(message: CanMessage, interval: Duration) -> Result<Self, CanError> {
        Self::validate_interval(interval)?;

        Ok(Self {
            id: 0, // Will be assigned by scheduler
            message,
            interval,
            enabled: true,
        })
    }

    /// Validate that the interval is within allowed range.
    fn validate_interval(interval: Duration) -> Result<(), CanError> {
        let ms_u128 = interval.as_millis();
        let ms = u64::try_from(ms_u128).map_err(|_| CanError::InvalidParameter {
            parameter: "interval".to_string(),
            reason: format!(
                "interval must be between {MIN_INTERVAL_MS}ms and {MAX_INTERVAL_MS}ms, got {ms_u128}ms"
            ),
        })?;
        if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&ms) {
            return Err(CanError::InvalidParameter {
                parameter: "interval".to_string(),
                reason: format!(
                    "interval must be between {MIN_INTERVAL_MS}ms and {MAX_INTERVAL_MS}ms, got {ms}ms"
                ),
            });
        }
        Ok(())
    }

    /// Get the unique identifier.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Set the unique identifier (used internally by scheduler).
    pub(crate) fn set_id(&mut self, id: u32) {
        self.id = id;
    }

    /// Get a reference to the CAN message.
    #[must_use]
    pub fn message(&self) -> &CanMessage {
        &self.message
    }

    /// Get the send interval.
    #[must_use]
    pub fn interval(&self) -> Duration {
        self.interval
    }

    /// Check if sending is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update the message data.
    ///
    /// # Arguments
    ///
    /// * `data` - New data bytes for the message
    ///
    /// # Errors
    ///
    /// Returns an error if the data length is invalid for the message type.
    #[allow(clippy::needless_pass_by_value)]
    pub fn update_data(&mut self, data: Vec<u8>) -> Result<(), CanError> {
        use crate::message::MessageFlags;
        use crate::CanId;

        // Create a new message with the same ID but new data
        let new_message = if self.message.flags().contains(MessageFlags::FD) {
            // CAN-FD message
            CanMessage::new_fd(self.message.id(), &data)?
        } else {
            // CAN 2.0 message
            match self.message.id() {
                CanId::Standard(id) => CanMessage::new_standard(id, &data)?,
                CanId::Extended(id) => CanMessage::new_extended(id, &data)?,
            }
        };
        self.message = new_message;
        Ok(())
    }

    /// Update the send interval.
    ///
    /// # Arguments
    ///
    /// * `interval` - New send interval (must be between 1ms and 10000ms)
    ///
    /// # Errors
    ///
    /// Returns `CanError::InvalidParameter` if the interval is out of range.
    pub fn set_interval(&mut self, interval: Duration) -> Result<(), CanError> {
        Self::validate_interval(interval)?;
        self.interval = interval;
        Ok(())
    }

    /// Enable or disable sending.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CanId;

    fn create_test_message() -> CanMessage {
        CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04]).unwrap()
    }

    #[test]
    fn test_new_periodic_message() {
        let msg = create_test_message();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        assert_eq!(periodic.interval(), Duration::from_millis(100));
        assert!(periodic.is_enabled());
        assert_eq!(periodic.message().id(), CanId::Standard(0x123));
    }

    #[test]
    fn test_interval_validation_min() {
        let msg = create_test_message();
        let result = PeriodicMessage::new(msg, Duration::from_millis(0));
        assert!(result.is_err());
    }

    #[test]
    fn test_interval_validation_max() {
        let msg = create_test_message();
        let result = PeriodicMessage::new(msg, Duration::from_millis(10_001));
        assert!(result.is_err());
    }

    #[test]
    fn test_interval_validation_valid_min() {
        let msg = create_test_message();
        let result = PeriodicMessage::new(msg, Duration::from_millis(1));
        assert!(result.is_ok());
    }

    #[test]
    fn test_interval_validation_valid_max() {
        let msg = create_test_message();
        let result = PeriodicMessage::new(msg, Duration::from_millis(10_000));
        assert!(result.is_ok());
    }

    #[test]
    fn test_update_data() {
        let msg = create_test_message();
        let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        periodic.update_data(vec![0xAA, 0xBB]).unwrap();
        assert_eq!(periodic.message().data(), &[0xAA, 0xBB]);
    }

    #[test]
    fn test_set_interval() {
        let msg = create_test_message();
        let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        periodic.set_interval(Duration::from_millis(200)).unwrap();
        assert_eq!(periodic.interval(), Duration::from_millis(200));
    }

    #[test]
    fn test_set_enabled() {
        let msg = create_test_message();
        let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        assert!(periodic.is_enabled());
        periodic.set_enabled(false);
        assert!(!periodic.is_enabled());
        periodic.set_enabled(true);
        assert!(periodic.is_enabled());
    }
}
