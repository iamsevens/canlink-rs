# CANLink HAL

[![Crates.io](https://img.shields.io/crates/v/canlink-hal.svg)](https://crates.io/crates/canlink-hal)
[![Documentation](https://docs.rs/canlink-hal/badge.svg)](https://docs.rs/canlink-hal)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)

<a id="en"></a>

[English](#en) | [中文](#zh)

CANLink HAL is the core hardware abstraction layer of CANLink. It defines the `CanBackend` trait, common message types, and the backend registry used by real hardware backends.

## Scope

This crate is hardware-agnostic. The only real-hardware backend currently landed in this repository is `canlink-tscan` (LibTSCAN). The HAL is designed to host additional native backends in the future.

## Features

- Unified `CanBackend` trait
- Type-safe `CanMessage` / `CanId`
- Backend registry for runtime discovery
- Capability query via `HardwareCapability`
- Shared config and error types across backends

## Installation

```toml
[dependencies]
canlink-hal = "0.3.0"
```

## Quick Start

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_tscan::TSCanBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = TSCanBackend::new();
    let config = BackendConfig::new("tscan");

    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
```

Note: For hardware-free tests inside this repository, a `canlink-mock` backend exists but is not published to crates.io.

## Related Crates

- [canlink-tscan-sys](https://crates.io/crates/canlink-tscan-sys) - LibTSCAN FFI bindings
- [canlink-tscan](https://crates.io/crates/canlink-tscan) - LibTSCAN backend (real hardware)
- [canlink-cli](https://crates.io/crates/canlink-cli) - Command-line tool

## Documentation

- [API docs](https://docs.rs/canlink-hal)

## License

MIT OR Apache-2.0

<a id="zh"></a>

[中文](#zh) | [English](#en)

CANLink HAL 是 CANLink 的核心硬件抽象层，定义了 `CanBackend` trait、通用消息类型以及后端注册表，用于承载真实硬件后端。

## 定位

本 crate 与具体硬件无关。当前仓库唯一已落地的真实硬件后端是 `canlink-tscan`（基于 LibTSCAN）。HAL 设计允许未来新增其他原生后端。

## 特性

- 统一的 `CanBackend` trait
- 类型安全的 `CanMessage` / `CanId`
- 后端注册表用于运行时发现
- 通过 `HardwareCapability` 查询能力
- 统一的配置与错误类型

## 安装

```toml
[dependencies]
canlink-hal = "0.3.0"
```

## 快速开始

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_tscan::TSCanBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = TSCanBackend::new();
    let config = BackendConfig::new("tscan");

    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
```

说明：本仓库内部测试还使用 `canlink-mock`（未发布到 crates.io）。

## 相关包

- [canlink-tscan-sys](https://crates.io/crates/canlink-tscan-sys) - LibTSCAN FFI 绑定
- [canlink-tscan](https://crates.io/crates/canlink-tscan) - LibTSCAN 真实硬件后端
- [canlink-cli](https://crates.io/crates/canlink-cli) - 命令行工具

## 文档

- [API 文档](https://docs.rs/canlink-hal)

## 许可证

MIT OR Apache-2.0
