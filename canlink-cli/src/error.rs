//! CLI error handling.
//!
//! Defines error types and exit codes for the CLI.

use thiserror::Error;

/// CLI error type.
#[derive(Error, Debug)]
pub enum CliError {
    /// Backend not found.
    #[error("Backend not found: {0}")]
    BackendNotFound(String),

    /// Backend error.
    #[error("Backend error: {0}")]
    BackendError(#[from] canlink_hal::CanError),


    /// Configuration error.
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Invalid argument.
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// I/O error.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    /// Parse error.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Timeout error.
    #[error("Operation timed out")]
    Timeout,

    /// No messages received.
    #[error("No messages received")]
    NoMessages,
}

impl CliError {
    /// Get the exit code for this error.
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::BackendNotFound(_) => 2,
            CliError::BackendError(_) => 3,
            CliError::ConfigError(_) => 4,
            CliError::InvalidArgument(_) => 5,
            CliError::IoError(_) => 6,
            CliError::ParseError(_) => 7,
            CliError::Timeout => 8,
            CliError::NoMessages => 9,
        }
    }
}

/// Result type for CLI operations.
pub type CliResult<T> = Result<T, CliError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_exit_codes() {
        assert_eq!(CliError::BackendNotFound("test".to_string()).exit_code(), 2);
        assert_eq!(CliError::ConfigError("test".to_string()).exit_code(), 4);
        assert_eq!(CliError::InvalidArgument("test".to_string()).exit_code(), 5);
        assert_eq!(CliError::Timeout.exit_code(), 8);
        assert_eq!(CliError::NoMessages.exit_code(), 9);
    }

    #[test]
    fn test_error_display() {
        let err = CliError::BackendNotFound("tscan".to_string());
        assert_eq!(err.to_string(), "Backend not found: tscan");

        let err = CliError::InvalidArgument("bad value".to_string());
        assert_eq!(err.to_string(), "Invalid argument: bad value");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let cli_err: CliError = io_err.into();
        assert!(matches!(cli_err, CliError::IoError(_)));
    }
}
