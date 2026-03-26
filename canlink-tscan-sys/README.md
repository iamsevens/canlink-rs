# CANLink TSCan Sys

[![Crates.io](https://img.shields.io/crates/v/canlink-tscan-sys.svg)](https://crates.io/crates/canlink-tscan-sys)
[![Documentation](https://docs.rs/canlink-tscan-sys/badge.svg)](https://docs.rs/canlink-tscan-sys)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)

<a id="en"></a>

[English](#en) | [中文](#zh)

CANLink TSCan Sys provides low-level, unsafe Rust FFI bindings to the LibTSCAN C API.

## Warning

This crate exposes raw C functions. For a safe, high-level API, use `canlink-tscan` instead.

## Validation Scope

Real-hardware regression in this repository is limited to TOSUN-related devices. LibTSCAN documentation lists more device types on the same backend path, but they are not individually validated here.

## Platform Support

- Windows 10/11 x64 (validated)
- Vendor package may include Windows x86 / Linux artifacts, but they are not validated here
- LibTSCAN runtime required (minimum `libTSCAN.dll` + `libTSCAN.lib`; dependent DLLs such as `libTSH.dll` may also be required)
- Full vendor runtime bundle is recommended
- LibTSCAN is not distributed by this project

## Installation

```toml
[dependencies]
canlink-tscan-sys = "0.3.0"
```

## Basic Usage

```rust,no_run
use canlink_tscan_sys::*;
use std::ptr;

unsafe {
    initialize_lib_tscan(true, false, true);

    let mut device_count = 0;
    tscan_scan_devices(&mut device_count);

    let mut handle = 0;
    tscan_connect(ptr::null(), &mut handle);

    // ... use device ...

    tscan_disconnect_by_handle(handle);
    finalize_lib_tscan();
}
```

## Build and Runtime Requirements

- Build/link requires `libTSCAN.lib`, and runtime requires `libTSCAN.dll`.
- In practice, dependent DLLs (for example `libTSH.dll`) may also be required by `libTSCAN.dll`.
- For reliability, provide the full vendor runtime bundle for the matching architecture (x64/x86).
- At runtime, ensure required DLLs are in the executable directory or in `PATH`.
- See `docs/guides/libtscan-setup-guide.md` for setup details.

## Related Crates

- [canlink-hal](https://crates.io/crates/canlink-hal) - HAL abstraction
- [canlink-tscan](https://crates.io/crates/canlink-tscan) - Safe LibTSCAN backend
- [canlink-cli](https://crates.io/crates/canlink-cli) - CLI tool

## Documentation

- [API docs](https://docs.rs/canlink-tscan-sys)

## License

MIT OR Apache-2.0

<a id="zh"></a>

[中文](#zh) | [English](#en)

CANLink TSCan Sys 提供 LibTSCAN C API 的底层 Rust FFI 绑定（不安全接口）。

## 警告

此 crate 直接暴露 C 函数。若需要安全、高层 API，请使用 `canlink-tscan`。

## 验证范围

当前仓库的实机回归仅覆盖同星 / TOSUN 相关硬件。LibTSCAN 文档列出的其他设备类型尚未逐项验证。

## 平台支持

- Windows 10/11 x64（已验证）
- 厂商包可能包含 Windows x86 / Linux 相关库，但这些目标尚未在本项目验证
- 需要 LibTSCAN 运行库（最低要求 `libTSCAN.dll` + `libTSCAN.lib`，且可能需要依赖 DLL，如 `libTSH.dll`）
- 建议使用厂商提供的完整运行库目录
- 本项目不分发 LibTSCAN 文件

## 安装

```toml
[dependencies]
canlink-tscan-sys = "0.3.0"
```

## 基础用法

```rust,no_run
use canlink_tscan_sys::*;
use std::ptr;

unsafe {
    initialize_lib_tscan(true, false, true);

    let mut device_count = 0;
    tscan_scan_devices(&mut device_count);

    let mut handle = 0;
    tscan_connect(ptr::null(), &mut handle);

    // ... 使用设备 ...

    tscan_disconnect_by_handle(handle);
    finalize_lib_tscan();
}
```

## 构建与运行要求

- 构建/链接需要 `libTSCAN.lib`，运行时需要 `libTSCAN.dll`。
- 实际运行中，`libTSCAN.dll` 可能还依赖其他 DLL（例如 `libTSH.dll`）。
- 为保证稳定性，建议按目标位数（x64/x86）提供厂商完整运行库目录。
- 运行时确保所需 DLL 位于可执行文件目录或 `PATH` 中。
- 具体配置参考 `docs/guides/libtscan-setup-guide.md`。

## 相关包

- [canlink-hal](https://crates.io/crates/canlink-hal) - HAL 抽象层
- [canlink-tscan](https://crates.io/crates/canlink-tscan) - 安全的 LibTSCAN 后端
- [canlink-cli](https://crates.io/crates/canlink-cli) - 命令行工具

## 文档

- [API 文档](https://docs.rs/canlink-tscan-sys)

## 许可证

MIT OR Apache-2.0
