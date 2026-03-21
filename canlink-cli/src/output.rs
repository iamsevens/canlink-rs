//! Output formatting for CLI.
//!
//! Supports both human-readable and JSON output formats.

use serde::Serialize;
use std::io::{self};

/// Output formatter that supports multiple formats.
pub struct OutputFormatter {
    /// Whether output should be encoded as JSON.
    json: bool,
}

impl OutputFormatter {
    /// Create a new output formatter.
    pub fn new(json: bool) -> Self {
        Self { json }
    }

    /// Check if JSON output is enabled.
    pub fn is_json(&self) -> bool {
        self.json
    }

    /// Print a serializable value.
    pub fn print<T: Serialize>(&self, value: &T) -> io::Result<()> {
        if self.json {
            self.print_json(value)
        } else {
            // For non-JSON, we expect the caller to implement Display
            // This is a fallback that uses JSON for structured data
            self.print_json(value)
        }
    }

    /// Print as JSON.
    fn print_json<T: Serialize>(&self, value: &T) -> io::Result<()> {
        let json = serde_json::to_string_pretty(value).map_err(io::Error::other)?;
        println!("{}", json);
        Ok(())
    }

    /// Print a simple message (not JSON).
    pub fn print_message(&self, message: &str) -> io::Result<()> {
        if self.json {
            let msg = serde_json::json!({ "message": message });
            self.print_json(&msg)
        } else {
            println!("{}", message);
            Ok(())
        }
    }

    /// Print an error message to stderr.
    pub fn print_error(&self, error: &str) -> io::Result<()> {
        if self.json {
            let err = serde_json::json!({ "error": error });
            let json = serde_json::to_string_pretty(&err).map_err(io::Error::other)?;
            eprintln!("{}", json);
        } else {
            eprintln!("Error: {}", error);
        }
        Ok(())
    }

    /// Print a success message.
    pub fn print_success(&self, message: &str) -> io::Result<()> {
        if self.json {
            let msg = serde_json::json!({ "status": "success", "message": message });
            self.print_json(&msg)
        } else {
            println!("✓ {}", message);
            Ok(())
        }
    }

    /// Print an info message.
    pub fn print_info(&self, message: &str) -> io::Result<()> {
        if self.json {
            let msg = serde_json::json!({ "status": "info", "message": message });
            self.print_json(&msg)
        } else {
            println!("ℹ {}", message);
            Ok(())
        }
    }
}

/// Helper for printing backend list.
#[derive(Serialize)]
pub struct BackendListOutput {
    /// Registered backend names.
    pub backends: Vec<String>,
}

/// Helper for printing backend info.
#[derive(Serialize)]
pub struct BackendInfoOutput {
    /// Backend name.
    pub name: String,
    /// Backend semantic version string.
    pub version: String,
    /// Number of CAN channels.
    pub channel_count: u32,
    /// Whether backend supports CAN FD.
    pub supports_canfd: bool,
    /// Maximum supported bitrate in bps.
    pub max_bitrate: u32,
    /// Supported bitrate list in bps.
    pub supported_bitrates: Vec<u32>,
    /// Hardware filter capacity.
    pub filter_count: u32,
}

/// Helper for printing CAN message.
#[derive(Serialize)]
pub struct MessageOutput {
    /// CAN identifier text.
    pub id: String,
    /// Payload bytes text.
    pub data: String,
    /// Optional timestamp text.
    pub timestamp: Option<String>,
    /// Message flags (e.g. FD/BRS/ESI).
    pub flags: Vec<String>,
}

impl MessageOutput {
    /// Format for human-readable output.
    pub fn format_human(&self) -> String {
        let mut parts = vec![format!("ID: {}", self.id), format!("Data: {}", self.data)];

        if let Some(ref ts) = self.timestamp {
            parts.push(format!("Timestamp: {}", ts));
        }

        if !self.flags.is_empty() {
            parts.push(format!("Flags: {}", self.flags.join(", ")));
        }

        parts.join(" | ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_formatter_creation() {
        let formatter = OutputFormatter::new(false);
        assert!(!formatter.json);

        let formatter = OutputFormatter::new(true);
        assert!(formatter.json);
    }

    #[test]
    fn test_backend_list_output() {
        let output = BackendListOutput {
            backends: vec!["mock".to_string(), "tsmaster".to_string()],
        };
        let json = serde_json::to_string(&output).unwrap();
        assert!(json.contains("mock"));
        assert!(json.contains("tsmaster"));
    }

    #[test]
    fn test_message_output_format() {
        let msg = MessageOutput {
            id: "0x123".to_string(),
            data: "01 02 03".to_string(),
            timestamp: Some("1234567890".to_string()),
            flags: vec!["FD".to_string(), "BRS".to_string()],
        };

        let formatted = msg.format_human();
        assert!(formatted.contains("ID: 0x123"));
        assert!(formatted.contains("Data: 01 02 03"));
        assert!(formatted.contains("Timestamp: 1234567890"));
        assert!(formatted.contains("Flags: FD, BRS"));
    }
}
