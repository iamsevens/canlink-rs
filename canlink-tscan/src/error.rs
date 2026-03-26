//! Error conversion for `LibTSCAN` backend.
//!
//! This module converts `LibTSCAN` error codes (u32) to the unified `CanError` type.

use canlink_hal::CanError;
use canlink_tscan_sys::tscan_get_error_description;
use std::ffi::CStr;
use std::os::raw::c_char;

/// Convert `LibTSCAN` error code to CanError.
///
/// # Arguments
/// * `code` - `LibTSCAN` error code (0 = success, non-zero = error)
///
/// # Returns
/// * `Ok(())` if code is 0
/// * `Err(CanError)` with appropriate error type if code is non-zero
///
/// # Examples
///
/// ```ignore
/// let result = check_error(0);
/// assert!(result.is_ok());
///
/// let result = check_error(1);
/// assert!(result.is_err());
/// ```
pub fn check_error(code: u32) -> Result<(), CanError> {
    if code == 0 {
        Ok(())
    } else {
        Err(convert_error(code))
    }
}

/// Convert `LibTSCAN` error code to CanError with detailed description.
///
/// This function queries `LibTSCAN` for the error description and maps
/// the error code to the appropriate CanError variant.
///
/// # Arguments
/// * `code` - `LibTSCAN` error code
///
/// # Returns
/// Appropriate `CanError` variant based on the error code
fn convert_error(code: u32) -> CanError {
    // Get error description from LibTSCAN
    let description = get_error_description(code);

    // Map common error codes to specific CanError variants
    match code {
        // Device/Connection errors (1xxx range in LibTSCAN)
        1001 => CanError::DeviceNotFound {
            device: description,
        },
        1002 => CanError::InitializationFailed {
            reason: description,
        },
        1003 => CanError::ChannelNotFound { channel: 0, max: 0 },
        1004 => CanError::ChannelNotOpen { channel: 0 },

        // Configuration errors (2xxx range)
        2001..=2999 => CanError::ConfigError {
            reason: description,
        },

        // Communication errors (3xxx range)
        3001 => CanError::SendFailed {
            reason: description,
        },
        3002 => CanError::ReceiveFailed {
            reason: description,
        },

        // Timeout errors (4xxx range)
        4001 => CanError::Timeout { timeout_ms: 0 },

        // Permission/Resource errors (5xxx range)
        5001 => CanError::PermissionDenied {
            operation: description.clone(),
        },
        5002 => CanError::InsufficientResources {
            resource: description,
        },

        // Invalid parameter errors (6xxx range)
        6001..=6999 => CanError::InvalidParameter {
            parameter: "unknown".to_string(),
            reason: description,
        },

        // All other errors
        _ => CanError::Other {
            message: format!("LibTSCAN error {}: {}", code, description),
        },
    }
}

/// Get error description from `LibTSCAN`.
///
/// # Arguments
/// * `code` - Error code
///
/// # Returns
/// Human-readable error description string
fn get_error_description(code: u32) -> String {
    unsafe {
        let mut desc_ptr: *const c_char = std::ptr::null();
        let result = tscan_get_error_description(code, &mut desc_ptr);

        if result == 0 && !desc_ptr.is_null() {
            // Successfully got description from LibTSCAN
            CStr::from_ptr(desc_ptr).to_string_lossy().into_owned()
        } else {
            // Failed to get description, return generic message
            format!("Unknown error code: {}", code)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_error_success() {
        let result = check_error(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_error_failure() {
        let result = check_error(1);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_error_device_not_found() {
        let err = convert_error(1001);
        assert!(matches!(err, CanError::DeviceNotFound { .. }));
    }

    #[test]
    fn test_convert_error_config() {
        let err = convert_error(2001);
        assert!(matches!(err, CanError::ConfigError { .. }));
    }

    #[test]
    fn test_convert_error_send_failed() {
        let err = convert_error(3001);
        assert!(matches!(err, CanError::SendFailed { .. }));
    }

    #[test]
    fn test_convert_error_timeout() {
        let err = convert_error(4001);
        assert!(matches!(err, CanError::Timeout { .. }));
    }

    #[test]
    fn test_convert_error_other() {
        let err = convert_error(9999);
        assert!(matches!(err, CanError::Other { .. }));
    }
}
