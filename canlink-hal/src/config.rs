//! Configuration management for backends.
//!
//! This module provides types for loading and managing backend configuration
//! from TOML files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Backend configuration.
///
/// Contains the backend name and backend-specific parameters loaded from
/// a TOML configuration file.
///
/// # Examples
///
/// ```
/// use canlink_hal::BackendConfig;
///
/// let config = BackendConfig {
///     backend_name: "mock".to_string(),
///     retry_count: Some(3),
///     retry_interval_ms: Some(1000),
///     parameters: std::collections::HashMap::new(),
/// };
///
/// assert_eq!(config.backend_name, "mock");
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendConfig {
    /// Backend name (e.g., "tsmaster", "mock", "peak")
    pub backend_name: String,

    /// Number of initialization retry attempts (default: 3)
    #[serde(default = "default_retry_count")]
    pub retry_count: Option<u32>,

    /// Retry interval in milliseconds (default: 1000)
    #[serde(default = "default_retry_interval")]
    pub retry_interval_ms: Option<u64>,

    /// Backend-specific parameters
    #[serde(flatten)]
    pub parameters: HashMap<String, toml::Value>,
}

#[allow(clippy::unnecessary_wraps)]
fn default_retry_count() -> Option<u32> {
    Some(3)
}

#[allow(clippy::unnecessary_wraps)]
fn default_retry_interval() -> Option<u64> {
    Some(1000)
}

impl BackendConfig {
    /// Create a new backend configuration.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendConfig;
    ///
    /// let config = BackendConfig::new("mock");
    /// assert_eq!(config.backend_name, "mock");
    /// ```
    #[must_use]
    pub fn new(backend_name: impl Into<String>) -> Self {
        Self {
            backend_name: backend_name.into(),
            retry_count: Some(3),
            retry_interval_ms: Some(1000),
            parameters: HashMap::new(),
        }
    }

    /// Get a parameter value.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendConfig;
    ///
    /// let mut config = BackendConfig::new("mock");
    /// config.parameters.insert("device_index".to_string(), toml::Value::Integer(0));
    ///
    /// let value = config.get_parameter("device_index");
    /// assert!(value.is_some());
    /// ```
    #[must_use]
    pub fn get_parameter(&self, key: &str) -> Option<&toml::Value> {
        self.parameters.get(key)
    }

    /// Get a parameter as an integer.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendConfig;
    ///
    /// let mut config = BackendConfig::new("mock");
    /// config.parameters.insert("device_index".to_string(), toml::Value::Integer(0));
    ///
    /// assert_eq!(config.get_int("device_index"), Some(0));
    /// ```
    #[must_use]
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.parameters.get(key)?.as_integer()
    }

    /// Get a parameter as a string.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendConfig;
    ///
    /// let mut config = BackendConfig::new("mock");
    /// config.parameters.insert("device".to_string(), toml::Value::String("can0".to_string()));
    ///
    /// assert_eq!(config.get_string("device"), Some("can0"));
    /// ```
    #[must_use]
    pub fn get_string(&self, key: &str) -> Option<&str> {
        self.parameters.get(key)?.as_str()
    }

    /// Get a parameter as a boolean.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendConfig;
    ///
    /// let mut config = BackendConfig::new("mock");
    /// config.parameters.insert("canfd".to_string(), toml::Value::Boolean(true));
    ///
    /// assert_eq!(config.get_bool("canfd"), Some(true));
    /// ```
    #[must_use]
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.parameters.get(key)?.as_bool()
    }
}

/// Complete configuration file structure.
///
/// Represents the top-level structure of a `canlink.toml` configuration file.
///
/// # Examples
///
/// ```toml
/// [backend]
/// backend_name = "mock"
/// retry_count = 3
/// retry_interval_ms = 1000
/// device_index = 0
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanlinkConfig {
    /// Backend configuration
    pub backend: BackendConfig,
}

impl CanlinkConfig {
    /// Load configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the TOML configuration file
    ///
    /// # Errors
    ///
    /// Returns `CanError::ConfigError` if the file cannot be read or parsed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use canlink_hal::CanlinkConfig;
    ///
    /// let config = CanlinkConfig::from_file("canlink.toml").unwrap();
    /// println!("Backend: {}", config.backend.backend_name);
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, crate::error::CanError> {
        let path = path.as_ref();
        let content =
            std::fs::read_to_string(path).map_err(|e| crate::error::CanError::ConfigError {
                reason: format!("Failed to read config file '{}': {}", path.display(), e),
            })?;

        Self::parse_toml(&content)
    }

    /// Parse configuration from a TOML string.
    ///
    /// # Errors
    ///
    /// Returns `CanError::ConfigError` if the TOML is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanlinkConfig;
    ///
    /// let toml = r#"
    /// [backend]
    /// backend_name = "mock"
    /// "#;
    ///
    /// let config = CanlinkConfig::parse_toml(toml).unwrap();
    /// assert_eq!(config.backend.backend_name, "mock");
    /// ```
    pub fn parse_toml(s: &str) -> Result<Self, crate::error::CanError> {
        toml::from_str(s).map_err(|e| crate::error::CanError::ConfigError {
            reason: format!("Failed to parse TOML config: {e}"),
        })
    }

    /// Create a default configuration with the specified backend.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::CanlinkConfig;
    ///
    /// let config = CanlinkConfig::with_backend("mock");
    /// assert_eq!(config.backend.backend_name, "mock");
    /// ```
    #[must_use]
    pub fn with_backend(backend_name: impl Into<String>) -> Self {
        Self {
            backend: BackendConfig::new(backend_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_config_new() {
        let config = BackendConfig::new("mock");
        assert_eq!(config.backend_name, "mock");
        assert_eq!(config.retry_count, Some(3));
        assert_eq!(config.retry_interval_ms, Some(1000));
    }

    #[test]
    fn test_backend_config_parameters() {
        let mut config = BackendConfig::new("mock");
        config
            .parameters
            .insert("device_index".to_string(), toml::Value::Integer(0));
        config
            .parameters
            .insert("canfd".to_string(), toml::Value::Boolean(true));
        config.parameters.insert(
            "device".to_string(),
            toml::Value::String("can0".to_string()),
        );

        assert_eq!(config.get_int("device_index"), Some(0));
        assert_eq!(config.get_bool("canfd"), Some(true));
        assert_eq!(config.get_string("device"), Some("can0"));
    }

    #[test]
    fn test_canlink_config_from_str() {
        let toml = r#"
[backend]
backend_name = "mock"
retry_count = 5
retry_interval_ms = 2000
device_index = 1
"#;

        let config = CanlinkConfig::parse_toml(toml).unwrap();
        assert_eq!(config.backend.backend_name, "mock");
        assert_eq!(config.backend.retry_count, Some(5));
        assert_eq!(config.backend.retry_interval_ms, Some(2000));
        assert_eq!(config.backend.get_int("device_index"), Some(1));
    }

    #[test]
    fn test_canlink_config_with_backend() {
        let config = CanlinkConfig::with_backend("mock");
        assert_eq!(config.backend.backend_name, "mock");
    }

    #[test]
    fn test_invalid_toml() {
        let toml = "invalid toml {{{";
        assert!(CanlinkConfig::parse_toml(toml).is_err());
    }
}
