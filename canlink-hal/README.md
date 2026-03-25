# CANLink HAL

[![Crates.io](https://img.shields.io/crates/v/canlink-hal.svg)](https://crates.io/crates/canlink-hal)
[![Documentation](https://docs.rs/canlink-hal/badge.svg)](https://docs.rs/canlink-hal)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)

**CANLink HAL** 是 CANLink-RS 项目的核心硬件抽象层，提供统一的 API 来访问不同的 CAN 硬件后端。

## 特性

- 🔌 **统一接口** - 单一 API 支持多种硬件后端
- 🛡️ **类型安全** - 强类型的 CAN 消息和 ID
- 🚀 **零成本抽象** - 编译时优化，运行时开销极小
- 📦 **后端注册** - 动态注册和切换后端
- 🔧 **灵活配置** - 支持多种配置方式
- 📊 **能力查询** - 查询硬件能力和限制

## 安装

添加到您的 `Cargo.toml`：

```toml
[dependencies]
canlink-hal = "0.3.0"
```

## 核心概念

### 1. CanBackend Trait

所有 CAN 硬件后端都实现 `CanBackend` trait：

```rust
pub trait CanBackend: Send {
    fn initialize(&mut self, config: &BackendConfig) -> CanResult<()>;
    fn close(&mut self) -> CanResult<()>;
    fn open_channel(&mut self, channel: u8) -> CanResult<()>;
    fn close_channel(&mut self, channel: u8) -> CanResult<()>;
    fn send_message(&mut self, message: &CanMessage) -> CanResult<()>;
    fn receive_message(&mut self) -> CanResult<Option<CanMessage>>;
    fn get_capability(&self) -> CanResult<HardwareCapability>;
}
```

### 2. CanMessage

类型安全的 CAN 消息表示：

```rust
use canlink_hal::{CanMessage, CanId};

// 标准帧 (11-bit ID)
let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;

// 扩展帧 (29-bit ID)
let msg = CanMessage::new_extended(0x12345678, &[0xAA, 0xBB])?;

// CAN-FD 消息
let data = vec![0x42; 64];
let msg = CanMessage::new_canfd(0x200, &data, false)?;

// 远程帧
let msg = CanMessage::new_remote(CanId::Standard(0x123), 8)?;
```

### 3. CanId

类型安全的 CAN 标识符：

```rust
use canlink_hal::CanId;

// 标准 ID (11-bit)
let id = CanId::Standard(0x123);
assert!(!id.is_extended());

// 扩展 ID (29-bit)
let id = CanId::Extended(0x12345678);
assert!(id.is_extended());
```

### 4. BackendRegistry

全局后端注册表，用于管理多个后端：

```rust
use canlink_hal::{BackendRegistry, BackendConfig};
use canlink_mock::MockBackendFactory;
use std::sync::Arc;

// 获取全局注册表
let registry = BackendRegistry::global();

// 注册后端
let factory = Arc::new(MockBackendFactory::new());
registry.register(factory)?;

// 创建后端实例
let config = BackendConfig::new("mock");
let backend = registry.create_backend(&config)?;

// 列出所有后端
let backends = registry.list_backends();
```

## 使用示例

### 基础使用

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建后端
    let mut backend = MockBackend::new();

    // 初始化
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;

    // 打开通道
    backend.open_channel(0)?;

    // 发送消息
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;

    // 接收消息
    if let Some(msg) = backend.receive_message()? {
        println!("收到: ID={:X}, 数据={:?}", msg.id(), msg.data());
    }

    // 清理
    backend.close_channel(0)?;
    backend.close()?;

    Ok(())
}
```

### 查询硬件能力

```rust
use canlink_hal::CanBackend;

let capability = backend.get_capability()?;

println!("通道数: {}", capability.channel_count);
println!("支持 CAN-FD: {}", capability.supports_canfd);
println!("最大波特率: {} bps", capability.max_bitrate);
println!("支持的波特率: {:?}", capability.supported_bitrates);
println!("过滤器数量: {}", capability.filter_count);
```

### 使用后端注册表

```rust
use canlink_hal::{BackendRegistry, BackendConfig};
use canlink_mock::MockBackendFactory;
use canlink_tscan::TSCanBackendFactory;
use std::sync::Arc;

// 注册多个后端
let registry = BackendRegistry::global();

registry.register(Arc::new(MockBackendFactory::new()))?;
registry.register(Arc::new(TSCanBackendFactory::new()))?;

// 列出所有后端
for name in registry.list_backends() {
    println!("可用后端: {}", name);
}

// 根据配置创建后端
let config = BackendConfig::new("mock");
let mut backend = registry.create_backend(&config)?;

backend.initialize(&config)?;
// 使用后端...
```

### 错误处理

```rust
use canlink_hal::{CanError, CanResult};

fn send_with_retry(
    backend: &mut dyn CanBackend,
    msg: &CanMessage,
    max_retries: u32,
) -> CanResult<()> {
    let mut attempts = 0;

    loop {
        match backend.send_message(msg) {
            Ok(_) => return Ok(()),
            Err(CanError::SendFailed { reason }) if attempts < max_retries => {
                attempts += 1;
                eprintln!("发送失败 (尝试 {}/{}): {}", attempts, max_retries, reason);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(e) => return Err(e),
        }
    }
}
```

### 消息过滤

```rust
use canlink_hal::{CanMessage, CanId};

fn filter_messages(
    messages: Vec<CanMessage>,
    filter_id: CanId,
) -> Vec<CanMessage> {
    messages
        .into_iter()
        .filter(|msg| msg.id() == filter_id)
        .collect()
}

// 使用
let messages = vec![
    CanMessage::new_standard(0x100, &[1, 2])?,
    CanMessage::new_standard(0x200, &[3, 4])?,
    CanMessage::new_standard(0x100, &[5, 6])?,
];

let filtered = filter_messages(messages, CanId::Standard(0x100));
assert_eq!(filtered.len(), 2);
```

## API 文档

### 核心 Traits

- **`CanBackend`** - 后端实现的主要接口
- **`BackendFactory`** - 后端工厂接口

### 核心类型

- **`CanMessage`** - CAN 消息
- **`CanId`** - CAN 标识符 (Standard/Extended)
- **`MessageFlags`** - 消息标志 (RTR, FD, BRS, ESI)
- **`HardwareCapability`** - 硬件能力描述
- **`BackendConfig`** - 后端配置
- **`BackendVersion`** - 后端版本信息

### 错误类型

- **`CanError`** - 统一的错误类型
- **`CanResult<T>`** - 结果类型别名

### 工具类

- **`BackendRegistry`** - 全局后端注册表

## 性能

CANLink HAL 设计注重性能：

- **能力查询**: < 1 µs (实际: 0.641 µs)
- **消息转换**: ~3 ns
- **抽象层开销**: < 5% (实际: 可忽略不计)

详见 [性能基准测试报告](../docs/reports/performance-benchmark-report.md)。

## 测试

```bash
# 运行单元测试
cargo test -p canlink-hal

# 运行集成测试
cargo test -p canlink-hal --test '*'

# 运行基准测试
cargo bench -p canlink-hal
```

## 实现后端

要实现自己的后端，需要：

1. 实现 `CanBackend` trait
2. 实现 `BackendFactory` trait
3. 注册到 `BackendRegistry`

示例：

```rust
use canlink_hal::{
    BackendConfig, BackendFactory, BackendVersion, CanBackend,
    CanError, CanMessage, CanResult, HardwareCapability,
};

pub struct MyBackend {
    // 您的字段...
}

impl CanBackend for MyBackend {
    fn initialize(&mut self, config: &BackendConfig) -> CanResult<()> {
        // 初始化逻辑
        Ok(())
    }

    fn send_message(&mut self, message: &CanMessage) -> CanResult<()> {
        // 发送逻辑
        Ok(())
    }

    // 实现其他方法...
}

pub struct MyBackendFactory;

impl BackendFactory for MyBackendFactory {
    fn create(&self, _config: &BackendConfig) -> CanResult<Box<dyn CanBackend>> {
        Ok(Box::new(MyBackend::new()))
    }

    fn name(&self) -> &'static str {
        "my-backend"
    }

    fn version(&self) -> BackendVersion {
        BackendVersion::new(0, 1, 0)
    }
}
```

## 相关包

- [canlink-mock](../canlink-mock/) - Mock 测试后端
- [canlink-tscan](../canlink-tscan/) - LibTSCAN 真实硬件后端（当前已验证为同星 / TOSUN 相关硬件）
- [canlink-cli](../canlink-cli/) - 命令行工具

## 示例

查看 `examples/` 目录获取更多示例：

- `basic_usage.rs` - 基础使用
- `backend_switching.rs` - 后端切换
- `error_handling.rs` - 错误处理
- `capability_query.rs` - 能力查询

## 贡献

欢迎贡献！请查看 [贡献指南](../CONTRIBUTING.md)。

## 许可证

MIT OR Apache-2.0

## 文档

- [API 文档](https://docs.rs/canlink-hal)
- [用户指南](../docs/user-guide.md)
- [架构设计](../docs/architecture.md)
