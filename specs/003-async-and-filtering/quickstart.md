# 快速入门: 异步 API 与消息过滤

**功能分支**: `003-async-and-filtering`
**创建日期**: 2026-01-10
**状态**: 草稿

---

## 概述

本指南帮助你快速上手 canlink-rs 的异步 API 和消息过滤功能。

## 前置条件

- Rust 1.75+
- 已完成 002-can-hardware-abstraction 的基础设置
- 了解基本的 CAN 协议概念

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
canlink-hal = { version = "0.1", features = ["async", "tracing"] }
canlink-mock = "0.2"

# 异步运行时
tokio = { version = "1", features = ["full"] }
```

### Feature Flags

| Feature | 说明 |
|---------|------|
| `async` | 启用异步 API |
| `async-tokio` | 使用 tokio 运行时（默认） |
| `async-async-std` | 使用 async-std 运行时 |
| `tracing` | 启用日志支持 |
| `hot-reload` | 启用配置热重载 |
| `full` | 启用所有功能 |

---

## 快速开始

### 1. 基本消息过滤

```rust
use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter, MessageFilter};
use canlink_hal::message::CanMessage;

fn main() {
    // 创建过滤器链（最多 4 个硬件过滤器）
    let mut chain = FilterChain::new(4);

    // 添加 ID 过滤器：精确匹配 0x123
    chain.add_filter(Box::new(IdFilter::new(0x123)));

    // 添加范围过滤器：匹配 0x200-0x2FF
    chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));

    // 添加掩码过滤器：匹配 0x400-0x4FF（高 4 位为 0100）
    chain.add_filter(Box::new(IdFilter::with_mask(0x400, 0x700)));

    // 测试消息
    let msg1 = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();
    let msg2 = CanMessage::new_standard(0x250, &[5, 6, 7, 8]).unwrap();
    let msg3 = CanMessage::new_standard(0x999, &[9, 10, 11, 12]).unwrap();

    println!("msg1 (0x123) matches: {}", chain.matches(&msg1)); // true
    println!("msg2 (0x250) matches: {}", chain.matches(&msg2)); // true
    println!("msg3 (0x999) matches: {}", chain.matches(&msg3)); // false
}
```

### 2. 配置队列溢出策略

```rust
use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
use canlink_hal::message::CanMessage;
use std::time::Duration;

fn main() {
    // 策略 1: 丢弃最旧消息（默认，适合实时监控）
    let mut queue1 = BoundedQueue::new(100);

    // 策略 2: 丢弃最新消息（适合数据记录）
    let mut queue2 = BoundedQueue::with_policy(
        100,
        QueueOverflowPolicy::DropNewest,
    );

    // 策略 3: 阻塞等待（适合关键消息）
    let mut queue3 = BoundedQueue::with_policy(
        100,
        QueueOverflowPolicy::Block {
            timeout: Duration::from_millis(100),
        },
    );

    // 使用队列
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4]).unwrap();

    match queue1.push(msg.clone()) {
        Ok(()) => println!("Message enqueued"),
        Err(e) => println!("Failed: {}", e),
    }

    // 查看统计信息
    let stats = queue1.stats();
    println!("Enqueued: {}, Dropped: {}", stats.enqueued, stats.dropped);
}
```

### 3. 异步消息收发

```rust
use canlink_hal::backend::CanBackendAsync;
use canlink_mock::MockBackend;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建 Mock 后端
    let mut backend = MockBackend::new();
    backend.initialize()?;
    backend.open_channel(0)?;

    // 异步发送消息
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message_async(0, &msg).await?;
    println!("Message sent asynchronously");

    // 异步接收消息（带超时）
    match tokio::time::timeout(
        Duration::from_millis(100),
        backend.receive_message_async(0),
    ).await {
        Ok(Ok(msg)) => println!("Received: {:?}", msg),
        Ok(Err(e)) => println!("Receive error: {}", e),
        Err(_) => println!("Receive timeout"),
    }

    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
```

### 4. 连接监控

```rust
use canlink_hal::monitor::{ConnectionMonitor, ConnectionState, ReconnectConfig};
use canlink_mock::MockBackend;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = Arc::new(Mutex::new(MockBackend::new()));

    // 创建监控器（不启用自动重连）
    let mut monitor = ConnectionMonitor::new(
        backend.clone(),
        Duration::from_secs(1),
    );

    // 注册状态变化回调
    monitor.on_state_change(|old, new| {
        println!("Connection: {:?} -> {:?}", old, new);
        if new == ConnectionState::Disconnected {
            eprintln!("Warning: Connection lost!");
        }
    }).await;

    // 启动监控
    monitor.start().await;

    // ... 应用逻辑 ...

    // 停止监控
    monitor.stop().await;
    Ok(())
}
```

### 5. 启用自动重连

```rust
use canlink_hal::monitor::{ConnectionMonitor, ReconnectConfig};

// 创建带自动重连的监控器
let mut monitor = ConnectionMonitor::with_reconnect(
    backend,
    Duration::from_secs(1),
    ReconnectConfig {
        max_retries: 5,
        retry_interval: Duration::from_secs(2),
        backoff_multiplier: 1.5,
    },
);

monitor.on_state_change(|old, new| {
    match new {
        ConnectionState::Disconnected => {
            println!("Disconnected, will retry...");
        }
        ConnectionState::Reconnecting => {
            println!("Attempting reconnect...");
        }
        ConnectionState::Connected => {
            if old == ConnectionState::Reconnecting {
                println!("Reconnected!");
            }
        }
    }
}).await;

monitor.start().await;
```

---

## 配置文件

### 完整配置示例

创建 `canlink.toml`：

```toml
# 后端配置
[backend]
type = "mock"

# 过滤器配置
[filters]
[[filters.id_filters]]
id = 0x123
mask = 0x7FF
extended = false

[[filters.id_filters]]
id = 0x456
mask = 0x700  # 匹配 0x400-0x4FF

[[filters.range_filters]]
start_id = 0x200
end_id = 0x2FF
extended = false

# 队列配置
[queue]
capacity = 2000

[queue.overflow_policy]
type = "drop_oldest"

# 监控配置
[monitor]
heartbeat_interval_ms = 1000

# 可选：启用自动重连
# [monitor.reconnect]
# max_retries = 3
# retry_interval_ms = 1000
# backoff_multiplier = 2.0
```

### 从配置文件加载

```rust
use canlink_hal::config::CanlinkConfig;
use canlink_hal::filter::FilterChain;
use canlink_hal::queue::BoundedQueue;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载配置
    let config = CanlinkConfig::from_file("canlink.toml")?;

    // 创建过滤器链
    let chain = FilterChain::from_config(&config.filters)?;

    // 创建队列
    let queue = BoundedQueue::from_config(&config.queue)?;

    println!("Loaded {} filters", chain.total_filter_count());
    println!("Queue capacity: {}", queue.capacity());

    Ok(())
}
```

---

## 自定义过滤器

实现 `MessageFilter` trait 创建自定义过滤器：

```rust
use canlink_hal::filter::MessageFilter;
use canlink_hal::message::CanMessage;

/// 数据内容过滤器
///
/// 根据消息数据的第一个字节过滤。
struct DataFilter {
    first_byte: u8,
}

impl DataFilter {
    fn new(first_byte: u8) -> Self {
        Self { first_byte }
    }
}

impl MessageFilter for DataFilter {
    fn matches(&self, message: &CanMessage) -> bool {
        message.data().first() == Some(&self.first_byte)
    }

    fn priority(&self) -> u32 {
        10  // 较低优先级
    }

    fn is_hardware(&self) -> bool {
        false  // 软件过滤器
    }
}

// 使用自定义过滤器
fn main() {
    let mut chain = FilterChain::new(4);
    chain.add_filter(Box::new(DataFilter::new(0x01)));

    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04]).unwrap();
    println!("Matches: {}", chain.matches(&msg)); // true
}
```

---

## 日志配置

启用 `tracing` feature 后，可以配置日志：

```rust
use tracing_subscriber;

fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 现在所有 canlink 操作都会输出日志
    // ...
}
```

日志级别说明：

| 级别 | 内容 |
|------|------|
| ERROR | 错误和异常 |
| WARN | 警告（如队列溢出、高频消息） |
| INFO | 重要操作（初始化、关闭、状态变化） |
| DEBUG | 详细操作（消息收发） |
| TRACE | 最详细信息（过滤器匹配、队列操作） |

---

## 常见问题

### Q: 硬件过滤器和软件过滤器有什么区别？

**A**: 硬件过滤器由 CAN 控制器执行，性能更高，但数量有限（通常 4-16 个）。软件过滤器由 CPU 执行，数量不限但会消耗 CPU 资源。FilterChain 会自动管理：优先使用硬件过滤器，超出限制时回退到软件过滤。

### Q: 如何选择队列溢出策略？

**A**:
- **DropOldest**（默认）：适合实时监控，始终获取最新数据
- **DropNewest**：适合数据记录，保留完整历史
- **Block**：适合关键消息，不允许丢失

### Q: 为什么默认不启用自动重连？

**A**: 自动重连可能掩盖硬件问题，不适合所有场景。建议在应用层根据具体需求决定是否启用。

### Q: 如何处理内存压力？

**A**: 可以调用 `queue.adjust_capacity(new_size)` 减小队列容量。多余的消息会根据溢出策略处理。

---

## 下一步

- 阅读 [API 文档](https://docs.rs/canlink-hal) 了解完整 API
- 查看 [示例代码](../examples/) 了解更多用法
- 阅读 [data-model.md](data-model.md) 了解数据模型设计
- 阅读 [contracts/](contracts/) 了解 API 契约

---

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-01-10 | 初始版本 |
