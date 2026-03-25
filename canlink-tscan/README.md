# CANLink TSCan Backend

[![Crates.io](https://img.shields.io/crates/v/canlink-tscan.svg)](https://crates.io/crates/canlink-tscan)
[![Documentation](https://docs.rs/canlink-tscan/badge.svg)](https://docs.rs/canlink-tscan)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)

**CANLink TSCan Backend** 是 CANLink-RS 当前唯一已落地的真实硬件后端实现，通过 `LibTSCAN` 库提供对其可识别 CAN 硬件的接入。

## 定位说明

- 当前版本聚焦 `TSMaster/LibTSCAN` 生态，优先保证这条后端链路的稳定性与可用性。
- 当前仓库已完成实机接入与回归验证的范围，是同星 / TOSUN 相关硬件。
- `TSMaster/LibTSCAN` 文档与头文件暴露了多种设备类型，因此这条后端路径理论上可能覆盖更广的硬件范围；但除已验证的同星 / TOSUN 相关硬件外，其余类型尚未专项验证。
- 如果未来需要支持不经过 `LibTSCAN` 的厂商 SDK / DLL 路径，会以新的独立 backend crate 落地，而不是继续扩大 `canlink-tscan` 的职责。

## 支持范围说明

| 范围 | 当前状态 | 说明 |
|---|---|---|
| CANLink-RS 当前唯一已落地的真实硬件后端 | 是 | `canlink-tscan` 是当前唯一已落地的真实硬件后端 |
| 已实机验证的硬件范围 | 同星 / TOSUN 相关硬件 | 当前仓库已完成实机接入与回归验证 |
| 文档可见但未专项验证的 `LibTSCAN` 设备类型 | 存在 | 见下表，不等于当前仓库已逐项承诺兼容 |
| 其他厂商原生后端 | 未实现于本 crate | 若未来需要，会以新的独立 backend crate 提供 |

### 文档可见但未专项验证的 `LibTSCAN` 设备类型

下表依据官方 `TSMaster/LibTSCAN` 头文件与 API 文档整理，来源与获取方式见 `../docs/vendor/tsmaster/README.md`：

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

## 特性

- 🔌 **LibTSCAN 硬件路径** - 通过统一 DLL 路径接入 `LibTSCAN` 可识别硬件
- 🚀 **高性能** - 直接 FFI 调用，最小化开销
- 🛡️ **类型安全** - Rust 封装的安全 API
- 📊 **完整功能** - 支持 CAN 2.0 和 CAN-FD
- 🔧 **易于集成** - 实现标准 CanBackend trait
- 🧪 **测试覆盖** - 完整的单元测试和集成测试

## 系统要求

- **操作系统**: 当前在 Windows 10/11 (x64) 完成验证
  - LibTSCAN 文档包含 Linux/macOS 相关库与示例，但本 crate 尚未在这些平台验证
- **硬件**: `LibTSCAN` 可识别的 CAN 硬件；当前已实机验证的是同星 / TOSUN 相关硬件
- **依赖**: LibTSCAN 运行库（Windows: libTSCAN.dll + libTSCAN.lib；其他平台未验证）

## 安装

### 1. 添加依赖

```toml
[dependencies]
canlink-hal = "0.3.0"
canlink-tscan = "0.3.0"
```

### 2. 安装 LibTSCAN

从 TSMaster 官网下载并安装 LibTSCAN（本项目不提供 LibTSCAN 文件，请按厂商许可自行获取）：

1. 下载 TSMaster API（推荐）或 TSMaster 安装包
   - TSMaster API 下载页：`https://www.tosunai.com/downloads/tsmaster-api/`
   - TSMaster 安装包下载页：`https://www.tosunai.com/en/downloads/`
2. 准备 `libTSCAN.dll` 与 `libTSCAN.lib`（x64）
3. 按 `docs/guides/libtscan-setup-guide.md` 配置运行库路径与环境变量

### 3. 连接硬件

连接可被 `LibTSCAN` 识别的硬件到计算机；当前仓库的实机验证路径基于同星 / TOSUN 相关硬件。

## 快速开始

### 基础使用

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_tscan::TSCanBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建后端
    let mut backend = TSCanBackend::new();

    // 初始化
    let config = BackendConfig::new("tscan");
    backend.initialize(&config)?;

    // 打开通道 0
    backend.open_channel(0)?;

    // 发送消息
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    backend.send_message(&msg)?;
    println!("消息已发送");

    // 接收消息
    if let Some(msg) = backend.receive_message()? {
        println!("收到消息: ID={:X}, 数据={:?}", msg.id(), msg.data());
    }

    // 清理
    backend.close_channel(0)?;
    backend.close()?;

    Ok(())
}
```

### 使用后端工厂

```rust
use canlink_hal::{BackendRegistry, BackendConfig};
use canlink_tscan::TSCanBackendFactory;
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 注册 TSCan 后端
    let registry = BackendRegistry::global();
    let factory = Arc::new(TSCanBackendFactory::new());
    registry.register(factory)?;

    // 创建后端实例
    let config = BackendConfig::new("tscan");
    let mut backend = registry.create_backend(&config)?;

    // 使用后端
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // ... 您的代码 ...

    Ok(())
}
```

## 功能示例

### 查询硬件能力

```rust
use canlink_hal::CanBackend;

let capability = backend.get_capability()?;

println!("设备信息:");
println!("  通道数: {}", capability.channel_count);
println!("  支持 CAN-FD: {}", capability.supports_canfd);
println!("  最大波特率: {} bps", capability.max_bitrate);
println!("  时间戳精度: {:?}", capability.timestamp_precision);
```

### 发送不同类型的消息

```rust
use canlink_hal::{CanMessage, CanId};

// 标准帧
let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
backend.send_message(&msg)?;

// 扩展帧
let msg = CanMessage::new_extended(0x12345678, &[0xAA, 0xBB, 0xCC])?;
backend.send_message(&msg)?;

// CAN-FD 消息 (如果硬件支持)
if capability.supports_canfd {
    let data = vec![0x42; 64];
    let msg = CanMessage::new_canfd(0x200, &data, false)?;
    backend.send_message(&msg)?;
}

// 远程帧
let msg = CanMessage::new_remote(CanId::Standard(0x456), 8)?;
backend.send_message(&msg)?;
```

### 接收消息

```rust
use std::time::Duration;

// 接收单条消息
if let Some(msg) = backend.receive_message()? {
    println!("ID: {:X}", msg.id());
    println!("数据: {:?}", msg.data());
    println!("时间戳: {:?}", msg.timestamp());
}

// 持续接收
loop {
    match backend.receive_message()? {
        Some(msg) => {
            println!("收到: ID={:X}, 数据={:?}", msg.id(), msg.data());
        }
        None => {
            // 没有消息，短暂等待
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
```

### 多通道操作

```rust
// 打开多个通道
backend.open_channel(0)?;
backend.open_channel(1)?;

// 在不同通道发送消息
let msg1 = CanMessage::new_standard(0x100, &[1, 2])?;
backend.send_message(&msg1)?;  // 发送到通道 0

let msg2 = CanMessage::new_standard(0x200, &[3, 4])?;
backend.send_message(&msg2)?;  // 发送到通道 1

// 关闭通道
backend.close_channel(0)?;
backend.close_channel(1)?;
```

## 配置选项

### BackendConfig

```rust
use canlink_hal::BackendConfig;

let mut config = BackendConfig::new("tscan");

// 设置重试次数
config.set_retry_count(3);

// 设置重试间隔 (毫秒)
config.set_retry_interval_ms(100);

// 设置超时 (毫秒)
config.set_timeout_ms(5000);
```

## 错误处理

```rust
use canlink_hal::CanError;

match backend.send_message(&msg) {
    Ok(_) => println!("发送成功"),
    Err(CanError::NotInitialized) => {
        eprintln!("错误: 后端未初始化");
    }
    Err(CanError::ChannelNotOpen { channel }) => {
        eprintln!("错误: 通道 {} 未打开", channel);
    }
    Err(CanError::SendFailed { reason }) => {
        eprintln!("发送失败: {}", reason);
    }
    Err(CanError::HardwareError { code, message }) => {
        eprintln!("硬件错误 {}: {}", code, message);
    }
    Err(e) => {
        eprintln!("未知错误: {:?}", e);
    }
}
```

## 性能

TSCan 后端提供高性能的硬件访问：

- **消息转换开销**: ~3 ns
- **FFI 调用开销**: ~100 ns
- **实际发送延迟**: 取决于硬件和总线负载

详见 [性能基准测试报告](../docs/reports/performance-benchmark-report.md)。

## 测试

### 单元测试

```bash
# 运行单元测试 (不需要硬件)
cargo test -p canlink-tscan --lib
```

### 硬件测试

```bash
# 运行硬件集成测试 (需要连接已验证的 LibTSCAN 硬件，当前为同星 / TOSUN 相关设备)
cargo test -p canlink-tscan --test backend_test
```

### 基准测试

```bash
# 运行性能基准测试
cargo bench -p canlink-tscan
```

## 故障排除

### 问题: "No TSMaster devices found"

**原因**: 未连接硬件或驱动未安装

**解决方案**:
1. 确认 TSMaster 设备已连接
2. 检查设备管理器中的驱动状态
3. 重新安装 TSMaster 驱动程序

### 问题: "DLL not found"

**原因**: libTSCAN.dll 不在系统路径中

**解决方案**:
1. 将 `libTSCAN.dll` 复制到应用程序目录
2. 或添加 DLL 路径到 PATH 环境变量
3. 或将 DLL 复制到 `C:\Windows\System32`

### 问题: "Hardware error: Bus-Off"

**原因**: CAN 总线错误过多，设备进入 Bus-Off 状态

**解决方案**:
1. 检查总线终端电阻 (120Ω)
2. 检查波特率配置是否正确
3. 检查总线连接是否正常
4. 重新初始化设备

### 问题: "Channel not open"

**原因**: 尝试在未打开的通道上操作

**解决方案**:
```rust
// 确保在操作前打开通道
backend.open_channel(0)?;
```

## 硬件兼容性

当前应这样理解本 crate 的硬件兼容性：

- 已完成实机回归验证的是同星 / TOSUN 相关硬件。
- `LibTSCAN` 文档暴露了更多设备类型，因此同一后端路径理论上可能支持更广的硬件范围。
- 除已验证范围外，其他设备类型当前仍属于“文档可见但未专项验证”状态，不应解读为已逐型号兼容承诺。

## 限制

- **平台**: 当前仅在 Windows (x64) 验证，其他平台未验证
- **依赖**: 需要 LibTSCAN 动态库
- **硬件**: 需要物理 CAN 硬件与 `LibTSCAN` 运行环境；当前已验证的是同星 / TOSUN 相关硬件
- **并发**: 单个后端实例不是线程安全的

## 架构

```
┌─────────────────────────────────────┐
│      应用程序代码                    │
└─────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────┐
│      TSCanBackend                   │
│   (Rust 安全封装)                    │
└─────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────┐
│    canlink-tscan-sys                │
│      (FFI 绑定)                      │
└─────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────┐
│      libTSCAN.dll                   │
│   (LibTSCAN 运行时)                  │
└─────────────────────────────────────┘
                │
                ▼
┌─────────────────────────────────────┐
│   LibTSCAN 可识别硬件                │
└─────────────────────────────────────┘
```

## 相关包

- [canlink-hal](../canlink-hal/) - 核心硬件抽象层
- [canlink-tscan-sys](../canlink-tscan-sys/) - LibTSCAN FFI 绑定
- [canlink-mock](../canlink-mock/) - Mock 测试后端
- [canlink-cli](../canlink-cli/) - 命令行工具

## 示例

查看 `examples/` 目录获取更多示例：

- `backend_test.rs` - 硬件测试示例
- `send_receive.rs` - 发送和接收示例
- `multi_channel.rs` - 多通道操作示例

## 文档

- [API 文档](https://docs.rs/canlink-tscan)
- [硬件测试指南](../docs/guides/hardware-test-guide.md)
- [LibTSCAN API 参考](../docs/libtscan-api.md)

## 贡献

欢迎贡献！请查看 [贡献指南](../CONTRIBUTING.md)。

## 许可证

MIT OR Apache-2.0

## 支持

- **项目支持**: 请通过仓库 Issues 反馈问题
- **TSMaster 支持**: [TOSUN 官网](https://www.tosun.com/)

## TSCan Daemon Workaround (Vendor Bug)

To isolate the known vendor DLL hang in `DISCONNECT_*`, `canlink-tscan` now supports an out-of-process daemon path and enables it by default.

This is a temporary workaround for a vendor-side issue in `libTSCAN.dll`. When the vendor provides a stable fix, this workaround can be removed or downgraded in a future release.

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

When `use_daemon = false`, backend falls back to direct DLL calls.
