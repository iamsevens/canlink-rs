//! ConnectionMonitor unit tests (T049)
//!
//! Tests for the connection monitor implementation.

use canlink_hal::monitor::{ConnectionMonitor, ConnectionState, ReconnectConfig};
use std::time::Duration;

#[test]
fn test_new_monitor() {
    let monitor = ConnectionMonitor::new(Duration::from_secs(1));
    // New monitor starts in Connected state
    assert_eq!(monitor.state(), ConnectionState::Connected);
    assert!(!monitor.auto_reconnect_enabled());
}

#[test]
fn test_with_reconnect() {
    let reconnect_config = ReconnectConfig {
        max_retries: 5,
        retry_interval: Duration::from_millis(100),
        backoff_multiplier: 2.0,
    };

    let monitor = ConnectionMonitor::with_reconnect(Duration::from_secs(1), reconnect_config);

    assert_eq!(monitor.state(), ConnectionState::Connected);
    assert!(monitor.auto_reconnect_enabled());
    assert!(monitor.reconnect_config().is_some());
}

#[test]
fn test_state_transitions() {
    let mut monitor = ConnectionMonitor::new(Duration::from_millis(100));

    // Initial state is Connected
    assert_eq!(monitor.state(), ConnectionState::Connected);

    // Simulate disconnection
    monitor.set_state(ConnectionState::Disconnected);
    assert_eq!(monitor.state(), ConnectionState::Disconnected);

    // Simulate reconnecting
    monitor.set_state(ConnectionState::Reconnecting);
    assert_eq!(monitor.state(), ConnectionState::Reconnecting);

    // Back to connected
    monitor.set_state(ConnectionState::Connected);
    assert_eq!(monitor.state(), ConnectionState::Connected);
}

#[test]
fn test_connection_state_can_send() {
    assert!(ConnectionState::Connected.can_send());
    assert!(!ConnectionState::Disconnected.can_send());
    assert!(!ConnectionState::Reconnecting.can_send());
}

#[test]
fn test_connection_state_can_receive() {
    assert!(ConnectionState::Connected.can_receive());
    assert!(!ConnectionState::Disconnected.can_receive());
    assert!(!ConnectionState::Reconnecting.can_receive());
}

#[test]
fn test_connection_state_is_active() {
    assert!(ConnectionState::Connected.is_active());
    assert!(!ConnectionState::Disconnected.is_active());
    assert!(!ConnectionState::Reconnecting.is_active());
}

#[test]
fn test_connection_state_default() {
    let state = ConnectionState::default();
    assert_eq!(state, ConnectionState::Disconnected);
}

#[test]
fn test_reconnect_config_default() {
    let config = ReconnectConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.retry_interval, Duration::from_secs(1));
    assert!((config.backoff_multiplier - 2.0).abs() < f32::EPSILON);
}

#[test]
fn test_reconnect_config_fixed_interval() {
    let config = ReconnectConfig::fixed_interval(5, Duration::from_millis(500));
    assert_eq!(config.max_retries, 5);
    assert_eq!(config.retry_interval, Duration::from_millis(500));
    assert_eq!(config.backoff_multiplier, 1.0);

    // All attempts should have the same interval
    assert_eq!(
        config.interval_for_attempt(0),
        config.interval_for_attempt(3)
    );
}

#[test]
fn test_reconnect_config_exponential_backoff() {
    let config = ReconnectConfig::exponential_backoff(5, Duration::from_secs(1), 2.0);

    assert_eq!(config.interval_for_attempt(0), Duration::from_secs(1));
    assert_eq!(config.interval_for_attempt(1), Duration::from_secs(2));
    assert_eq!(config.interval_for_attempt(2), Duration::from_secs(4));
}

#[test]
fn test_reconnect_config_should_retry() {
    let config = ReconnectConfig {
        max_retries: 3,
        retry_interval: Duration::from_secs(1),
        backoff_multiplier: 2.0,
    };

    assert!(config.should_retry(0));
    assert!(config.should_retry(2));
    assert!(!config.should_retry(3));
    assert!(!config.should_retry(10));
}

#[test]
fn test_reconnect_config_unlimited_retries() {
    let config = ReconnectConfig {
        max_retries: 0, // Unlimited
        retry_interval: Duration::from_secs(1),
        backoff_multiplier: 2.0,
    };

    assert!(config.should_retry(100));
    assert!(config.should_retry(1000));
}

#[test]
fn test_heartbeat_interval() {
    let monitor = ConnectionMonitor::new(Duration::from_millis(500));
    assert_eq!(monitor.heartbeat_interval(), Duration::from_millis(500));
}

#[test]
fn test_monitor_default() {
    let monitor = ConnectionMonitor::default();
    assert_eq!(monitor.heartbeat_interval(), Duration::from_secs(1));
    assert!(!monitor.auto_reconnect_enabled());
}

#[test]
fn test_reconnect_config_new() {
    let config = ReconnectConfig::new();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.retry_interval, Duration::from_secs(1));
}
