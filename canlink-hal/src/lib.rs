//! # CANLink HAL
//! <a id="en"></a>
//! [English](#en) | [中文](#zh)
//!
//! CANLink HAL is the core hardware abstraction layer of CANLink. It defines the
//! `CanBackend` trait, message types, and the backend registry used by real
//! hardware backends.
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use canlink_hal::{BackendConfig, CanBackend, CanMessage};
//! use canlink_tscan::TSCanBackend;
//!
//! # fn main() -> Result<(), canlink_hal::CanError> {
//! let mut backend = TSCanBackend::new();
//! let config = BackendConfig::new("tscan");
//!
//! backend.initialize(&config)?;
//! backend.open_channel(0)?;
//!
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
//! backend.send_message(&msg)?;
//!
//! backend.close_channel(0)?;
//! backend.close()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Scope
//!
//! This crate is hardware-agnostic. The only real-hardware backend currently
//! landed in this repository is `canlink-tscan` (LibTSCAN).
//!
//! ## Related Crates
//!
//! - [`canlink-tscan-sys`](https://docs.rs/canlink-tscan-sys) - LibTSCAN FFI
//! - [`canlink-tscan`](https://docs.rs/canlink-tscan) - LibTSCAN backend
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - CLI tool
//!
//! <a id="zh"></a>
//! [中文](#zh) | [English](#en)
//!
//! CANLink HAL 是 CANLink 的核心硬件抽象层，定义 `CanBackend` trait、消息类型以及后端注册表。
//!
//! ## 快速开始
//!
//! ```rust,ignore
//! use canlink_hal::{BackendConfig, CanBackend, CanMessage};
//! use canlink_tscan::TSCanBackend;
//!
//! # fn main() -> Result<(), canlink_hal::CanError> {
//! let mut backend = TSCanBackend::new();
//! let config = BackendConfig::new("tscan");
//!
//! backend.initialize(&config)?;
//! backend.open_channel(0)?;
//!
//! let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
//! backend.send_message(&msg)?;
//!
//! backend.close_channel(0)?;
//! backend.close()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## 定位
//!
//! 本 crate 与具体硬件无关。当前仓库唯一已落地的真实硬件后端是 `canlink-tscan`（LibTSCAN）。
//!
//! ## 相关包
//!
//! - [`canlink-tscan-sys`](https://docs.rs/canlink-tscan-sys) - LibTSCAN FFI 绑定
//! - [`canlink-tscan`](https://docs.rs/canlink-tscan) - LibTSCAN 后端
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - 命令行工具
//!#![deny(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

// Core modules
pub mod backend;
pub mod capability;
pub mod config;
pub mod error;
pub mod message;
pub mod registry;
pub mod state;
pub mod version;

// New modules introduced in v0.2.0 (003-async-and-filtering)
pub mod filter;
pub mod monitor;
pub mod queue;

// Conditional modules
#[cfg(feature = "tracing")]
pub mod logging;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

// Periodic message sending (004 spec FR-001 to FR-006)
#[cfg(feature = "periodic")]
pub mod periodic;

// ISO-TP protocol support (004 spec FR-007 to FR-019)
#[cfg(feature = "isotp")]
pub mod isotp;

// Resource management documentation
pub mod resource;

// Re-exports
#[cfg(feature = "async")]
pub use backend::CanBackendAsync;
pub use backend::{
    retry_initialize, switch_backend, BackendFactory, CanBackend, MessageRateMonitor,
};
pub use capability::{HardwareCapability, TimestampPrecision};
pub use config::{BackendConfig, CanlinkConfig};
pub use error::{BusErrorKind, CanError, CanResult};
pub use error::{FilterError, FilterResult};
pub use error::{MonitorError, MonitorResult};
pub use error::{QueueError, QueueResult};
pub use message::{CanId, CanMessage, MessageFlags, Timestamp};
pub use registry::{BackendInfo, BackendRegistry};
pub use state::BackendState;
pub use version::BackendVersion;

// Periodic message sending re-exports (004 spec)
#[cfg(feature = "periodic")]
pub use periodic::{
    run_scheduler, PeriodicMessage, PeriodicScheduler, PeriodicStats, SchedulerCommand,
};

// ISO-TP protocol re-exports (004 spec)
#[cfg(feature = "isotp")]
pub use isotp::{
    AddressingMode, FlowStatus, FrameSize, IsoTpConfig, IsoTpConfigBuilder, IsoTpError, IsoTpFrame,
    IsoTpState, RxState, StMin, TxState,
};
