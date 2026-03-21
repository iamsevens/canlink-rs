//! ISO-TP error types.

use thiserror::Error;

/// ISO-TP error types.
#[derive(Debug, Error)]
pub enum IsoTpError {
    /// Invalid frame format
    #[error("Invalid frame format: {reason}")]
    InvalidFrame {
        /// Reason for the error
        reason: String,
    },

    /// Invalid PCI type
    #[error("Invalid PCI type: 0x{pci:02X}")]
    InvalidPci {
        /// The invalid PCI byte
        pci: u8,
    },

    /// Sequence number mismatch
    #[error("Sequence number mismatch: expected {expected}, got {actual}")]
    SequenceMismatch {
        /// Expected sequence number
        expected: u8,
        /// Actual sequence number received
        actual: u8,
    },

    /// Receive timeout
    #[error("Receive timeout after {timeout_ms}ms")]
    RxTimeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Timeout waiting for Flow Control
    #[error("Timeout waiting for Flow Control after {timeout_ms}ms")]
    FcTimeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },

    /// Too many FC(Wait) responses
    #[error("Too many FC(Wait) responses: {count} exceeds max {max}")]
    TooManyWaits {
        /// Number of Wait responses received
        count: u8,
        /// Maximum allowed Wait responses
        max: u8,
    },

    /// Buffer overflow
    #[error("Buffer overflow: received {received} bytes, max {max}")]
    BufferOverflow {
        /// Bytes received
        received: usize,
        /// Maximum buffer size
        max: usize,
    },

    /// Remote reported overflow
    #[error("Remote reported overflow")]
    RemoteOverflow,

    /// Data too large
    #[error("Data too large: {size} bytes, max {max}")]
    DataTooLarge {
        /// Size of data attempted to send
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Data is empty
    #[error("Data is empty")]
    EmptyData,

    /// Transfer aborted
    #[error("Transfer aborted")]
    Aborted,

    /// Invalid configuration
    #[error("Invalid configuration: {reason}")]
    InvalidConfig {
        /// Reason for the error
        reason: String,
    },

    /// Backend error
    #[error("Backend error: {0}")]
    BackendError(#[from] crate::CanError),

    /// Backend disconnected
    #[error("Backend disconnected")]
    BackendDisconnected,

    /// Buffer allocation failed
    #[error("Buffer allocation failed: requested {size} bytes")]
    BufferAllocationFailed {
        /// Requested buffer size
        size: usize,
    },

    /// Channel busy
    #[error("Channel busy: {state}")]
    ChannelBusy {
        /// Current state description
        state: String,
    },

    /// Unexpected frame type
    #[error("Unexpected frame type: expected {expected}, got {actual}")]
    UnexpectedFrame {
        /// Expected frame type
        expected: String,
        /// Actual frame type received
        actual: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = IsoTpError::InvalidFrame {
            reason: "too short".to_string(),
        };
        assert_eq!(err.to_string(), "Invalid frame format: too short");

        let err = IsoTpError::InvalidPci { pci: 0x40 };
        assert_eq!(err.to_string(), "Invalid PCI type: 0x40");

        let err = IsoTpError::SequenceMismatch {
            expected: 5,
            actual: 7,
        };
        assert_eq!(
            err.to_string(),
            "Sequence number mismatch: expected 5, got 7"
        );

        let err = IsoTpError::RxTimeout { timeout_ms: 1000 };
        assert_eq!(err.to_string(), "Receive timeout after 1000ms");

        let err = IsoTpError::TooManyWaits { count: 11, max: 10 };
        assert_eq!(
            err.to_string(),
            "Too many FC(Wait) responses: 11 exceeds max 10"
        );
    }

    #[test]
    fn test_error_variants() {
        // Ensure all variants can be constructed
        let _ = IsoTpError::InvalidFrame {
            reason: "test".to_string(),
        };
        let _ = IsoTpError::InvalidPci { pci: 0 };
        let _ = IsoTpError::SequenceMismatch {
            expected: 0,
            actual: 1,
        };
        let _ = IsoTpError::RxTimeout { timeout_ms: 100 };
        let _ = IsoTpError::FcTimeout { timeout_ms: 100 };
        let _ = IsoTpError::TooManyWaits { count: 1, max: 10 };
        let _ = IsoTpError::BufferOverflow {
            received: 5000,
            max: 4095,
        };
        let _ = IsoTpError::RemoteOverflow;
        let _ = IsoTpError::DataTooLarge {
            size: 5000,
            max: 4095,
        };
        let _ = IsoTpError::EmptyData;
        let _ = IsoTpError::Aborted;
        let _ = IsoTpError::InvalidConfig {
            reason: "test".to_string(),
        };
        let _ = IsoTpError::BackendDisconnected;
        let _ = IsoTpError::BufferAllocationFailed { size: 1000 };
        let _ = IsoTpError::ChannelBusy {
            state: "receiving".to_string(),
        };
        let _ = IsoTpError::UnexpectedFrame {
            expected: "CF".to_string(),
            actual: "SF".to_string(),
        };
    }
}
