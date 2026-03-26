//! Hardware capability types.
//!
//! This module defines types for describing hardware capabilities and limitations.

use serde::{Deserialize, Serialize};

/// Hardware capability description.
///
/// Describes the capabilities and limitations of a CAN hardware backend.
/// Applications can query these capabilities at runtime to adapt their behavior.
///
/// # Examples
///
/// ```
/// use canlink_hal::{HardwareCapability, TimestampPrecision};
///
/// let capability = HardwareCapability {
///     channel_count: 2,
///     supports_canfd: true,
///     max_bitrate: 8_000_000,
///     supported_bitrates: vec![125_000, 250_000, 500_000, 1_000_000],
///     filter_count: 16,
///     timestamp_precision: TimestampPrecision::Microsecond,
/// };
///
/// assert_eq!(capability.channel_count, 2);
/// assert!(capability.supports_canfd);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareCapability {
    /// Number of supported channels
    pub channel_count: u8,

    /// Whether CAN-FD is supported
    pub supports_canfd: bool,

    /// Maximum bitrate in bits per second
    pub max_bitrate: u32,

    /// List of supported bitrates (bps)
    pub supported_bitrates: Vec<u32>,

    /// Number of hardware filters supported
    pub filter_count: u8,

    /// Timestamp precision
    pub timestamp_precision: TimestampPrecision,
}

impl HardwareCapability {
    /// Create a new hardware capability description.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::{HardwareCapability, TimestampPrecision};
    ///
    /// let capability = HardwareCapability::new(
    ///     2,
    ///     true,
    ///     8_000_000,
    ///     vec![125_000, 250_000, 500_000, 1_000_000],
    ///     16,
    ///     TimestampPrecision::Microsecond,
    /// );
    /// ```
    #[must_use]
    pub fn new(
        channel_count: u8,
        supports_canfd: bool,
        max_bitrate: u32,
        supported_bitrates: Vec<u32>,
        filter_count: u8,
        timestamp_precision: TimestampPrecision,
    ) -> Self {
        Self {
            channel_count,
            supports_canfd,
            max_bitrate,
            supported_bitrates,
            filter_count,
            timestamp_precision,
        }
    }

    /// Check if a specific bitrate is supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::{HardwareCapability, TimestampPrecision};
    ///
    /// let capability = HardwareCapability::new(
    ///     2,
    ///     true,
    ///     8_000_000,
    ///     vec![125_000, 250_000, 500_000, 1_000_000],
    ///     16,
    ///     TimestampPrecision::Microsecond,
    /// );
    ///
    /// assert!(capability.supports_bitrate(500_000));
    /// assert!(!capability.supports_bitrate(2_000_000));
    /// ```
    #[must_use]
    pub fn supports_bitrate(&self, bitrate: u32) -> bool {
        self.supported_bitrates.contains(&bitrate)
    }

    /// Check if a specific channel exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::{HardwareCapability, TimestampPrecision};
    ///
    /// let capability = HardwareCapability::new(
    ///     2,
    ///     true,
    ///     8_000_000,
    ///     vec![125_000, 250_000, 500_000, 1_000_000],
    ///     16,
    ///     TimestampPrecision::Microsecond,
    /// );
    ///
    /// assert!(capability.has_channel(0));
    /// assert!(capability.has_channel(1));
    /// assert!(!capability.has_channel(2));
    /// ```
    #[must_use]
    pub fn has_channel(&self, channel: u8) -> bool {
        channel < self.channel_count
    }
}

/// Timestamp precision.
///
/// Indicates the precision of timestamps provided by the hardware.
///
/// # Examples
///
/// ```
/// use canlink_hal::TimestampPrecision;
///
/// let precision = TimestampPrecision::Microsecond;
/// assert_eq!(precision.resolution_us(), Some(1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TimestampPrecision {
    /// Microsecond precision (1 µs)
    Microsecond,

    /// Millisecond precision (1 ms = 1000 µs)
    Millisecond,

    /// No timestamp support
    None,
}

impl TimestampPrecision {
    /// Get the resolution in microseconds.
    ///
    /// Returns `None` if timestamps are not supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::TimestampPrecision;
    ///
    /// assert_eq!(TimestampPrecision::Microsecond.resolution_us(), Some(1));
    /// assert_eq!(TimestampPrecision::Millisecond.resolution_us(), Some(1000));
    /// assert_eq!(TimestampPrecision::None.resolution_us(), None);
    /// ```
    #[must_use]
    pub const fn resolution_us(&self) -> Option<u64> {
        match self {
            Self::Microsecond => Some(1),
            Self::Millisecond => Some(1000),
            Self::None => None,
        }
    }

    /// Check if timestamps are supported.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::TimestampPrecision;
    ///
    /// assert!(TimestampPrecision::Microsecond.is_supported());
    /// assert!(TimestampPrecision::Millisecond.is_supported());
    /// assert!(!TimestampPrecision::None.is_supported());
    /// ```
    #[must_use]
    pub const fn is_supported(&self) -> bool {
        !matches!(self, Self::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_capability() {
        let capability = HardwareCapability::new(
            2,
            true,
            8_000_000,
            vec![125_000, 250_000, 500_000, 1_000_000],
            16,
            TimestampPrecision::Microsecond,
        );

        assert_eq!(capability.channel_count, 2);
        assert!(capability.supports_canfd);
        assert_eq!(capability.max_bitrate, 8_000_000);
        assert_eq!(capability.filter_count, 16);
    }

    #[test]
    fn test_supports_bitrate() {
        let capability = HardwareCapability::new(
            2,
            true,
            8_000_000,
            vec![125_000, 250_000, 500_000, 1_000_000],
            16,
            TimestampPrecision::Microsecond,
        );

        assert!(capability.supports_bitrate(500_000));
        assert!(!capability.supports_bitrate(2_000_000));
    }

    #[test]
    fn test_has_channel() {
        let capability = HardwareCapability::new(
            2,
            true,
            8_000_000,
            vec![125_000, 250_000, 500_000, 1_000_000],
            16,
            TimestampPrecision::Microsecond,
        );

        assert!(capability.has_channel(0));
        assert!(capability.has_channel(1));
        assert!(!capability.has_channel(2));
    }

    #[test]
    fn test_timestamp_precision() {
        assert_eq!(TimestampPrecision::Microsecond.resolution_us(), Some(1));
        assert_eq!(TimestampPrecision::Millisecond.resolution_us(), Some(1000));
        assert_eq!(TimestampPrecision::None.resolution_us(), None);

        assert!(TimestampPrecision::Microsecond.is_supported());
        assert!(TimestampPrecision::Millisecond.is_supported());
        assert!(!TimestampPrecision::None.is_supported());
    }
}
