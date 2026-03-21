//! Monitor configuration (FR-010)
//!
//! Provides configuration structures for loading monitor settings from TOML.

use std::time::Duration;

use serde::Deserialize;

use super::{ConnectionMonitor, ReconnectConfig};

/// Monitor configuration from TOML
///
/// # Example TOML
///
/// ```toml
/// [monitor]
/// heartbeat_interval_ms = 1000
///
/// # Optional: enable auto-reconnect
/// [monitor.reconnect]
/// max_retries = 5
/// retry_interval_ms = 2000
/// backoff_multiplier = 1.5
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct MonitorConfig {
    /// Heartbeat interval in milliseconds
    #[serde(default = "default_heartbeat_ms")]
    pub heartbeat_interval_ms: u64,

    /// Reconnect configuration (optional)
    pub reconnect: Option<ReconnectConfigFile>,
}

fn default_heartbeat_ms() -> u64 {
    1000
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: default_heartbeat_ms(),
            reconnect: None,
        }
    }
}

/// Reconnect configuration from TOML
#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectConfigFile {
    /// Maximum retries (0 = unlimited)
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry interval in milliseconds
    #[serde(default = "default_retry_interval_ms")]
    pub retry_interval_ms: u64,

    /// Backoff multiplier
    #[serde(default = "default_backoff")]
    pub backoff_multiplier: f32,
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_interval_ms() -> u64 {
    1000
}

fn default_backoff() -> f32 {
    2.0
}

impl Default for ReconnectConfigFile {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_interval_ms: default_retry_interval_ms(),
            backoff_multiplier: default_backoff(),
        }
    }
}

impl From<ReconnectConfigFile> for ReconnectConfig {
    fn from(config: ReconnectConfigFile) -> Self {
        ReconnectConfig {
            max_retries: config.max_retries,
            retry_interval: Duration::from_millis(config.retry_interval_ms),
            backoff_multiplier: config.backoff_multiplier,
        }
    }
}

impl MonitorConfig {
    /// Load configuration from TOML string
    ///
    /// # Errors
    ///
    /// Returns `toml::de::Error` if the TOML string is invalid or cannot be
    /// deserialized into a `MonitorConfig`.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Create a `ConnectionMonitor` from this configuration
    #[must_use]
    pub fn into_monitor(self) -> ConnectionMonitor {
        let heartbeat = Duration::from_millis(self.heartbeat_interval_ms);

        if let Some(reconnect) = self.reconnect {
            ConnectionMonitor::with_reconnect(heartbeat, reconnect.into())
        } else {
            ConnectionMonitor::new(heartbeat)
        }
    }
}

impl ConnectionMonitor {
    /// Create a `ConnectionMonitor` from configuration
    #[must_use]
    pub fn from_config(config: &MonitorConfig) -> Self {
        config.clone().into_monitor()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = MonitorConfig::default();
        assert_eq!(config.heartbeat_interval_ms, 1000);
        assert!(config.reconnect.is_none());
    }

    #[test]
    fn test_parse_basic() {
        let toml = r"
            heartbeat_interval_ms = 500
        ";

        let config: MonitorConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.heartbeat_interval_ms, 500);
        assert!(config.reconnect.is_none());
    }

    #[test]
    fn test_parse_with_reconnect() {
        let toml = r"
            heartbeat_interval_ms = 500

            [reconnect]
            max_retries = 5
            retry_interval_ms = 2000
            backoff_multiplier = 1.5
        ";

        let config: MonitorConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.heartbeat_interval_ms, 500);

        let reconnect = config.reconnect.unwrap();
        assert_eq!(reconnect.max_retries, 5);
        assert_eq!(reconnect.retry_interval_ms, 2000);
        assert!((reconnect.backoff_multiplier - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_into_monitor() {
        let config = MonitorConfig {
            heartbeat_interval_ms: 500,
            reconnect: Some(ReconnectConfigFile::default()),
        };

        let monitor = config.into_monitor();
        assert_eq!(monitor.heartbeat_interval(), Duration::from_millis(500));
        assert!(monitor.auto_reconnect_enabled());
    }

    #[test]
    fn test_into_monitor_no_reconnect() {
        let config = MonitorConfig::default();
        let monitor = config.into_monitor();
        assert!(!monitor.auto_reconnect_enabled());
    }
}
