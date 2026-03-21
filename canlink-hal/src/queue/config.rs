//! Queue configuration (FR-011, FR-017)
//!
//! Provides configuration structures for loading queue settings from TOML.

use std::time::Duration;

use serde::Deserialize;

use super::{BoundedQueue, QueueOverflowPolicy};

/// Queue configuration from TOML
///
/// # Example TOML
///
/// ```toml
/// [queue]
/// capacity = 2000
///
/// [queue.overflow_policy]
/// type = "drop_oldest"
/// ```
///
/// Or with block policy:
///
/// ```toml
/// [queue]
/// capacity = 1000
///
/// [queue.overflow_policy]
/// type = "block"
/// timeout_ms = 100
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    /// Queue capacity (default: 1000)
    #[serde(default = "default_capacity")]
    pub capacity: usize,

    /// Overflow policy configuration
    #[serde(default)]
    pub overflow_policy: OverflowPolicyConfig,
}

fn default_capacity() -> usize {
    super::bounded::DEFAULT_QUEUE_CAPACITY
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            capacity: default_capacity(),
            overflow_policy: OverflowPolicyConfig::default(),
        }
    }
}

/// Overflow policy configuration from TOML
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[derive(Default)]
pub enum OverflowPolicyConfig {
    /// Drop oldest message
    #[default]
    DropOldest,
    /// Drop newest message
    DropNewest,
    /// Block with timeout
    Block {
        /// Timeout in milliseconds
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
    },
}

fn default_timeout_ms() -> u64 {
    100
}

impl From<OverflowPolicyConfig> for QueueOverflowPolicy {
    fn from(config: OverflowPolicyConfig) -> Self {
        match config {
            OverflowPolicyConfig::DropOldest => QueueOverflowPolicy::DropOldest,
            OverflowPolicyConfig::DropNewest => QueueOverflowPolicy::DropNewest,
            OverflowPolicyConfig::Block { timeout_ms } => QueueOverflowPolicy::Block {
                timeout: Duration::from_millis(timeout_ms),
            },
        }
    }
}

impl QueueConfig {
    /// Create a `BoundedQueue` from this configuration
    #[must_use]
    pub fn into_queue(self) -> BoundedQueue {
        BoundedQueue::with_policy(self.capacity, self.overflow_policy.into())
    }

    /// Load configuration from TOML string
    ///
    /// # Errors
    ///
    /// Returns `toml::de::Error` if the TOML string is invalid or cannot be
    /// deserialized into a `QueueConfig`.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }
}

impl BoundedQueue {
    /// Create a `BoundedQueue` from configuration
    #[must_use]
    pub fn from_config(config: &QueueConfig) -> Self {
        BoundedQueue::with_policy(config.capacity, config.overflow_policy.clone().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = QueueConfig::default();
        assert_eq!(config.capacity, 1000);
        assert!(matches!(
            config.overflow_policy,
            OverflowPolicyConfig::DropOldest
        ));
    }

    #[test]
    fn test_parse_drop_oldest() {
        let toml = r#"
            capacity = 500
            [overflow_policy]
            type = "drop_oldest"
        "#;

        let config: QueueConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.capacity, 500);
        assert!(matches!(
            config.overflow_policy,
            OverflowPolicyConfig::DropOldest
        ));
    }

    #[test]
    fn test_parse_drop_newest() {
        let toml = r#"
            capacity = 200
            [overflow_policy]
            type = "drop_newest"
        "#;

        let config: QueueConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.capacity, 200);
        assert!(matches!(
            config.overflow_policy,
            OverflowPolicyConfig::DropNewest
        ));
    }

    #[test]
    fn test_parse_block() {
        let toml = r#"
            capacity = 100
            [overflow_policy]
            type = "block"
            timeout_ms = 250
        "#;

        let config: QueueConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.capacity, 100);
        match config.overflow_policy {
            OverflowPolicyConfig::Block { timeout_ms } => {
                assert_eq!(timeout_ms, 250);
            }
            _ => panic!("Expected Block policy"),
        }
    }

    #[test]
    fn test_into_queue() {
        let config = QueueConfig {
            capacity: 50,
            overflow_policy: OverflowPolicyConfig::DropNewest,
        };

        let queue = config.into_queue();
        assert_eq!(queue.capacity(), 50);
        assert!(matches!(queue.policy(), QueueOverflowPolicy::DropNewest));
    }

    #[test]
    fn test_policy_conversion() {
        let policy: QueueOverflowPolicy = OverflowPolicyConfig::DropOldest.into();
        assert!(matches!(policy, QueueOverflowPolicy::DropOldest));

        let policy: QueueOverflowPolicy = OverflowPolicyConfig::DropNewest.into();
        assert!(matches!(policy, QueueOverflowPolicy::DropNewest));

        let policy: QueueOverflowPolicy = OverflowPolicyConfig::Block { timeout_ms: 100 }.into();
        assert!(matches!(policy, QueueOverflowPolicy::Block { .. }));
        assert_eq!(policy.timeout(), Some(Duration::from_millis(100)));
    }
}
