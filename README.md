# CANLink

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)

<a id="zh"></a>

[中文](#zh) | [English](#en)

> **重要提示**：`canlink-tscan` 依赖厂商 `LibTSCAN` 运行库（本项目不分发），当前仅在 Windows 环境完成验证。获取与配置见 `docs/guides/libtscan-setup-guide.md`。

**一句话结论**：CANLink（Rust 实现）当前唯一已落地的真实硬件后端是 `LibTSCAN`。根据 `TSMaster/LibTSCAN` 文档，这条后端路径具备识别多种设备类型的能力；但本项目目前仅对同星 / TOSUN 相关硬件完成了实机接入与回归，其他文档枚举设备类型尚未逐项验证。

## 当前支持范围

### 后端实现状态

| 类型 | 当前状态 | 说明 |
|---|---|---|
| Mock 后端 | 已实现 | 无需硬件，用于开发、测试、CI、回归（未发布到 crates.io） |
| LibTSCAN 后端（`canlink-tscan`） | 已实现 | 当前唯一已落地的真实硬件接入路径，已在 Windows 环境验证（其他平台未验证） |
| SocketCAN 原生后端 | 未实现 | 当前仓库没有对应后端 |
| PEAK / PCAN 原生后端 | 未实现 | 当前仓库没有对应后端 |
| Vector / VN 原生后端 | 未实现 | 当前仓库没有对应后端 |
| 其他厂商原生后端 | 未实现 | 架构允许后续新增，但当前仓库没有对应后端 |

### 已实际接入并验证过的硬件

| 硬件范围 | 当前状态 | 说明 |
|---|---|---|
| 同星 / TOSUN 相关硬件（通过 `LibTSCAN`） | 已验证 | 当前仓库已完成实机接入与回归验证的真实硬件范围 |

### 文档显示可通过 `LibTSCAN` 识别或接入，但尚未专项验证的设备类型

下表依据官方 `TSMaster/LibTSCAN` 头文件与 API 文档整理，来源与获取方式见 `docs/vendor/tsmaster/README.md`。它表示“同一后端路径下文档可见的设备类型”，不表示“当前仓库已经逐项完成兼容性承诺”。

| 设备类型 | 文档枚举值 | 当前状态 |
|---|---|---|
| 同星 TCP 设备 | `TS_TCP_DEVICE` | 文档可见，未专项验证 |
| 同星扩展 USB 设备 | `TS_USB_DEVICE_EX` | 文档可见，未专项验证 |
| 同星无线 OBD | `TS_WIRELESS_OBD` | 文档可见，未专项验证 |
| 同星 TC1005 系列 | `TS_TC1005_DEVICE` | 文档可见，未专项验证 |
| Vector XL | `XL_USB_DEVICE` | 文档可见，未专项验证 |
| PEAK / PCAN | `PEAK_USB_DEVICE` | 文档可见，未专项验证 |
| Kvaser | `KVASER_USB_DEVICE` | 文档可见，未专项验证 |
| ZLG | `ZLG_USB_DEVICE` | 文档可见，未专项验证 |
| Intrepid / Vehicle Spy 生态设备 | `ICS_USB_DEVICE` | 文档可见，未专项验证 |
| IXXAT | `IXXAT_USB_DEVICE` | 文档可见，未专项验证 |
| CANable | `CANABLE_USB_DEVICE` | 文档可见，未专项验证 |

## 容易混淆的点

这个项目现在的情况不是“已经逐项验证了多家硬件厂商”，而是：

- 当前唯一已落地的真实硬件接入路径只有一条：`LibTSCAN`
- 当前已完成实机接入与回归的范围是同星 / TOSUN 相关硬件
- 文档里出现的多种设备类型，说明 `LibTSCAN` 这条后端路径可能覆盖更广的硬件范围
- 这些设备即使可通过 `LibTSCAN` 暴露，也仍然属于同一个后端能力范围，不等于仓库已经有多个厂商原生后端
- 整体架构允许未来新增不经过 `LibTSCAN` 的原生后端，例如其他厂商 SDK / DLL 路径；但这些后端当前尚未实现

换句话说：

- “当前是否已经有多个真实硬件后端” 的答案：`不是，当前只有 LibTSCAN`
- “当前是否已经逐项验证了文档列出的所有设备类型” 的答案：`不是，目前只验证了同星 / TOSUN 相关硬件`
- “同一 LibTSCAN 后端下是否可能覆盖多种设备类型” 的答案：`是，文档显示可能可以，但多数类型尚未专项验证`

## 项目定位

CANLink 是一个围绕 `TSMaster/LibTSCAN` 构建的 CAN 接入层，保留了 HAL 抽象的工程价值：

- 统一 API
- Mock 测试能力
- 后续扩展能力

但当前对外应该理解为：

- 一个 `LibTSCAN` 真实硬件后端
- 一个 `Mock` 测试后端（内部使用）
- 一个允许未来新增其他原生后端的 HAL 架构

而不是“多后端、多厂商、可自由切换任意 CAN 设备”的成熟平台。

## 适配策略

1. 优先把 `LibTSCAN` 这一条真实硬件链路做稳定。
2. 对 `LibTSCAN` 已暴露的设备类型，优先复用同一个后端，而不是立刻再拆新的 crate。
3. 当出现新的厂商 SDK 路径时，再以新的独立 backend crate 落地。

## 架构

```
┌─────────────────────────────────────────────────────────┐
│               统一的 CAN 抽象接口与模型                │
└─────────────────────────────────────────────────────────┘
                          │
             ┌────────────┴────────────┐
             ▼                         ▼
┌──────────────────────┐   ┌────────────────────────────┐
│    canlink-mock      │   │        canlink-tscan       │
│   无硬件测试后端     │   │      LibTSCAN 后端         │
└──────────────────────┘   └────────────────────────────┘
                                       │
                                       ▼
                         ┌────────────────────────────┐
                         │      canlink-tscan-sys     │
                         │ LibTSCAN DLL/Lib FFI 绑定  │
                         └────────────────────────────┘
```

## 工作区组件

| Crate | 作用 | 状态 |
|---|---|---|
| [canlink-hal](canlink-hal/) | 核心抽象层、消息模型、注册表、通用接口 | 当前主入口 |
| [canlink-mock](canlink-mock/) | Mock 后端，服务于测试与无硬件开发 | 当前可用（未发布到 crates.io） |
| [canlink-tscan](canlink-tscan/) | TSMaster 真实硬件后端 | 当前可用 |
| [canlink-tscan-sys](canlink-tscan-sys/) | LibTSCAN 原始 FFI 绑定 | 当前可用 |
| [canlink-cli](canlink-cli/) | 命令行工具，用于调试、验证与演示 | 当前可用 |

### Crate Map（已发布）

| Crate | 角色 | 依赖关系 |
|---|---|---|
| `canlink-hal` | 核心 HAL | 无（基础层） |
| `canlink-tscan-sys` | LibTSCAN FFI 绑定 | 无（FFI 层） |
| `canlink-tscan` | LibTSCAN 后端 | 依赖 `canlink-hal` + `canlink-tscan-sys` |
| `canlink-cli` | 命令行工具 | 依赖 `canlink-hal` + `canlink-tscan` |

## 运行前提

### Mock 模式

不需要硬件，也不需要安装 TSMaster。

### 真实硬件模式

当前真实硬件模式已验证环境：

- Windows 环境（已验证；Linux/macOS 未验证）
- 可用且版本匹配的 LibTSCAN 运行库（最低要求 `libTSCAN.dll` + `libTSCAN.lib`，通常还需 `libTSH.dll` 等依赖 DLL）
- 建议使用厂商提供的完整运行库目录（按目标位数 x64/x86 匹配）
- 可通过完整安装 `TSMaster` 获得运行库，也可单独提供匹配的 `LibTSCAN bundle`（需遵守厂商许可）

`canlink-tscan-sys` 支持以下常见方式：

- 默认从常见安装位置查找 `TSMaster/LibTSCAN`
- 通过 `TSMASTER_HOME` 指定安装目录
- 通过 `CANLINK_TSCAN_BUNDLE_DIR` 指定 DLL/Lib 所在目录

这意味着项目依赖的是 `LibTSCAN` 运行库，而不是必须依赖 `TSMaster` 图形界面本身。

## 快速开始

### 1. 构建工作区

```bash
git clone https://github.com/iamsevens/canlink-rs.git
cd canlink-rs
cargo build --workspace
```

### 2. 使用 Mock 后端

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;

    if let Some(msg) = backend.receive_message()? {
        println!("收到: ID={:X}, 数据={:?}", msg.id(), msg.data());
    }

    backend.close_channel(0)?;
    backend.close()?;

    Ok(())
}
```

### 3. 切换到 LibTSCAN 后端

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

> 真实硬件模式已在 Windows + LibTSCAN 运行库环境验证，其他平台尚未验证。

## 文档

- [用户指南](docs/user-guide.md)
- [API 参考](docs/api-reference.md)
- [LibTSCAN 获取与配置](docs/guides/libtscan-setup-guide.md)
- [硬件测试指南](docs/guides/hardware-test-guide.md)

## 常用命令

```bash
# 全量测试
cargo test --workspace

# 质量检查
scripts\check.bat

# 硬件回归（需要连接 LibTSCAN 硬件）
scripts\tscan_hw_regression.bat
```

## 许可证

MIT OR Apache-2.0

## 总结

CANLink 当前是以 `LibTSCAN` 为核心的真实硬件接入路径，其他厂商原生后端尚未实现。对外应理解为：一个稳定的 HAL 基础 + 一条真实硬件后端 + 可扩展的架构。

<a id="en"></a>

[English](#en) | [中文](#zh)

> **Important**: `canlink-tscan` depends on the vendor `LibTSCAN` runtime (not distributed by this project). It has only been validated on Windows. See `docs/guides/libtscan-setup-guide.md` for setup.

**One-line summary**: CANLink currently ships exactly one real-hardware backend via `LibTSCAN`. According to the `TSMaster/LibTSCAN` documentation, this backend path can recognize multiple device types, but only TOSUN-related hardware has been physically validated so far.

## Current Support Scope

### Backend Status

| Type | Status | Notes |
|---|---|---|
| Mock backend | Implemented | No hardware required; for development/testing (not published to crates.io) |
| LibTSCAN backend (`canlink-tscan`) | Implemented | The only landed real-hardware backend; validated on Windows only |
| SocketCAN native backend | Not implemented | Not available in this repo |
| PEAK / PCAN native backend | Not implemented | Not available in this repo |
| Vector / VN native backend | Not implemented | Not available in this repo |
| Other vendor native backends | Not implemented | Architecture allows future additions |

### Hardware Verified in This Repo

| Hardware Scope | Status | Notes |
|---|---|---|
| TOSUN-related hardware (via `LibTSCAN`) | Validated | Real-hardware regression has been completed on this scope |

### Device Types Listed by LibTSCAN Docs (Not Yet Individually Validated)

The table below is derived from the official `TSMaster/LibTSCAN` headers and API docs (see `docs/vendor/tsmaster/README.md`). It indicates device types visible on the same backend path, not a compatibility promise for each device.

| Device Type | Enum Value | Status |
|---|---|---|
| TOSUN TCP device | `TS_TCP_DEVICE` | Documented, not validated |
| TOSUN USB EX device | `TS_USB_DEVICE_EX` | Documented, not validated |
| TOSUN Wireless OBD | `TS_WIRELESS_OBD` | Documented, not validated |
| TOSUN TC1005 series | `TS_TC1005_DEVICE` | Documented, not validated |
| Vector XL | `XL_USB_DEVICE` | Documented, not validated |
| PEAK / PCAN | `PEAK_USB_DEVICE` | Documented, not validated |
| Kvaser | `KVASER_USB_DEVICE` | Documented, not validated |
| ZLG | `ZLG_USB_DEVICE` | Documented, not validated |
| Intrepid / Vehicle Spy devices | `ICS_USB_DEVICE` | Documented, not validated |
| IXXAT | `IXXAT_USB_DEVICE` | Documented, not validated |
| CANable | `CANABLE_USB_DEVICE` | Documented, not validated |

## Common Confusions

This project is **not** “multiple vendor backends already validated.” The current reality is:

- There is exactly one landed real-hardware backend path: `LibTSCAN`
- The only validated physical hardware in this repo is TOSUN-related devices
- The presence of many device types in docs means the same backend path may cover more hardware
- That does **not** mean multiple native vendor backends exist
- The architecture allows new native backends later, but they are not implemented yet

## Project Positioning

CANLink is a CAN access layer built around `TSMaster/LibTSCAN` while preserving a clean HAL abstraction:

- Unified API
- Mock backend for testing
- Future extensibility

At present, treat CANLink as:

- One real-hardware backend (`LibTSCAN`)
- One internal mock backend
- An extensible HAL foundation

## Adaptation Strategy

1. Stabilize the `LibTSCAN` hardware path first.
2. Reuse the same backend for additional LibTSCAN-visible device types.
3. Add new native backends only when a separate vendor SDK is required.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│        Unified CAN abstraction and data model           │
└─────────────────────────────────────────────────────────┘
                          │
             ┌────────────┴────────────┐
             ▼                         ▼
┌──────────────────────┐   ┌────────────────────────────┐
│    canlink-mock      │   │        canlink-tscan       │
│   Mock backend       │   │      LibTSCAN backend      │
└──────────────────────┘   └────────────────────────────┘
                                       │
                                       ▼
                         ┌────────────────────────────┐
                         │      canlink-tscan-sys     │
                         │ LibTSCAN DLL/Lib FFI bind  │
                         └────────────────────────────┘
```

## Workspace Components

| Crate | Role | Status |
|---|---|---|
| [canlink-hal](canlink-hal/) | Core HAL, message model, registry, interfaces | Primary entry point |
| [canlink-mock](canlink-mock/) | Mock backend for tests | Available (not published to crates.io) |
| [canlink-tscan](canlink-tscan/) | TSMaster real-hardware backend | Available |
| [canlink-tscan-sys](canlink-tscan-sys/) | LibTSCAN raw FFI bindings | Available |
| [canlink-cli](canlink-cli/) | CLI tool for debugging and demos | Available |

### Crate Map (Published)

| Crate | Role | Dependency |
|---|---|---|
| `canlink-hal` | Core HAL | None (foundation) |
| `canlink-tscan-sys` | LibTSCAN FFI | None (FFI layer) |
| `canlink-tscan` | LibTSCAN backend | depends on `canlink-hal` + `canlink-tscan-sys` |
| `canlink-cli` | CLI tool | depends on `canlink-hal` + `canlink-tscan` |

## Prerequisites

### Mock Mode

No hardware and no TSMaster installation required.

### Real Hardware Mode

Validated environment so far:

- Windows (validated; Linux/macOS not validated)
- Matching LibTSCAN runtime bundle (minimum `libTSCAN.dll` + `libTSCAN.lib`, and usually dependent DLLs such as `libTSH.dll`)
- Full vendor runtime bundle is recommended for the target architecture (x64/x86)
- Runtime obtained via full TSMaster installation or a standalone LibTSCAN bundle (subject to vendor license)

`canlink-tscan-sys` supports these discovery paths:

- Common TSMaster install locations
- `TSMASTER_HOME` (install root)
- `CANLINK_TSCAN_BUNDLE_DIR` (DLL/LIB directory)

This means the project depends on the LibTSCAN runtime, not the TSMaster GUI itself.

## Quick Start

### 1. Build the workspace

```bash
git clone https://github.com/iamsevens/canlink-rs.git
cd canlink-rs
cargo build --workspace
```

### 2. Use the Mock backend

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");

    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;

    if let Some(msg) = backend.receive_message()? {
        println!("Received: ID={:X}, Data={:?}", msg.id(), msg.data());
    }

    backend.close_channel(0)?;
    backend.close()?;

    Ok(())
}
```

### 3. Switch to LibTSCAN backend

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

> Real-hardware mode has only been validated on Windows with LibTSCAN runtime.

## Documentation

- [User Guide](docs/user-guide.md)
- [API Reference](docs/api-reference.md)
- [LibTSCAN Setup](docs/guides/libtscan-setup-guide.md)
- [Hardware Test Guide](docs/guides/hardware-test-guide.md)

## Common Commands

```bash
# Full tests
cargo test --workspace

# Quality checks
scripts\check.bat

# Hardware regression (LibTSCAN hardware required)
scripts\tscan_hw_regression.bat
```

## License

MIT OR Apache-2.0

## Summary

CANLink currently delivers a stable HAL foundation plus a single real-hardware backend via `LibTSCAN`. Other native backends are not implemented yet but can be added when required.
