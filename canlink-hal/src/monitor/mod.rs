//! Connection monitoring module (FR-010)
//!
//! This module provides connection state monitoring with optional
//! automatic reconnection support.
//!
//! # Overview
//!
//! The monitor module helps track the health of CAN backend connections
//! and optionally handles automatic reconnection when connections are lost.
//!
//! # Components
//!
//! - [`ConnectionMonitor`]: Main monitoring component
//! - [`ConnectionState`]: Current connection state (Connected, Disconnected, Reconnecting)
//! - [`ReconnectConfig`]: Configuration for automatic reconnection
//! - [`MonitorConfig`]: Configuration loaded from TOML files
//!
//! # Connection States
//!
//! - [`ConnectionState::Connected`]: Backend is operational
//! - [`ConnectionState::Disconnected`]: Connection lost, needs re-initialization
//! - [`ConnectionState::Reconnecting`]: Auto-reconnect in progress
//!
//! # Example
//!
//! ```rust
//! use canlink_hal::monitor::{ConnectionMonitor, ConnectionState, ReconnectConfig};
//! use std::time::Duration;
//!
//! // Create monitor without auto-reconnect
//! let monitor = ConnectionMonitor::new(Duration::from_secs(1));
//! assert_eq!(monitor.state(), ConnectionState::Connected);
//!
//! // Create monitor with auto-reconnect
//! let reconnect_config = ReconnectConfig::exponential_backoff(
//!     5,                          // max retries
//!     Duration::from_secs(1),     // initial interval
//!     2.0,                        // backoff multiplier
//! );
//! let monitor = ConnectionMonitor::with_reconnect(
//!     Duration::from_secs(1),
//!     reconnect_config,
//! );
//! assert!(monitor.auto_reconnect_enabled());
//! ```
//!
//! # Auto-Reconnect
//!
//! When auto-reconnect is enabled, the monitor will automatically attempt
//! to reconnect when a disconnection is detected. The reconnection behavior
//! is controlled by [`ReconnectConfig`]:
//!
//! - `max_retries`: Maximum reconnection attempts (0 = unlimited)
//! - `retry_interval`: Initial delay between attempts
//! - `backoff_multiplier`: Exponential backoff factor

mod config;
mod connection;
mod reconnect;
mod state;

pub use config::MonitorConfig;
pub use connection::ConnectionMonitor;
pub use reconnect::ReconnectConfig;
pub use state::ConnectionState;
