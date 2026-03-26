//! # `CANLink` TSCan Sys
//! <a id="en"></a>
//! [English](#en) | [中文](#zh)
//!
//! Low-level, unsafe Rust FFI bindings to the `LibTSCAN` C API.
//!
//! ## Warning
//!
//! This crate exposes raw C functions. For a safe, high-level API, use
//! `canlink-tscan` instead.
//!
//! ## Platform
//!
//! Windows 10/11 x64 is validated in this project.
//! Vendor packages may also include Windows x86 / Linux artifacts, but they are
//! not validated here, and this crate currently compiles only for Windows targets.
//! `LibTSCAN` runtime is required and is not distributed by this project.
//! In practice, besides `libTSCAN.dll` + `libTSCAN.lib`, dependent DLLs such as
//! `libTSH.dll` may also be required; using the full vendor runtime bundle is recommended.
//!
//! ## Basic Usage
//!
//! ```rust,no_run
//! use canlink_tscan_sys::*;
//! use std::ptr;
//!
//! unsafe {
//!     initialize_lib_tscan(true, false, true);
//!
//!     let mut device_count = 0;
//!     tscan_scan_devices(&mut device_count);
//!
//!     let mut handle = 0;
//!     tscan_connect(ptr::null(), &mut handle);
//!
//!     // ... use device ...
//!
//!     tscan_disconnect_by_handle(handle);
//!     finalize_lib_tscan();
//! }
//! ```
//!
//! ## Related Crates
//!
//! - [`canlink-hal`](https://docs.rs/canlink-hal) - HAL abstraction
//! - [`canlink-tscan`](https://docs.rs/canlink-tscan) - Safe `LibTSCAN` backend
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - CLI tool
//!
//! <a id="zh"></a>
//! [中文](#zh) | [English](#en)
//!
//! `CANLink` TSCan Sys 提供 `LibTSCAN` C API 的底层 Rust FFI 绑定（不安全接口）。
//!
//! ## 警告
//!
//! 此 crate 直接暴露 C 函数。若需要安全、高层 API，请使用 `canlink-tscan`。
//!
//! ## 平台
//!
//! 当前仅在 Windows 10/11 x64 环境验证。
//! 厂商包中可能包含 Windows x86 / Linux 相关库，但这些目标在本项目中尚未验证，
//! 且当前 crate 仅在 Windows 目标下编译。
//! 需要 `LibTSCAN` 运行库，且本项目不分发该运行库。
//! 实际运行中，除 `libTSCAN.dll` + `libTSCAN.lib` 外，通常还需要依赖 DLL（如 `libTSH.dll`）；
//! 建议使用厂商提供的完整运行库目录。
//!
//! ## 基础用法
//!
//! ```rust,no_run
//! use canlink_tscan_sys::*;
//! use std::ptr;
//!
//! unsafe {
//!     initialize_lib_tscan(true, false, true);
//!
//!     let mut device_count = 0;
//!     tscan_scan_devices(&mut device_count);
//!
//!     let mut handle = 0;
//!     tscan_connect(ptr::null(), &mut handle);
//!
//!     // ... 使用设备 ...
//!
//!     tscan_disconnect_by_handle(handle);
//!     finalize_lib_tscan();
//! }
//! ```
//!
//! ## 相关包
//!
//! - [`canlink-hal`](https://docs.rs/canlink-hal) - HAL 抽象层
//! - [`canlink-tscan`](https://docs.rs/canlink-tscan) - 安全的 `LibTSCAN` 后端
//! - [`canlink-cli`](https://docs.rs/canlink-cli) - 命令行工具
//!
#![cfg(windows)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![deny(missing_docs)]

pub mod functions;
pub mod types;

#[cfg(test)]
mod bundle;

// Re-export everything for convenience
pub use functions::*;
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(CHN1, 0);
        assert_eq!(CHN2, 1);
        assert_eq!(MASK_CANPROP_DIR_TX, 0x01);
        assert_eq!(MASK_CANPROP_EXTEND, 0x04);
    }

    #[test]
    fn test_enum_values() {
        assert_eq!(TLIBCANFDControllerType::lfdtCAN as i32, 0);
        assert_eq!(TLIBCANFDControllerType::lfdtISOCAN as i32, 1);
        assert_eq!(TLIBCANFDControllerMode::lfdmNormal as i32, 0);
    }
}
