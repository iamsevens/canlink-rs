//! Error injection for testing.
//!
//! This module provides error injection capabilities for the Mock backend,
//! allowing tests to simulate various error conditions.

use canlink_hal::{BusErrorKind, CanError};
use serde::{Deserialize, Serialize};

/// Error injection configuration.
///
/// Defines which operations should fail and what errors they should return.
///
/// # Examples
///
/// ```
/// use canlink_mock::ErrorInjector;
/// use canlink_hal::{CanError, BusErrorKind};
///
/// let mut injector = ErrorInjector::new();
///
/// // Inject a send failure
/// injector.inject_send_error(CanError::SendFailed {
///     reason: "Bus-Off state".to_string(),
/// });
///
/// // Check if send should fail
/// if let Some(error) = injector.should_fail_send() {
///     println!("Send will fail: {:?}", error);
/// }
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorInjector {
    /// Error to inject on `send_message` calls
    send_error: Option<ErrorConfig>,

    /// Error to inject on `receive_message` calls
    receive_error: Option<ErrorConfig>,

    /// Error to inject on initialize calls
    init_error: Option<ErrorConfig>,

    /// Error to inject on `open_channel` calls
    open_channel_error: Option<ErrorConfig>,

    /// Error to inject on `close_channel` calls
    close_channel_error: Option<ErrorConfig>,

    /// Counter for tracking injection attempts
    injection_count: usize,
}

/// Error configuration for injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorConfig {
    /// The error to inject
    pub error: ErrorType,

    /// How many times to inject this error (0 = infinite)
    pub count: usize,

    /// How many calls to skip before injecting (0 = inject immediately)
    pub skip: usize,

    /// Current skip counter
    #[serde(skip)]
    current_skip: usize,

    /// Current injection counter
    #[serde(skip)]
    current_count: usize,
}

/// Types of errors that can be injected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorType {
    /// Send operation failed
    SendFailed {
        /// Reason for the failure
        reason: String,
    },

    /// Receive operation failed
    ReceiveFailed {
        /// Reason for the failure
        reason: String,
    },

    /// Initialization failed
    InitializationFailed {
        /// Reason for the failure
        reason: String,
    },

    /// Channel not found
    ChannelNotFound {
        /// Channel number that was requested
        channel: u8,
        /// Maximum valid channel number
        max: u8,
    },

    /// Channel already open
    ChannelAlreadyOpen {
        /// Channel number that is already open
        channel: u8,
    },

    /// Channel not open
    ChannelNotOpen {
        /// Channel number that is not open
        channel: u8,
    },

    /// Bus error occurred (stores error kind as string)
    BusError {
        /// Kind of bus error (as string for serialization)
        kind: String,
    },

    /// Timeout occurred
    Timeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Invalid state
    InvalidState {
        /// Expected state
        expected: String,
        /// Current state
        current: String,
    },

    /// Unsupported feature
    UnsupportedFeature {
        /// Name of the unsupported feature
        feature: String,
    },
}

impl ErrorInjector {
    /// Create a new error injector with no errors configured.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::ErrorInjector;
    ///
    /// let injector = ErrorInjector::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Inject an error on `send_message` calls.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to inject
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::ErrorInjector;
    /// use canlink_hal::CanError;
    ///
    /// let mut injector = ErrorInjector::new();
    /// injector.inject_send_error(CanError::SendFailed {
    ///     reason: "Bus-Off".to_string(),
    /// });
    /// ```
    pub fn inject_send_error(&mut self, error: CanError) {
        self.send_error = Some(ErrorConfig::once(error.into()));
    }

    /// Inject an error on `send_message` calls with count and skip.
    ///
    /// # Arguments
    ///
    /// * `error` - The error to inject
    /// * `count` - Number of times to inject (0 = infinite)
    /// * `skip` - Number of calls to skip before injecting
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::ErrorInjector;
    /// use canlink_hal::CanError;
    ///
    /// let mut injector = ErrorInjector::new();
    /// // Fail the 3rd and 4th send attempts
    /// injector.inject_send_error_with_config(
    ///     CanError::SendFailed { reason: "Test".to_string() },
    ///     2,  // inject 2 times
    ///     2,  // skip first 2 calls
    /// );
    /// ```
    pub fn inject_send_error_with_config(&mut self, error: CanError, count: usize, skip: usize) {
        self.send_error = Some(ErrorConfig::new(error.into(), count, skip));
    }

    /// Inject an error on `receive_message` calls.
    pub fn inject_receive_error(&mut self, error: CanError) {
        self.receive_error = Some(ErrorConfig::once(error.into()));
    }

    /// Inject an error on `receive_message` calls with count and skip.
    pub fn inject_receive_error_with_config(&mut self, error: CanError, count: usize, skip: usize) {
        self.receive_error = Some(ErrorConfig::new(error.into(), count, skip));
    }

    /// Inject an error on initialize calls.
    pub fn inject_init_error(&mut self, error: CanError) {
        self.init_error = Some(ErrorConfig::once(error.into()));
    }

    /// Inject an error on `open_channel` calls.
    pub fn inject_open_channel_error(&mut self, error: CanError) {
        self.open_channel_error = Some(ErrorConfig::once(error.into()));
    }

    /// Inject an error on `close_channel` calls.
    pub fn inject_close_channel_error(&mut self, error: CanError) {
        self.close_channel_error = Some(ErrorConfig::once(error.into()));
    }

    /// Check if `send_message` should fail and return the error.
    ///
    /// # Returns
    ///
    /// `Some(error)` if the operation should fail, `None` otherwise.
    pub fn should_fail_send(&mut self) -> Option<CanError> {
        Self::check_error(&mut self.send_error, &mut self.injection_count)
    }

    /// Check if `receive_message` should fail and return the error.
    pub fn should_fail_receive(&mut self) -> Option<CanError> {
        Self::check_error(&mut self.receive_error, &mut self.injection_count)
    }

    /// Check if initialize should fail and return the error.
    pub fn should_fail_init(&mut self) -> Option<CanError> {
        Self::check_error(&mut self.init_error, &mut self.injection_count)
    }

    /// Check if `open_channel` should fail and return the error.
    pub fn should_fail_open_channel(&mut self) -> Option<CanError> {
        Self::check_error(&mut self.open_channel_error, &mut self.injection_count)
    }

    /// Check if `close_channel` should fail and return the error.
    pub fn should_fail_close_channel(&mut self) -> Option<CanError> {
        Self::check_error(&mut self.close_channel_error, &mut self.injection_count)
    }

    /// Clear all injected errors.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_mock::ErrorInjector;
    /// use canlink_hal::CanError;
    ///
    /// let mut injector = ErrorInjector::new();
    /// injector.inject_send_error(CanError::SendFailed {
    ///     reason: "Test".to_string(),
    /// });
    /// injector.clear();
    /// assert!(injector.should_fail_send().is_none());
    /// ```
    pub fn clear(&mut self) {
        self.send_error = None;
        self.receive_error = None;
        self.init_error = None;
        self.open_channel_error = None;
        self.close_channel_error = None;
        self.injection_count = 0;
    }

    /// Get the total number of errors injected.
    #[must_use]
    pub fn injection_count(&self) -> usize {
        self.injection_count
    }

    /// Check if an error should be injected based on configuration.
    fn check_error(
        config: &mut Option<ErrorConfig>,
        injection_count: &mut usize,
    ) -> Option<CanError> {
        if let Some(cfg) = config {
            // Check if we should skip this call
            if cfg.current_skip < cfg.skip {
                cfg.current_skip += 1;
                return None;
            }

            // Check if we've reached the injection limit
            if cfg.count > 0 && cfg.current_count >= cfg.count {
                return None;
            }

            // Inject the error
            cfg.current_count += 1;
            *injection_count += 1;

            Some(cfg.error.clone().into())
        } else {
            None
        }
    }
}

impl ErrorConfig {
    /// Create a new error configuration.
    fn new(error: ErrorType, count: usize, skip: usize) -> Self {
        Self {
            error,
            count,
            skip,
            current_skip: 0,
            current_count: 0,
        }
    }

    /// Create a configuration that injects the error once.
    fn once(error: ErrorType) -> Self {
        Self::new(error, 1, 0)
    }
}

impl From<CanError> for ErrorType {
    fn from(error: CanError) -> Self {
        match error {
            CanError::SendFailed { reason } => ErrorType::SendFailed { reason },
            CanError::ReceiveFailed { reason } => ErrorType::ReceiveFailed { reason },
            CanError::InitializationFailed { reason } => ErrorType::InitializationFailed { reason },
            CanError::ChannelNotFound { channel, max } => {
                ErrorType::ChannelNotFound { channel, max }
            }
            CanError::ChannelAlreadyOpen { channel } => ErrorType::ChannelAlreadyOpen { channel },
            CanError::ChannelNotOpen { channel } => ErrorType::ChannelNotOpen { channel },
            CanError::BusError { kind } => ErrorType::BusError {
                kind: kind.description().to_string(),
            },
            CanError::Timeout { timeout_ms } => ErrorType::Timeout { timeout_ms },
            CanError::InvalidState { expected, current } => {
                ErrorType::InvalidState { expected, current }
            }
            CanError::UnsupportedFeature { feature } => ErrorType::UnsupportedFeature { feature },
            _ => ErrorType::SendFailed {
                reason: "Unknown error".to_string(),
            },
        }
    }
}

impl From<ErrorType> for CanError {
    fn from(error: ErrorType) -> Self {
        match error {
            ErrorType::SendFailed { reason } => CanError::SendFailed { reason },
            ErrorType::ReceiveFailed { reason } => CanError::ReceiveFailed { reason },
            ErrorType::InitializationFailed { reason } => CanError::InitializationFailed { reason },
            ErrorType::ChannelNotFound { channel, max } => {
                CanError::ChannelNotFound { channel, max }
            }
            ErrorType::ChannelAlreadyOpen { channel } => CanError::ChannelAlreadyOpen { channel },
            ErrorType::ChannelNotOpen { channel } => CanError::ChannelNotOpen { channel },
            ErrorType::BusError { kind } => {
                // Convert string back to BusErrorKind (default to BitError if unknown)
                let bus_kind = match kind.as_str() {
                    "Stuff error" => BusErrorKind::StuffError,
                    "CRC error" => BusErrorKind::CrcError,
                    "Form error" => BusErrorKind::FormError,
                    "Acknowledgment error" => BusErrorKind::AckError,
                    "Bus-Off" => BusErrorKind::BusOff,
                    "Error passive" => BusErrorKind::ErrorPassive,
                    "Error warning" => BusErrorKind::ErrorWarning,
                    _ => BusErrorKind::BitError, // Default for "Bit error" and unknown
                };
                CanError::BusError { kind: bus_kind }
            }
            ErrorType::Timeout { timeout_ms } => CanError::Timeout { timeout_ms },
            ErrorType::InvalidState { expected, current } => {
                CanError::InvalidState { expected, current }
            }
            ErrorType::UnsupportedFeature { feature } => CanError::UnsupportedFeature { feature },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_injector() {
        let injector = ErrorInjector::new();
        assert_eq!(injector.injection_count(), 0);
    }

    #[test]
    fn test_inject_send_error_once() {
        let mut injector = ErrorInjector::new();
        injector.inject_send_error(CanError::SendFailed {
            reason: "Test".to_string(),
        });

        // First call should fail
        assert!(injector.should_fail_send().is_some());
        assert_eq!(injector.injection_count(), 1);

        // Second call should succeed
        assert!(injector.should_fail_send().is_none());
        assert_eq!(injector.injection_count(), 1);
    }

    #[test]
    fn test_inject_send_error_with_skip() {
        let mut injector = ErrorInjector::new();
        injector.inject_send_error_with_config(
            CanError::SendFailed {
                reason: "Test".to_string(),
            },
            1,
            2,
        );

        // First two calls should succeed (skip)
        assert!(injector.should_fail_send().is_none());
        assert!(injector.should_fail_send().is_none());

        // Third call should fail
        assert!(injector.should_fail_send().is_some());
        assert_eq!(injector.injection_count(), 1);

        // Fourth call should succeed
        assert!(injector.should_fail_send().is_none());
    }

    #[test]
    fn test_inject_send_error_multiple_times() {
        let mut injector = ErrorInjector::new();
        injector.inject_send_error_with_config(
            CanError::SendFailed {
                reason: "Test".to_string(),
            },
            3,
            0,
        );

        // First three calls should fail
        assert!(injector.should_fail_send().is_some());
        assert!(injector.should_fail_send().is_some());
        assert!(injector.should_fail_send().is_some());
        assert_eq!(injector.injection_count(), 3);

        // Fourth call should succeed
        assert!(injector.should_fail_send().is_none());
    }

    #[test]
    fn test_inject_multiple_error_types() {
        let mut injector = ErrorInjector::new();
        injector.inject_send_error(CanError::SendFailed {
            reason: "Send test".to_string(),
        });
        injector.inject_receive_error(CanError::ReceiveFailed {
            reason: "Receive test".to_string(),
        });

        assert!(injector.should_fail_send().is_some());
        assert!(injector.should_fail_receive().is_some());
        assert_eq!(injector.injection_count(), 2);
    }

    #[test]
    fn test_clear_errors() {
        let mut injector = ErrorInjector::new();
        injector.inject_send_error(CanError::SendFailed {
            reason: "Test".to_string(),
        });

        assert!(injector.should_fail_send().is_some());
        assert_eq!(injector.injection_count(), 1);

        injector.clear();
        assert!(injector.should_fail_send().is_none());
        assert_eq!(injector.injection_count(), 0);
    }

    #[test]
    fn test_infinite_injection() {
        let mut injector = ErrorInjector::new();
        injector.inject_send_error_with_config(
            CanError::SendFailed {
                reason: "Test".to_string(),
            },
            0, // infinite
            0,
        );

        // Should fail indefinitely
        for _ in 0..100 {
            assert!(injector.should_fail_send().is_some());
        }
        assert_eq!(injector.injection_count(), 100);
    }
}
