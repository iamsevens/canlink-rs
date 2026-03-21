//! Logging module (FR-013)
//!
//! This module provides logging functionality using the `tracing` framework.
//! It is conditionally compiled when the `tracing` feature is enabled.
//!
//! # Feature Flag
//!
//! This module requires the `tracing` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! canlink-hal = { version = "0.1", features = ["tracing"] }
//! ```
//!
//! # Log Levels
//!
//! | Level | Content |
//! |-------|---------|
//! | ERROR | Errors and exceptions |
//! | WARN  | Warnings (queue overflow, high-frequency messages) |
//! | INFO  | Important operations (init, close, state changes) |
//! | DEBUG | Detailed operations (message send/receive) |
//! | TRACE | Most detailed info (filter matching, queue ops) |

/// Re-export tracing macros for convenience
pub use tracing::{debug, error, info, trace, warn};

/// Re-export tracing span for structured logging
pub use tracing::span;

/// Re-export tracing Level for configuration
pub use tracing::Level;

// Note: To enable logging output, applications should set up their own
// tracing subscriber. For example, using tracing-subscriber:
//
// ```rust,ignore
// use tracing_subscriber::FmtSubscriber;
//
// fn main() {
//     let subscriber = FmtSubscriber::builder()
//         .with_max_level(tracing::Level::INFO)
//         .finish();
//     tracing::subscriber::set_global_default(subscriber).unwrap();
//     // Now all canlink operations will be logged
// }
// ```

/// Span names for structured logging
pub mod spans {
    /// Span for backend operations
    pub const BACKEND: &str = "canlink::backend";
    /// Span for message operations
    pub const MESSAGE: &str = "canlink::message";
    /// Span for filter operations
    pub const FILTER: &str = "canlink::filter";
    /// Span for queue operations
    pub const QUEUE: &str = "canlink::queue";
    /// Span for monitor operations
    pub const MONITOR: &str = "canlink::monitor";
}

/// Log a backend initialization event
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_backend_init {
    ($backend_name:expr) => {
        tracing::info!(
            target: $crate::logging::spans::BACKEND,
            backend = $backend_name,
            "Backend initialized"
        );
    };
}

/// Log a backend close event
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_backend_close {
    ($backend_name:expr) => {
        tracing::info!(
            target: $crate::logging::spans::BACKEND,
            backend = $backend_name,
            "Backend closed"
        );
    };
}

/// Log a message send event
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_message_send {
    ($channel:expr, $id:expr) => {
        tracing::debug!(
            target: $crate::logging::spans::MESSAGE,
            channel = $channel,
            id = $id,
            "Message sent"
        );
    };
}

/// Log a message receive event
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_message_receive {
    ($channel:expr, $id:expr) => {
        tracing::debug!(
            target: $crate::logging::spans::MESSAGE,
            channel = $channel,
            id = $id,
            "Message received"
        );
    };
}

/// Log a queue overflow event
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_queue_overflow {
    ($policy:expr, $dropped_id:expr) => {
        tracing::warn!(
            target: $crate::logging::spans::QUEUE,
            policy = ?$policy,
            dropped_id = $dropped_id,
            "Queue overflow, message dropped"
        );
    };
}

/// Log a high-frequency message warning (FR-016)
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_high_frequency_warning {
    ($rate:expr, $threshold:expr) => {
        tracing::warn!(
            target: $crate::logging::spans::MESSAGE,
            rate = $rate,
            threshold = $threshold,
            "High message frequency detected"
        );
    };
}

/// Log a connection state change
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_connection_state_change {
    ($old:expr, $new:expr) => {
        tracing::info!(
            target: $crate::logging::spans::MONITOR,
            old_state = ?$old,
            new_state = ?$new,
            "Connection state changed"
        );
    };
}

/// Log a filter match event
#[macro_export]
#[cfg(feature = "tracing")]
macro_rules! log_filter_match {
    ($filter_type:expr, $id:expr, $matched:expr) => {
        tracing::trace!(
            target: $crate::logging::spans::FILTER,
            filter_type = $filter_type,
            message_id = $id,
            matched = $matched,
            "Filter evaluated"
        );
    };
}

// No-op versions when tracing is disabled
#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_backend_init {
    ($backend_name:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_backend_close {
    ($backend_name:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_message_send {
    ($channel:expr, $id:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_message_receive {
    ($channel:expr, $id:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_queue_overflow {
    ($policy:expr, $dropped_id:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_high_frequency_warning {
    ($rate:expr, $threshold:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_connection_state_change {
    ($old:expr, $new:expr) => {};
}

#[macro_export]
#[cfg(not(feature = "tracing"))]
macro_rules! log_filter_match {
    ($filter_type:expr, $id:expr, $matched:expr) => {};
}
