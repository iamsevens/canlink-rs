# CANLink CLI

[![Crates.io](https://img.shields.io/crates/v/canlink-cli.svg)](https://crates.io/crates/canlink-cli)
[![Documentation](https://docs.rs/canlink-cli/badge.svg)](https://docs.rs/canlink-cli)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)

<a id="en"></a>

[English](#en) | [中文](#zh)

Command-line interface for interacting with CAN hardware through the CANLink HAL.

## Features

- List available backends
- Query backend capabilities
- Send CAN messages (single-shot or periodic)
- Receive CAN messages
- Validate configuration files
- Human-readable and JSON output

## Installation

### From Source

```bash
cargo install --path canlink-cli
```

### From Crates.io

```bash
cargo install canlink-cli
```

## Requirements

Real hardware usage requires:

- Windows
- LibTSCAN runtime (TSMaster installation or a standalone LibTSCAN bundle)

## Quick Start

```bash
# List available backends
canlink list

# Query backend capabilities
canlink info tscan

# Send a CAN message
canlink send tscan 0 0x123 01 02 03 04

# Receive messages
canlink receive tscan 0 --count 5
```

## Configuration File

Create a `canlink.toml` file:

```toml
[backend]
backend_name = "tscan"
retry_count = 3
retry_interval_ms = 1000
```

## JSON Output

All commands support JSON output with the `--json` flag:

```bash
canlink --json info tscan
```

## Exit Codes

- `0`: Success
- `2`: Backend not found
- `3`: Backend error
- `4`: Configuration error
- `5`: Invalid argument
- `6`: I/O error
- `7`: Parse error
- `8`: Timeout
- `9`: No messages received

## Related Crates

- [canlink-hal](https://crates.io/crates/canlink-hal) - Core HAL
- [canlink-tscan-sys](https://crates.io/crates/canlink-tscan-sys) - LibTSCAN FFI bindings
- [canlink-tscan](https://crates.io/crates/canlink-tscan) - LibTSCAN backend

## Documentation

- [API docs](https://docs.rs/canlink-cli)

## License

MIT OR Apache-2.0

<a id="zh"></a>

[中文](#zh) | [English](#en)

CANLink HAL 的命令行工具，用于与 CAN 硬件交互。

## 功能

- 列出可用后端
- 查询后端能力
- 发送 CAN 消息（单次或周期）
- 接收 CAN 消息
- 校验配置文件
- 人类可读与 JSON 输出

## 安装

### 从源码安装

```bash
cargo install --path canlink-cli
```

### 从 Crates.io 安装

```bash
cargo install canlink-cli
```

## 环境要求

真实硬件模式需要：

- Windows
- LibTSCAN 运行库（完整安装 TSMaster 或独立 LibTSCAN 包）

## 快速开始

```bash
# 列出可用后端
canlink list

# 查询后端能力
canlink info tscan

# 发送 CAN 消息
canlink send tscan 0 0x123 01 02 03 04

# 接收消息
canlink receive tscan 0 --count 5
```

## 配置文件

创建 `canlink.toml`：

```toml
[backend]
backend_name = "tscan"
retry_count = 3
retry_interval_ms = 1000
```

## JSON 输出

所有命令支持 `--json`：

```bash
canlink --json info tscan
```

## 退出码

- `0`: 成功
- `2`: 未找到后端
- `3`: 后端错误
- `4`: 配置错误
- `5`: 参数无效
- `6`: I/O 错误
- `7`: 解析错误
- `8`: 超时
- `9`: 未收到消息

## 相关包

- [canlink-hal](https://crates.io/crates/canlink-hal) - 核心 HAL
- [canlink-tscan-sys](https://crates.io/crates/canlink-tscan-sys) - LibTSCAN FFI 绑定
- [canlink-tscan](https://crates.io/crates/canlink-tscan) - LibTSCAN 后端

## 文档

- [API 文档](https://docs.rs/canlink-cli)

## 许可证

MIT OR Apache-2.0
