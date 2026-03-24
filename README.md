# CANLink-RS

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)

> **重要提示**：`canlink-tscan` 依赖厂商 `LibTSCAN` 运行库（本项目不分发），当前仅在 Windows 环境完成验证。获取与配置见 `docs/guides/libtscan-setup-guide.md`。

**一句话结论**：`CANLink-RS` 当前唯一已落地的真实硬件后端是 `LibTSCAN`。根据 `TSMaster/LibTSCAN` 文档，这条后端路径具备识别多种设备类型的能力；但本项目目前仅对同星 / TOSUN 相关硬件完成了实机接入与回归，其他文档枚举设备类型尚未逐项验证。

## 当前支持范围

### 后端实现状态

| 类型 | 当前状态 | 说明 |
|---|---|---|
| Mock 后端 | 已实现 | 无需硬件，用于开发、测试、CI、回归 |
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

`CANLink-RS` 是一个围绕 `TSMaster/LibTSCAN` 构建的 Rust CAN 接入层，保留了 HAL 抽象的工程价值：

- 统一 API
- Mock 测试能力
- 后续扩展能力

但当前对外应该理解为：

- 一个 `LibTSCAN` 真实硬件后端
- 一个 `Mock` 测试后端
- 一个允许未来新增其他原生后端的 HAL 架构

而不是“多后端、多厂商、可自由切换任意 CAN 设备”的成熟平台。

## 适配策略

当前仓库的适配策略很明确：

1. 优先把 `LibTSCAN` 这一条真实硬件链路做稳定。
2. 对 `LibTSCAN` 已暴露的设备类型，优先复用同一个后端，而不是立刻再拆新的 crate。
3. 是否可以直接复用，以能力校验和硬件回归结果为准。
4. 在没有实际实现和验证之前，不对其他设备类型写“已兼容”承诺。
5. 如果后续需要接入不经过 `LibTSCAN` 的厂商生态，则以新的独立 backend crate 落地，而不是继续扩大 `canlink-tscan` 的职责。

## 架构

```text
┌─────────────────────────────────────────────────────────┐
│                   应用程序层                            │
│             (你的 CAN 业务或工具代码)                  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│                    canlink-hal                          │
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
| [canlink-mock](canlink-mock/) | Mock 后端，服务于测试与无硬件开发 | 当前可用 |
| [canlink-tscan](canlink-tscan/) | TSMaster 真实硬件后端 | 当前可用 |
| [canlink-tscan-sys](canlink-tscan-sys/) | LibTSCAN 原始 FFI 绑定 | 当前可用 |
| [canlink-cli](canlink-cli/) | 命令行工具，用于调试、验证与演示 | 当前可用 |

## 运行前提

### Mock 模式

不需要硬件，也不需要安装 TSMaster。

### 真实硬件模式

当前真实硬件模式已验证环境：

- Windows 环境（已验证；Linux/macOS 未验证）
- 可用且版本匹配的 `libTSCAN.dll` 与 `libTSCAN.lib`
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

    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    backend.send_message(&msg)?;

    if let Some(received) = backend.receive_message()? {
        println!("ID={:X}, data={:?}", received.id(), received.data());
    }

    Ok(())
}
```

### 3. 切换到 LibTSCAN 后端

```rust
use canlink_hal::{BackendConfig, BackendRegistry};
use canlink_tscan::TSCanBackendFactory;
use std::sync::Arc;

fn main() {
    let registry = BackendRegistry::global();
    registry
        .register(Arc::new(TSCanBackendFactory::new()))
        .unwrap();

    let config = BackendConfig::new("tscan");
    let mut backend = registry.create_backend(&config).unwrap();

    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();
}
```

### 4. 使用 CLI

```bash
cargo install --path canlink-cli

canlink list
canlink info tscan
canlink send tscan 0 0x123 01 02 03 04
canlink receive tscan 0 --count 1
```

> 真实硬件模式已在 Windows + LibTSCAN 运行库环境验证，其他平台尚未验证。


## 文档

- [canlink-hal](canlink-hal/)
- [canlink-mock](canlink-mock/)
- [canlink-tscan](canlink-tscan/)
- [canlink-cli](canlink-cli/)
- [docs/guides/hardware-test-guide.md](docs/guides/hardware-test-guide.md)
- [docs/reports/performance-benchmark-report.md](docs/reports/performance-benchmark-report.md)
- [docs/release/release-guide.md](docs/release/release-guide.md)

## 常用命令

```bash
scripts\check.bat
cargo test --workspace
cargo doc --workspace --no-deps
cargo install --path canlink-cli
```

## 许可证

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

## 总结

如果你现在只想知道一件事，那就是：

`CANLink-RS` 当前支持 `Mock`，以及 `LibTSCAN` 这一条真实硬件链路；当前已完成实机回归的是同星 / TOSUN 相关硬件。文档中枚举的其他 `LibTSCAN` 设备类型仍需逐项验证，而 SocketCAN、PEAK 原生后端、Vector 原生后端等其他独立硬件生态当前尚未实现。
