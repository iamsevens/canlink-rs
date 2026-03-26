//! # CANLink TSCan Backend
//! <a id="en"></a>
//! [English](#en) | [中文](#zh)
//!
//! Safe Rust backend for LibTSCAN-backed CAN hardware. This crate implements
//! `CanBackend` from `canlink-hal`.
//!
//! ## Validation Scope
//!
//! - Real-hardware regression in this repository is limited to TOSUN-related devices.
//! - LibTSCAN documentation lists more device types on the same backend path, but
//!   they are not individually validated here.
//! - Future vendor-native SDK paths should land as separate backend crates.
//!
//! ## Requirements
//!
//! - Windows 10/11 x64 (validated)
//! - LibTSCAN runtime (`libTSCAN.dll` + `libTSCAN.lib`)
//! - LibTSCAN is not distributed by this project
//!
//! ## Quick Start
//!
//! ```rust,no_run
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
//! ## TSCan Daemon Workaround (Vendor Bug)
//!
//! To isolate a known vendor DLL hang in `DISCONNECT_*`, this crate supports an
//! out-of-process daemon path and enables it by default. See the crate README for
//! configuration details.
//!
//! ## Related Crates
//!
//! - [`canlink-hal`](https://docs.rs/canlink-hal) - Core HAL
//! - [`canlink-tscan-sys`](https://docs.rs/canlink-tscan-sys) - LibTSCAN FFI bindings
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - CLI tool
//!
//! <a id="zh"></a>
//! [中文](#zh) | [English](#en)
//!
//! 基于 LibTSCAN 的安全 Rust 后端，实现 `canlink-hal` 的 `CanBackend`。
//!
//! ## 验证范围
//!
//! - 当前仓库的实机回归仅覆盖同星 / TOSUN 相关硬件。
//! - LibTSCAN 文档列出的其他设备类型尚未逐项验证。
//! - 若未来需要厂商原生 SDK 路径，应以新的独立后端 crate 落地。
//!
//! ## 环境要求
//!
//! - Windows 10/11 x64（已验证）
//! - LibTSCAN 运行库（`libTSCAN.dll` + `libTSCAN.lib`）
//! - 本项目不分发 LibTSCAN 文件
//!
//! ## 快速开始
//!
//! ```rust,no_run
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
//! ## TSCan 守护进程规避方案（厂商 DLL 问题）
//!
//! 为隔离 `DISCONNECT_*` 调用的厂商 DLL 卡死问题，本 crate 支持独立守护进程路径并默认启用。
//! 具体配置见 crate README。
//!
//! ## 相关包
//!
//! - [`canlink-hal`](https://docs.rs/canlink-hal) - 核心 HAL
//! - [`canlink-tscan-sys`](https://docs.rs/canlink-tscan-sys) - LibTSCAN FFI 绑定
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - 命令行工具
//!#![deny(missing_docs)]

mod backend;
mod config;
mod convert;
#[doc(hidden)]
pub mod daemon;
mod error;

pub use backend::{TSCanBackend, TSCanBackendFactory};
pub use config::{FileConfig, TscanDaemonConfig};

// Re-export commonly used types from canlink-hal
pub use canlink_hal::{
    BackendConfig, BackendVersion, CanBackend, CanError, CanId, CanMessage, CanResult,
    HardwareCapability, MessageFlags, Timestamp,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backend_creation() {
        let backend = TSCanBackend::new();
        assert_eq!(backend.name(), "tscan");
    }
}

