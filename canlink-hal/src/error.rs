//! Error types for the CAN hardware abstraction layer.
//!
//! This module provides unified error types that are hardware-independent.
//! All backends use these error types for consistent error handling.

use thiserror::Error;

/// Unified CAN error type.
///
/// This enum represents all possible errors that can occur when working with
/// CAN hardware through the abstraction layer. Error codes are organized into
/// ranges:
///
/// - 1000-1999: Hardware-related errors
/// - 2000-2999: Protocol-related errors
/// - 3000-3999: Configuration-related errors
/// - 4000-4999: System-related errors
///
/// # Examples
///
/// ```
/// use canlink_hal::CanError;
///
/// let err = CanError::InvalidId { value: 0x800, max: 0x7FF };
/// assert!(matches!(err, CanError::InvalidId { .. }));
/// ```
#[derive(Error, Debug)]
pub enum CanError {
    // Hardware-related errors (1000-1999)
    /// Backend not found (1001)
    #[error("[1001] Backend not found: {name}")]
    BackendNotFound {
        /// Backend name that was not found
        name: String,
    },

    /// Backend already registered (1002)
    #[error("[1002] Backend '{name}' is already registered")]
    BackendAlreadyRegistered {
        /// Backend name that is already registered
        name: String,
    },

    /// Backend initialization failed (1003)
    #[error("[1003] Backend initialization failed: {reason}")]
    InitializationFailed {
        /// Reason for initialization failure
        reason: String,
    },

    /// Device not found (1004)
    #[error("[1004] Device not found: {device}")]
    DeviceNotFound {
        /// Device identifier
        device: String,
    },

    /// Channel not found (1005)
    #[error("[1005] Channel {channel} does not exist (max: {max})")]
    ChannelNotFound {
        /// Channel number that was requested
        channel: u8,
        /// Maximum channel number available
        max: u8,
    },

    /// Channel already open (1006)
    #[error("[1006] Channel {channel} is already open")]
    ChannelAlreadyOpen {
        /// Channel number
        channel: u8,
    },

    /// Channel not open (1007)
    #[error("[1007] Channel {channel} is not open")]
    ChannelNotOpen {
        /// Channel number
        channel: u8,
    },

    // Protocol-related errors (2000-2999)
    /// Invalid CAN ID (2001)
    #[error("[2001] Invalid CAN ID: {value:#X} (max: {max:#X})")]
    InvalidId {
        /// The invalid ID value
        value: u32,
        /// Maximum allowed ID value
        max: u32,
    },

    /// Invalid data length (2002)
    #[error("[2002] Invalid data length: expected max {expected}, got {actual}")]
    InvalidDataLength {
        /// Expected maximum length
        expected: usize,
        /// Actual length provided
        actual: usize,
    },

    /// Invalid message format (2003)
    #[error("[2003] Invalid message format: {reason}")]
    InvalidFormat {
        /// Reason for format error
        reason: String,
    },

    // Configuration-related errors (3000-3999)
    /// Configuration error (3001)
    #[error("[3001] Configuration error: {reason}")]
    ConfigError {
        /// Reason for configuration error
        reason: String,
    },

    /// Invalid parameter (3002)
    #[error("[3002] Invalid parameter '{parameter}': {reason}")]
    InvalidParameter {
        /// Parameter name
        parameter: String,
        /// Reason why parameter is invalid
        reason: String,
    },

    /// Version incompatible (3003)
    #[error("[3003] Version incompatible: backend {backend_version}, expected {expected_version}")]
    VersionIncompatible {
        /// Backend version
        backend_version: String,
        /// Expected version
        expected_version: String,
    },

    // System-related errors (4000-4999)
    /// Operation timed out (4001)
    #[error("[4001] Operation timed out after {timeout_ms}ms")]
    Timeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Insufficient resources (4002)
    #[error("[4002] Insufficient resources: {resource}")]
    InsufficientResources {
        /// Resource that is insufficient
        resource: String,
    },

    /// Permission denied (4003)
    #[error("[4003] Permission denied: {operation}")]
    PermissionDenied {
        /// Operation that was denied
        operation: String,
    },

    // Operation errors
    /// Send operation failed
    #[error("Send failed: {reason}")]
    SendFailed {
        /// Reason for send failure
        reason: String,
    },

    /// Receive operation failed
    #[error("Receive failed: {reason}")]
    ReceiveFailed {
        /// Reason for receive failure
        reason: String,
    },

    /// Bus error occurred
    #[error("Bus error: {kind:?}")]
    BusError {
        /// Type of bus error
        kind: BusErrorKind,
    },

    /// Feature not supported by hardware
    #[error("Unsupported feature: {feature}")]
    UnsupportedFeature {
        /// Feature that is not supported
        feature: String,
    },

    /// Backend is in wrong state for operation
    #[error("Invalid state: expected {expected}, current {current}")]
    InvalidState {
        /// Expected state
        expected: String,
        /// Current state
        current: String,
    },

    /// Other error
    #[error("Other error: {message}")]
    Other {
        /// Error message
        message: String,
    },
}

/// Bus error types.
///
/// These represent various types of errors that can occur on the CAN bus
/// at the physical and data link layers.
///
/// # Examples
///
/// ```
/// use canlink_hal::BusErrorKind;
///
/// let error = BusErrorKind::BitError;
/// assert_eq!(format!("{:?}", error), "BitError");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BusErrorKind {
    /// Bit error - transmitted bit differs from monitored bit
    BitError,

    /// Stuff error - more than 5 consecutive bits of same value
    StuffError,

    /// CRC error - calculated CRC differs from received CRC
    CrcError,

    /// ACK error - no acknowledgment received
    AckError,

    /// Form error - fixed-form bit field contains illegal value
    FormError,

    /// Bus-off - error counter exceeded threshold
    BusOff,

    /// Error passive - error counter in passive range
    ErrorPassive,

    /// Error warning - error counter in warning range
    ErrorWarning,
}

impl BusErrorKind {
    /// Get a human-readable description of the error.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BusErrorKind;
    ///
    /// let error = BusErrorKind::BitError;
    /// assert_eq!(error.description(), "Bit error");
    /// ```
    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            Self::BitError => "Bit error",
            Self::StuffError => "Stuff error",
            Self::CrcError => "CRC error",
            Self::AckError => "ACK error",
            Self::FormError => "Form error",
            Self::BusOff => "Bus-off",
            Self::ErrorPassive => "Error passive",
            Self::ErrorWarning => "Error warning",
        }
    }
}

/// Result type alias for CAN operations.
///
/// This is a convenience alias for `Result<T, CanError>`.
///
/// # Examples
///
/// ```
/// use canlink_hal::{CanResult, CanMessage};
///
/// fn send_message(msg: &CanMessage) -> CanResult<()> {
///     // Implementation
///     Ok(())
/// }
/// ```
pub type CanResult<T> = Result<T, CanError>;

// ============================================================================
// Filter Errors (FR-005 to FR-009)
// ============================================================================

/// Filter-related errors
///
/// These errors occur during message filter operations.
#[derive(Error, Debug)]
pub enum FilterError {
    /// Invalid filter configuration
    #[error("Invalid filter configuration: {reason}")]
    InvalidConfig {
        /// Reason for invalid configuration
        reason: String,
    },

    /// Filter ID out of range
    #[error("Filter ID {id:#X} out of range (max: {max:#X})")]
    IdOutOfRange {
        /// The invalid ID
        id: u32,
        /// Maximum allowed ID
        max: u32,
    },

    /// Invalid ID range (start > end)
    #[error("Invalid ID range: start {start:#X} > end {end:#X}")]
    InvalidRange {
        /// Start ID
        start: u32,
        /// End ID
        end: u32,
    },

    /// Hardware filter limit exceeded
    #[error("Hardware filter limit exceeded: max {max}, requested {requested}")]
    HardwareFilterLimitExceeded {
        /// Maximum hardware filters supported
        max: usize,
        /// Number of filters requested
        requested: usize,
    },

    /// Filter not found
    #[error("Filter not found at index {index}")]
    FilterNotFound {
        /// Index that was not found
        index: usize,
    },
}

/// Result type alias for filter operations
pub type FilterResult<T> = Result<T, FilterError>;

// ============================================================================
// Queue Errors (FR-011, FR-017)
// ============================================================================

/// Queue-related errors
///
/// These errors occur during message queue operations.
#[derive(Error, Debug)]
pub enum QueueError {
    /// Queue is full (Block policy timeout)
    #[error("Queue full: capacity {capacity}")]
    QueueFull {
        /// Queue capacity
        capacity: usize,
    },

    /// Message was dropped due to overflow policy
    #[error("Message dropped (ID: {id:#X}): {reason}")]
    MessageDropped {
        /// ID of the dropped message
        id: u32,
        /// Reason for dropping
        reason: String,
    },

    /// Invalid queue capacity
    #[error("Invalid queue capacity: {capacity} (min: 1)")]
    InvalidCapacity {
        /// The invalid capacity value
        capacity: usize,
    },

    /// Queue operation timeout
    #[error("Queue operation timed out after {timeout_ms}ms")]
    Timeout {
        /// Timeout in milliseconds
        timeout_ms: u64,
    },
}

/// Result type alias for queue operations
pub type QueueResult<T> = Result<T, QueueError>;

// ============================================================================
// Monitor Errors (FR-010)
// ============================================================================

/// Monitor-related errors
///
/// These errors occur during connection monitoring operations.
#[derive(Error, Debug)]
pub enum MonitorError {
    /// Reconnection failed
    #[error("Reconnect failed: {reason}")]
    ReconnectFailed {
        /// Reason for failure
        reason: String,
    },

    /// Monitor not started
    #[error("Monitor not started")]
    NotStarted,

    /// Monitor already running
    #[error("Monitor already running")]
    AlreadyRunning,

    /// Backend error during monitoring
    #[error("Backend error: {0}")]
    BackendError(#[from] CanError),

    /// Invalid monitor configuration
    #[error("Invalid monitor configuration: {reason}")]
    InvalidConfig {
        /// Reason for invalid configuration
        reason: String,
    },

    /// Heartbeat timeout
    #[error("Heartbeat timeout after {timeout_ms}ms")]
    HeartbeatTimeout {
        /// Timeout in milliseconds
        timeout_ms: u64,
    },
}

/// Result type alias for monitor operations
pub type MonitorResult<T> = Result<T, MonitorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CanError::InvalidId {
            value: 0x800,
            max: 0x7FF,
        };
        let msg = format!("{err}");
        assert!(msg.contains("0x800"));
        assert!(msg.contains("0x7FF"));
    }

    #[test]
    fn test_bus_error_kind() {
        let error = BusErrorKind::BitError;
        assert_eq!(error.description(), "Bit error");
    }

    #[test]
    fn test_error_codes() {
        let err = CanError::BackendNotFound {
            name: "test".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("[1001]"));
    }
}
