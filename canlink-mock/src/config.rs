//! Mock backend configuration.
//!
//! This module provides configuration structures for the Mock backend,
//! allowing customization of hardware capabilities, preset messages, and error injection.

use canlink_hal::{CanMessage, HardwareCapability, TimestampPrecision};
use serde::{Deserialize, Serialize};

/// Mock backend configuration.
///
/// Allows customization of the mock backend's behavior, including hardware capabilities,
/// preset messages to return, and error injection scenarios.
///
/// # Examples
///
/// ```
/// use canlink_mock::MockConfig;
///
/// let config = MockConfig::default();
/// assert_eq!(config.channel_count, 2);
/// assert!(config.supports_canfd);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct MockConfig {
    /// Number of CAN channels to simulate
    pub channel_count: u8,

    /// Whether to simulate CAN-FD support
    pub supports_canfd: bool,

    /// Maximum bitrate in bits per second
    pub max_bitrate: u32,

    /// List of supported bitrates
    pub supported_bitrates: Vec<u32>,

    /// Number of hardware filters
    pub filter_count: u16,

    /// Timestamp precision
    pub timestamp_precision: TimestampPrecision,

    /// Preset messages to return from `receive_message()`
    pub preset_messages: Vec<CanMessage>,

    /// Whether to simulate initialization failure
    pub fail_initialization: bool,

    /// Whether to simulate send failures
    pub fail_send: bool,

    /// Whether to simulate receive failures
    pub fail_receive: bool,

    /// Maximum number of messages to record (0 = unlimited)
    pub max_recorded_messages: usize,
}

impl Default for MockConfig {
    fn default() -> Self {
        Self {
            channel_count: 2,
            supports_canfd: true,
            max_bitrate: 8_000_000,
            supported_bitrates: vec![125_000, 250_000, 500_000, 1_000_000],
            filter_count: 16,
            timestamp_precision: TimestampPrecision::Microsecond,
            preset_messages: Vec::new(),
            fail_initialization: false,
            fail_send: false,
            fail_receive: false,
            max_recorded_messages: 0, // Unlimited
        }
    }
}

impl MockConfig {
    /// Create a new mock configuration with default values.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockConfig;
    ///
    /// let config = MockConfig::new();
    /// assert_eq!(config.channel_count, 2);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a configuration for a simple CAN 2.0 device.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockConfig;
    ///
    /// let config = MockConfig::can20_only();
    /// assert!(!config.supports_canfd);
    /// assert_eq!(config.channel_count, 1);
    /// ```
    #[must_use]
    pub fn can20_only() -> Self {
        Self {
            channel_count: 1,
            supports_canfd: false,
            max_bitrate: 1_000_000,
            supported_bitrates: vec![125_000, 250_000, 500_000, 1_000_000],
            filter_count: 8,
            timestamp_precision: TimestampPrecision::Millisecond,
            preset_messages: Vec::new(),
            fail_initialization: false,
            fail_send: false,
            fail_receive: false,
            max_recorded_messages: 0,
        }
    }

    /// Create a configuration with preset messages.
    ///
    /// # Arguments
    ///
    /// * `messages` - Messages to return from `receive_message()`
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockConfig;
    /// use canlink_hal::CanMessage;
    ///
    /// let messages = vec![
    ///     CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap(),
    /// ];
    /// let config = MockConfig::with_preset_messages(messages);
    /// assert_eq!(config.preset_messages.len(), 1);
    /// ```
    #[must_use]
    pub fn with_preset_messages(messages: Vec<CanMessage>) -> Self {
        Self {
            preset_messages: messages,
            ..Self::default()
        }
    }

    /// Convert to `HardwareCapability`.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::MockConfig;
    ///
    /// let config = MockConfig::default();
    /// let capability = config.to_capability();
    /// assert_eq!(capability.channel_count, 2);
    /// ```
    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    pub fn to_capability(&self) -> HardwareCapability {
        HardwareCapability::new(
            self.channel_count,
            self.supports_canfd,
            self.max_bitrate,
            self.supported_bitrates.clone(),
            self.filter_count as u8,
            self.timestamp_precision,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MockConfig::default();
        assert_eq!(config.channel_count, 2);
        assert!(config.supports_canfd);
        assert_eq!(config.max_bitrate, 8_000_000);
        assert!(!config.fail_initialization);
    }

    #[test]
    fn test_new_config() {
        let config = MockConfig::new();
        assert_eq!(config.channel_count, 2);
    }

    #[test]
    fn test_can20_only() {
        let config = MockConfig::can20_only();
        assert!(!config.supports_canfd);
        assert_eq!(config.channel_count, 1);
        assert_eq!(config.max_bitrate, 1_000_000);
    }

    #[test]
    fn test_with_preset_messages() {
        let messages = vec![
            CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap(),
            CanMessage::new_standard(0x456, &[4, 5, 6]).unwrap(),
        ];
        let config = MockConfig::with_preset_messages(messages);
        assert_eq!(config.preset_messages.len(), 2);
    }

    #[test]
    fn test_to_capability() {
        let config = MockConfig::default();
        let capability = config.to_capability();
        assert_eq!(capability.channel_count, config.channel_count);
        assert_eq!(capability.supports_canfd, config.supports_canfd);
        assert_eq!(capability.max_bitrate, config.max_bitrate);
    }
}
