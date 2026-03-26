# CANLink TSCan Backend

[![Crates.io](https://img.shields.io/crates/v/canlink-tscan.svg)](https://crates.io/crates/canlink-tscan)
[![Documentation](https://docs.rs/canlink-tscan/badge.svg)](https://docs.rs/canlink-tscan)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)

<a id="en"></a>

[English](#en) | [中文](#zh)

CANLink TSCan Backend is the real-hardware backend built on LibTSCAN. It implements `CanBackend` from `canlink-hal` and connects to CAN hardware that LibTSCAN can recognize.

## Validation Scope

- Real-hardware regression in this repository is limited to TOSUN-related devices.
- LibTSCAN headers and API docs enumerate multiple device types on the same backend path, but they are not individually validated here.
- If a future vendor SDK path is required, it should land as a separate backend crate rather than extending `canlink-tscan`.

## Documented Device Types (Not Yet Individually Validated)

The list below is derived from official `TSMaster/LibTSCAN` headers and API docs (see `docs/vendor/tsmaster/README.md`). It indicates device types visible on the same backend path, not a compatibility promise for each device.

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

## Requirements

- Windows 10/11 x64 (validated; other platforms not validated)
- Vendor package may include Windows x86 / Linux artifacts, but those targets are not validated in this project
- LibTSCAN runtime (minimum `libTSCAN.dll` + `libTSCAN.lib`; dependent DLLs such as `libTSH.dll` may also be required)
- Full vendor runtime bundle is recommended
- LibTSCAN is not distributed by this project

## Installation

```toml
[dependencies]
canlink-hal = "0.3.0"
canlink-tscan = "0.3.0"
```

## Setup LibTSCAN

1. Download the TSMaster API bundle or install TSMaster to obtain LibTSCAN.
2. For Windows x64, prepare the matching vendor runtime bundle (minimum `libTSCAN.dll` + `libTSCAN.lib`, and usually dependency DLLs like `libTSH.dll`).
3. Configure runtime paths by following `docs/guides/libtscan-setup-guide.md`.

> This backend depends on the LibTSCAN runtime, not the TSMaster GUI itself.

## Quick Start

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_tscan::TSCanBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = TSCanBackend::new();
    let config = BackendConfig::new("tscan");

    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    backend.send_message(&msg)?;

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
```

## TSCan Daemon Workaround (Vendor Bug)

To isolate a known vendor DLL hang in `DISCONNECT_*`, `canlink-tscan` supports an out-of-process daemon path and enables it by default. When the vendor provides a stable fix, this workaround can be removed or downgraded in a future release.

### `canlink-tscan.toml`

```toml
use_daemon = true
request_timeout_ms = 2000
disconnect_timeout_ms = 3000
restart_max_retries = 3
recv_timeout_ms = 0
# daemon_path = "C:/path/to/canlink-tscan-daemon.exe"
```

### Config Priority

Priority from high to low:

1. `BackendConfig.parameters`
2. `canlink-tscan.toml`
3. Built-in defaults

When `use_daemon = false`, the backend falls back to direct DLL calls.

## Related Crates

- [canlink-hal](https://crates.io/crates/canlink-hal) - Core HAL
- [canlink-tscan-sys](https://crates.io/crates/canlink-tscan-sys) - LibTSCAN FFI bindings
- [canlink-cli](https://crates.io/crates/canlink-cli) - CLI tool

## Documentation

- [API docs](https://docs.rs/canlink-tscan)

## License

MIT OR Apache-2.0

<a id="zh"></a>

[中文](#zh) | [English](#en)

CANLink TSCan Backend 是基于 LibTSCAN 的真实硬件后端，实现了 `canlink-hal` 的 `CanBackend`，用于连接 LibTSCAN 可识别的 CAN 硬件。

## 验证范围

- 当前仓库的实机回归仅覆盖同星 / TOSUN 相关硬件。
- LibTSCAN 头文件与 API 文档列出了更多设备类型，但尚未逐项验证。
- 若未来需要走厂商原生 SDK 路径，应以新的独立 backend crate 落地，而不是扩展 `canlink-tscan`。

## 文档可见但未专项验证的设备类型

下表依据官方 `TSMaster/LibTSCAN` 头文件与 API 文档整理（来源见 `docs/vendor/tsmaster/README.md`）。它表示同一后端路径下文档可见的设备类型，并不代表逐项兼容承诺。

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

## 环境要求

- Windows 10/11 x64（已验证；其他平台未验证）
- 厂商包可能包含 Windows x86 / Linux 相关库，但这些目标在本项目中尚未验证
- LibTSCAN 运行库（最低要求 `libTSCAN.dll` + `libTSCAN.lib`，且可能需要依赖 DLL，如 `libTSH.dll`）
- 建议使用厂商提供的完整运行库目录
- 本项目不分发 LibTSCAN 文件

## 安装

```toml
[dependencies]
canlink-hal = "0.3.0"
canlink-tscan = "0.3.0"
```

## 安装与配置 LibTSCAN

1. 下载 TSMaster API 包或安装 TSMaster 获取 LibTSCAN。
2. Windows x64 请准备匹配版本的厂商运行库目录（最低要求 `libTSCAN.dll` + `libTSCAN.lib`，通常还需要 `libTSH.dll` 等依赖 DLL）。
3. 参考 `docs/guides/libtscan-setup-guide.md` 配置运行库路径。

> 本后端依赖的是 LibTSCAN 运行库，而不是 TSMaster GUI 本身。

## 快速开始

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_tscan::TSCanBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = TSCanBackend::new();
    let config = BackendConfig::new("tscan");

    backend.initialize(&config)?;
    backend.open_channel(0)?;

    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    backend.send_message(&msg)?;

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
```

## TSCan 守护进程规避方案（厂商 DLL 问题）

为隔离 `DISCONNECT_*` 调用的厂商 DLL 卡死问题，`canlink-tscan` 支持独立守护进程路径，并默认启用。若厂商提供稳定修复，此规避可在后续版本移除或降级。

### `canlink-tscan.toml`

```toml
use_daemon = true
request_timeout_ms = 2000
disconnect_timeout_ms = 3000
restart_max_retries = 3
recv_timeout_ms = 0
# daemon_path = "C:/path/to/canlink-tscan-daemon.exe"
```

### 配置优先级

从高到低：

1. `BackendConfig.parameters`
2. `canlink-tscan.toml`
3. 内置默认值

当 `use_daemon = false` 时，后端会回退到直接 DLL 调用路径。

## 相关包

- [canlink-hal](https://crates.io/crates/canlink-hal) - 核心 HAL
- [canlink-tscan-sys](https://crates.io/crates/canlink-tscan-sys) - LibTSCAN FFI 绑定
- [canlink-cli](https://crates.io/crates/canlink-cli) - 命令行工具

## 文档

- [API 文档](https://docs.rs/canlink-tscan)

## 许可证

MIT OR Apache-2.0
