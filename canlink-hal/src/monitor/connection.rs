//! Connection monitor implementation (FR-010)
//!
//! Provides connection state monitoring with optional auto-reconnect.

use std::time::Duration;

use super::{ConnectionState, ReconnectConfig};

/// Connection monitor
///
/// Monitors backend connection state and optionally handles automatic
/// reconnection. By default, auto-reconnect is disabled.
///
/// # Example
///
/// ```rust,ignore
/// use canlink_hal::monitor::{ConnectionMonitor, ConnectionState};
/// use std::time::Duration;
///
/// let monitor = ConnectionMonitor::new(Duration::from_secs(1));
/// ```
pub struct ConnectionMonitor {
    /// Heartbeat interval
    heartbeat_interval: Duration,
    /// Reconnect configuration (None = disabled)
    reconnect_config: Option<ReconnectConfig>,
    /// Current connection state
    state: ConnectionState,
}

impl ConnectionMonitor {
    /// Create a new connection monitor without auto-reconnect
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - Interval between heartbeat checks
    #[must_use]
    pub fn new(heartbeat_interval: Duration) -> Self {
        Self {
            heartbeat_interval,
            reconnect_config: None,
            state: ConnectionState::Connected,
        }
    }

    /// Create a connection monitor with auto-reconnect enabled
    ///
    /// # Arguments
    ///
    /// * `heartbeat_interval` - Interval between heartbeat checks
    /// * `reconnect_config` - Reconnection configuration
    #[must_use]
    pub fn with_reconnect(heartbeat_interval: Duration, reconnect_config: ReconnectConfig) -> Self {
        Self {
            heartbeat_interval,
            reconnect_config: Some(reconnect_config),
            state: ConnectionState::Connected,
        }
    }

    /// Get the current connection state
    #[must_use]
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Get the heartbeat interval
    #[must_use]
    pub fn heartbeat_interval(&self) -> Duration {
        self.heartbeat_interval
    }

    /// Check if auto-reconnect is enabled
    #[must_use]
    pub fn auto_reconnect_enabled(&self) -> bool {
        self.reconnect_config.is_some()
    }

    /// Get the reconnect configuration
    #[must_use]
    pub fn reconnect_config(&self) -> Option<&ReconnectConfig> {
        self.reconnect_config.as_ref()
    }

    /// Set the connection state (for testing/manual control)
    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }
}

impl Default for ConnectionMonitor {
    fn default() -> Self {
        Self::new(Duration::from_secs(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_monitor() {
        let monitor = ConnectionMonitor::new(Duration::from_secs(1));
        assert_eq!(monitor.state(), ConnectionState::Connected);
        assert!(!monitor.auto_reconnect_enabled());
    }

    #[test]
    fn test_with_reconnect() {
        let monitor =
            ConnectionMonitor::with_reconnect(Duration::from_secs(1), ReconnectConfig::default());
        assert!(monitor.auto_reconnect_enabled());
    }

    #[test]
    fn test_set_state() {
        let mut monitor = ConnectionMonitor::new(Duration::from_secs(1));
        monitor.set_state(ConnectionState::Disconnected);
        assert_eq!(monitor.state(), ConnectionState::Disconnected);
    }
}
