//! Periodic message sending module.
//!
//! This module provides functionality for sending CAN messages at fixed time intervals.
//! It implements FR-001 to FR-006 from the 004 specification.
//!
//! # Features
//!
//! - Configure messages to send at fixed intervals (1ms - 10000ms)
//! - Dynamic data and interval updates without interrupting the send cycle
//! - Support for multiple concurrent periodic messages (at least 32)
//! - Statistics tracking (send count, actual intervals)
//! - Graceful error handling (skip on failure, continue next cycle)
//!
//! # Example
//!
//! ```rust,ignore
//! use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage};
//! use canlink_hal::{CanMessage, BackendConfig};
//! use canlink_mock::MockBackend;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let mut backend = MockBackend::new();
//!     backend.initialize(&BackendConfig::new("mock"))?;
//!     backend.open_channel(0)?;
//!
//!     let scheduler = PeriodicScheduler::new(backend, 32).await?;
//!
//!     let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
//!     let periodic = PeriodicMessage::new(msg, Duration::from_millis(100))?;
//!
//!     let id = scheduler.add(periodic).await?;
//!
//!     // Let it run...
//!     tokio::time::sleep(Duration::from_secs(1)).await;
//!
//!     scheduler.shutdown().await?;
//!     Ok(())
//! }
//! ```

mod message;
mod scheduler;
mod stats;

pub use message::PeriodicMessage;
pub use scheduler::{run_scheduler, PeriodicScheduler, SchedulerCommand};
pub use stats::PeriodicStats;
