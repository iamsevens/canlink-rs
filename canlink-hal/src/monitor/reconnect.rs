//! Reconnection configuration (FR-010)

use std::time::Duration;

/// Reconnection configuration
///
/// Configures automatic reconnection behavior. By default, auto-reconnect
/// is disabled to avoid masking hardware issues.
///
/// # Example
///
/// ```rust
/// use canlink_hal::monitor::ReconnectConfig;
/// use std::time::Duration;
///
/// let config = ReconnectConfig {
///     max_retries: 5,
///     retry_interval: Duration::from_secs(2),
///     backoff_multiplier: 1.5,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Maximum number of reconnection attempts
    ///
    /// Set to 0 for unlimited retries.
    pub max_retries: u32,

    /// Initial interval between reconnection attempts
    pub retry_interval: Duration,

    /// Backoff multiplier for exponential backoff
    ///
    /// After each failed attempt, the interval is multiplied by this value.
    /// Set to 1.0 for fixed intervals.
    pub backoff_multiplier: f32,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        }
    }
}

impl ReconnectConfig {
    /// Create a new reconnect config with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a config with fixed retry interval (no backoff)
    #[must_use]
    pub fn fixed_interval(max_retries: u32, interval: Duration) -> Self {
        Self {
            max_retries,
            retry_interval: interval,
            backoff_multiplier: 1.0,
        }
    }

    /// Create a config with exponential backoff
    #[must_use]
    pub fn exponential_backoff(
        max_retries: u32,
        initial_interval: Duration,
        multiplier: f32,
    ) -> Self {
        Self {
            max_retries,
            retry_interval: initial_interval,
            backoff_multiplier: multiplier,
        }
    }

    /// Calculate the interval for a given retry attempt
    #[must_use]
    #[allow(clippy::cast_possible_wrap)]
    pub fn interval_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.retry_interval;
        }

        let multiplier = self.backoff_multiplier.powi(attempt as i32);
        Duration::from_secs_f32(self.retry_interval.as_secs_f32() * multiplier)
    }

    /// Check if more retries are allowed
    #[must_use]
    pub fn should_retry(&self, current_attempt: u32) -> bool {
        self.max_retries == 0 || current_attempt < self.max_retries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ReconnectConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_interval, Duration::from_secs(1));
        assert!((config.backoff_multiplier - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_fixed_interval() {
        let config = ReconnectConfig::fixed_interval(5, Duration::from_millis(500));
        assert_eq!(config.max_retries, 5);
        assert!((config.backoff_multiplier - 1.0).abs() < f32::EPSILON);

        // All attempts should have the same interval
        assert_eq!(
            config.interval_for_attempt(0),
            config.interval_for_attempt(3)
        );
    }

    #[test]
    fn test_exponential_backoff() {
        let config = ReconnectConfig::exponential_backoff(5, Duration::from_secs(1), 2.0);

        assert_eq!(config.interval_for_attempt(0), Duration::from_secs(1));
        assert_eq!(config.interval_for_attempt(1), Duration::from_secs(2));
        assert_eq!(config.interval_for_attempt(2), Duration::from_secs(4));
    }

    #[test]
    fn test_should_retry() {
        let config = ReconnectConfig {
            max_retries: 3,
            ..Default::default()
        };

        assert!(config.should_retry(0));
        assert!(config.should_retry(2));
        assert!(!config.should_retry(3));
    }

    #[test]
    fn test_unlimited_retries() {
        let config = ReconnectConfig {
            max_retries: 0, // Unlimited
            ..Default::default()
        };

        assert!(config.should_retry(100));
        assert!(config.should_retry(1000));
    }
}
