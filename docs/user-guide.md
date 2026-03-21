# CANLink 使用指南

本指南将帮助您快速上手 CANLink 硬件抽象层，从基础概念到高级用法。

## 目录

1. [快速开始](#快速开始)
2. [核心概念](#核心概念)
3. [基础用法](#基础用法)
4. [异步 API](#异步-api)
5. [消息过滤](#消息过滤)
6. [连接监控](#连接监控)
7. [队列管理](#队列管理)
8. [配置热重载](#配置热重载)
9. [周期性消息发送](#周期性消息发送)
10. [ISO-TP 传输协议](#iso-tp-传输协议)
11. [高级特性](#高级特性)
12. [测试指南](#测试指南)
13. [最佳实践](#最佳实践)
14. [故障排除](#故障排除)

## 快速开始

### 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
canlink-hal = "0.2"
canlink-mock = "0.2"  # 用于测试

# 可选功能
[dependencies.canlink-hal]
version = "0.2"
features = ["async", "tracing", "hot-reload"]  # 按需启用
```

### Feature Flags

| Feature | 描述 |
|---------|------|
| `async` | 启用异步 API（`CanBackendAsync` trait） |
| `tracing` | 启用结构化日志（使用 tracing 框架） |
| `hot-reload` | 启用配置热重载功能 |
| `full` | 启用所有功能 |

### 第一个程序

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 创建后端
    let mut backend = MockBackend::new();

    // 2. 初始化
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;

    // 3. 打开通道
    backend.open_channel(0)?;

    // 4. 发送消息
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message(&msg)?;
    println!("消息发送成功！");

    // 5. 清理
    backend.close_channel(0)?;
    backend.close()?;

    Ok(())
}
```

## 核心概念

### 1. 后端（Backend）

后端是硬件接口的抽象。每个后端实现 `CanBackend` trait：

- **Mock Backend**: 软件模拟，用于测试
- **SocketCAN**: Linux CAN 接口（计划中）
- **PCAN**: PEAK-System 适配器（计划中）

### 2. 消息（Message）

CAN 消息包含：
- **ID**: 标准（11位）或扩展（29位）
- **数据**: 最多 8 字节（CAN 2.0）或 64 字节（CAN-FD）
- **标志**: 远程帧、CAN-FD 等

### 3. 通道（Channel）

物理 CAN 接口。一个后端可以有多个通道。

### 4. 能力（Capability）

描述硬件特性：
- 通道数量
- CAN-FD 支持
- 支持的波特率
- 硬件过滤器数量

## 基础用法

### 发送消息

#### 标准 ID 消息

```rust
use canlink_hal::{CanBackend, CanMessage};

// 11位 ID (0x000 - 0x7FF)
let msg = CanMessage::new_standard(0x123, &[0xAA, 0xBB, 0xCC, 0xDD])?;
backend.send_message(&msg)?;
```

#### 扩展 ID 消息

```rust
// 29位 ID (0x00000000 - 0x1FFFFFFF)
let msg = CanMessage::new_extended(0x12345678, &[1, 2, 3, 4, 5, 6, 7, 8])?;
backend.send_message(&msg)?;
```

#### CAN-FD 消息

```rust
use canlink_hal::CanId;

// 最多 64 字节
let data = vec![0; 64];
let msg = CanMessage::new_fd(CanId::Standard(0x200), &data)?;
backend.send_message(&msg)?;
```

#### 远程帧

```rust
// 请求 4 字节数据
let msg = CanMessage::new_remote(CanId::Standard(0x456), 4)?;
backend.send_message(&msg)?;
```

### 接收消息

```rust
use canlink_hal::CanBackend;

// 接收一条消息
if let Some(msg) = backend.receive_message()? {
    println!("收到消息:");
    println!("  ID: {:?}", msg.id());
    println!("  数据: {:02X?}", msg.data());
    println!("  长度: {} 字节", msg.data().len());

    // 检查消息类型
    if msg.is_fd() {
        println!("  这是 CAN-FD 消息");
    }
    if msg.is_remote() {
        println!("  这是远程帧");
    }
}
```

### 查询硬件能力

```rust
use canlink_hal::CanBackend;

let capability = backend.get_capability()?;

println!("硬件信息:");
println!("  通道数: {}", capability.channel_count);
println!("  CAN-FD 支持: {}", capability.supports_canfd);
println!("  最大波特率: {} bps", capability.max_bitrate);
println!("  硬件过滤器: {}", capability.filter_count);

// 检查特定波特率
if capability.supports_bitrate(1_000_000) {
    println!("  支持 1 Mbps");
}

// 检查通道可用性
if capability.has_channel(2) {
    println!("  通道 2 可用");
}
```

## 异步 API

v0.2.0 引入了异步 API 支持，允许您在异步运行时（如 Tokio）中使用 CAN 通信。

### 启用异步功能

在 `Cargo.toml` 中启用 `async` feature：

```toml
[dependencies]
canlink-hal = { version = "0.2", features = ["async"] }
tokio = { version = "1", features = ["full"] }
```

### CanBackendAsync Trait

异步 API 通过 `CanBackendAsync` trait 提供：

```rust
use canlink_hal::CanBackendAsync;

// trait 定义
pub trait CanBackendAsync: CanBackend {
    /// 异步发送消息
    async fn send_message_async(&mut self, message: &CanMessage) -> Result<(), CanError>;

    /// 异步接收消息（带超时）
    async fn receive_message_async(&mut self) -> Result<Option<CanMessage>, CanError>;

    /// 设置接收超时
    fn set_receive_timeout(&mut self, timeout: Duration);

    /// 获取当前接收超时
    fn receive_timeout(&self) -> Duration;
}
```

### 基本异步用法

```rust
use canlink_hal::{BackendConfig, CanBackend, CanBackendAsync, CanMessage};
use canlink_mock::MockBackend;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建并初始化后端
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // 设置接收超时
    backend.set_receive_timeout(Duration::from_millis(100));

    // 异步发送消息
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
    backend.send_message_async(&msg).await?;
    println!("消息已异步发送");

    // 异步接收消息
    match backend.receive_message_async().await? {
        Some(msg) => println!("收到消息: ID=0x{:03X}", msg.id().raw()),
        None => println!("超时，未收到消息"),
    }

    // 清理
    backend.close_channel(0)?;
    backend.close()?;

    Ok(())
}
```

### 并发接收多通道

异步 API 的主要优势是可以并发处理多个通道：

```rust
use canlink_hal::{CanBackend, CanBackendAsync};
use tokio::select;

async fn receive_from_multiple_channels(
    backend0: &mut impl CanBackendAsync,
    backend1: &mut impl CanBackendAsync,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        select! {
            result = backend0.receive_message_async() => {
                if let Ok(Some(msg)) = result {
                    println!("通道 0: ID=0x{:03X}", msg.id().raw());
                }
            }
            result = backend1.receive_message_async() => {
                if let Ok(Some(msg)) = result {
                    println!("通道 1: ID=0x{:03X}", msg.id().raw());
                }
            }
        }
    }
}
```

### 批量发送

```rust
use canlink_hal::{CanBackendAsync, CanMessage};
use futures::future::join_all;

async fn send_batch(
    backend: &mut impl CanBackendAsync,
    messages: Vec<CanMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 顺序发送（保证顺序）
    for msg in &messages {
        backend.send_message_async(msg).await?;
    }

    println!("已发送 {} 条消息", messages.len());
    Ok(())
}
```

### 超时处理

```rust
use canlink_hal::CanBackendAsync;
use std::time::Duration;
use tokio::time::timeout;

async fn receive_with_custom_timeout(
    backend: &mut impl CanBackendAsync,
) -> Result<(), Box<dyn std::error::Error>> {
    // 方法 1: 使用内置超时
    backend.set_receive_timeout(Duration::from_secs(1));
    let result = backend.receive_message_async().await?;

    // 方法 2: 使用 tokio::time::timeout
    let result = timeout(
        Duration::from_secs(5),
        backend.receive_message_async()
    ).await;

    match result {
        Ok(Ok(Some(msg))) => println!("收到: {:?}", msg),
        Ok(Ok(None)) => println!("内部超时"),
        Ok(Err(e)) => println!("错误: {}", e),
        Err(_) => println!("外部超时（5秒）"),
    }

    Ok(())
}
```

### 异步 vs 同步性能

异步 API 的吞吐量与同步 API 相当（≥95%），但在以下场景更有优势：

| 场景 | 推荐 API |
|------|----------|
| 单通道简单收发 | 同步 API |
| 多通道并发 | 异步 API |
| 与其他异步任务集成 | 异步 API |
| 低延迟要求 | 同步 API |
| 高并发应用 | 异步 API |

### 注意事项

1. **运行时要求**: 异步 API 需要 Tokio 运行时
2. **Feature Flag**: 必须启用 `async` feature
3. **超时设置**: 默认超时为 100ms，可通过 `set_receive_timeout` 调整
4. **线程安全**: 每个后端实例应在单个任务中使用

## 消息过滤

消息过滤允许您只接收感兴趣的 CAN 消息，减少 CPU 负载。

### 过滤器类型

CANLink 支持多种过滤器类型：

- **IdFilter**: 精确匹配单个 ID 或使用掩码匹配
- **RangeFilter**: 匹配 ID 范围内的消息
- **FilterChain**: 组合多个过滤器（OR 逻辑）

### 基本用法

```rust
use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter, MessageFilter};
use canlink_hal::CanMessage;

// 创建过滤器链（8 个硬件过滤器槽位）
let mut chain = FilterChain::new(8);

// 添加精确 ID 过滤器
chain.add_filter(Box::new(IdFilter::new(0x123)));

// 添加范围过滤器（0x200 到 0x2FF）
chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));

// 测试消息
let msg1 = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
let msg2 = CanMessage::new_standard(0x250, &[4, 5, 6]).unwrap();
let msg3 = CanMessage::new_standard(0x400, &[7, 8, 9]).unwrap();

assert!(chain.matches(&msg1));  // 匹配 IdFilter
assert!(chain.matches(&msg2));  // 匹配 RangeFilter
assert!(!chain.matches(&msg3)); // 不匹配任何过滤器
```

### 掩码过滤

使用掩码可以匹配一组相关的 ID：

```rust
use canlink_hal::filter::IdFilter;

// 匹配 0x120-0x12F（掩码 0x7F0 表示只检查高 7 位）
let filter = IdFilter::with_mask(0x120, 0x7F0);

// 这些 ID 都会匹配
// 0x120, 0x121, 0x122, ..., 0x12F
```

### 扩展帧过滤

```rust
use canlink_hal::filter::{IdFilter, RangeFilter};

// 扩展帧 ID 过滤器
let ext_filter = IdFilter::new_extended(0x12345678);

// 扩展帧范围过滤器
let ext_range = RangeFilter::new_extended(0x10000000, 0x1FFFFFFF);
```

### 与 MockBackend 集成

```rust
use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::{MockBackend, MockConfig};

// 创建带预设消息的后端
let preset = vec![
    CanMessage::new_standard(0x100, &[1]).unwrap(),
    CanMessage::new_standard(0x200, &[2]).unwrap(),
    CanMessage::new_standard(0x300, &[3]).unwrap(),
];
let config = MockConfig::with_preset_messages(preset);
let mut backend = MockBackend::with_config(config);

let backend_config = BackendConfig::new("mock");
backend.initialize(&backend_config).unwrap();
backend.open_channel(0).unwrap();

// 添加过滤器 - 只接收 0x200
backend.add_id_filter(0x200);

// 只有 0x200 的消息会被接收
while let Ok(Some(msg)) = backend.receive_message() {
    println!("收到: {:?}", msg.id()); // 只会打印 0x200
}
```

### 硬件 vs 软件过滤

过滤器可以标记为硬件过滤器，由 CAN 控制器执行：

```rust
use canlink_hal::filter::MessageFilter;

// 检查过滤器是否为硬件过滤器
if filter.is_hardware() {
    println!("这是硬件过滤器，由 CAN 控制器执行");
} else {
    println!("这是软件过滤器，由 CPU 执行");
}

// 获取过滤器优先级（用于排序）
let priority = filter.priority();
```

## 连接监控

连接监控帮助您跟踪 CAN 后端的健康状态，并可选择自动重连。

### 连接状态

```rust
use canlink_hal::monitor::ConnectionState;

// 三种连接状态
match state {
    ConnectionState::Connected => println!("已连接，正常工作"),
    ConnectionState::Disconnected => println!("已断开，需要重新初始化"),
    ConnectionState::Reconnecting => println!("正在重连中..."),
}

// 检查是否可以发送/接收
if state.can_send() {
    // 可以发送消息
}
```

### 基本监控

```rust
use canlink_hal::monitor::{ConnectionMonitor, ConnectionState};
use std::time::Duration;

// 创建监控器（1 秒心跳间隔）
let monitor = ConnectionMonitor::new(Duration::from_secs(1));

// 检查状态
assert_eq!(monitor.state(), ConnectionState::Connected);

// 检查心跳间隔
println!("心跳间隔: {:?}", monitor.heartbeat_interval());
```

### 自动重连

```rust
use canlink_hal::monitor::{ConnectionMonitor, ReconnectConfig};
use std::time::Duration;

// 配置自动重连
let reconnect_config = ReconnectConfig::exponential_backoff(
    5,                          // 最多重试 5 次
    Duration::from_secs(1),     // 初始间隔 1 秒
    2.0,                        // 指数退避因子
);

// 创建带自动重连的监控器
let monitor = ConnectionMonitor::with_reconnect(
    Duration::from_secs(1),
    reconnect_config,
);

assert!(monitor.auto_reconnect_enabled());
```

### 重连配置选项

```rust
use canlink_hal::monitor::ReconnectConfig;
use std::time::Duration;

// 固定间隔重连
let fixed = ReconnectConfig::fixed_interval(
    3,                          // 最多重试 3 次
    Duration::from_millis(500), // 每次间隔 500ms
);

// 指数退避重连
let exponential = ReconnectConfig::exponential_backoff(
    5,                          // 最多重试 5 次
    Duration::from_secs(1),     // 初始间隔 1 秒
    2.0,                        // 每次间隔翻倍
);
// 重试间隔: 1s, 2s, 4s, 8s, 16s

// 无限重试
let unlimited = ReconnectConfig {
    max_retries: 0,             // 0 表示无限重试
    retry_interval: Duration::from_secs(5),
    backoff_multiplier: 1.0,    // 固定间隔
};
```

### 模拟断开连接（测试用）

```rust
use canlink_hal::{BackendConfig, CanBackend, BackendState};
use canlink_mock::MockBackend;

let mut backend = MockBackend::new();
backend.initialize(&BackendConfig::new("mock")).unwrap();
backend.open_channel(0).unwrap();

// 模拟断开连接
backend.simulate_disconnect();
assert_eq!(backend.get_state(), BackendState::Error);

// 发送/接收现在会失败
assert!(backend.send_message(&msg).is_err());

// 模拟重新连接
backend.simulate_reconnect();
assert_eq!(backend.get_state(), BackendState::Ready);

// 现在可以正常工作了
assert!(backend.send_message(&msg).is_ok());
```

## 队列管理

队列管理帮助您处理高频消息场景，防止内存无限增长。

### 创建有界队列

```rust
use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
use canlink_hal::CanMessage;

// 创建容量为 100 的队列，使用 DropOldest 策略
let mut queue = BoundedQueue::with_policy(100, QueueOverflowPolicy::DropOldest);

// 推入消息
let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
queue.push(msg).unwrap();

// 弹出消息
if let Some(msg) = queue.pop() {
    println!("收到: {:?}", msg);
}
```

### 溢出策略

```rust
use canlink_hal::queue::QueueOverflowPolicy;

// DropOldest: 队列满时丢弃最旧的消息（默认）
let policy = QueueOverflowPolicy::DropOldest;

// DropNewest: 队列满时拒绝新消息
let policy = QueueOverflowPolicy::DropNewest;

// Block: 队列满时阻塞等待（带超时）
let policy = QueueOverflowPolicy::Block {
    timeout_ms: 1000, // 最多等待 1 秒
};
```

### 队列统计

```rust
use canlink_hal::queue::BoundedQueue;

let queue = BoundedQueue::new(100);

// 获取统计信息
let stats = queue.stats();
println!("入队: {}", stats.enqueued);
println!("出队: {}", stats.dequeued);
println!("丢弃: {}", stats.dropped);
println!("溢出次数: {}", stats.overflow_count);

// 检查队列状态
println!("当前长度: {}", queue.len());
println!("容量: {}", queue.capacity());
println!("是否为空: {}", queue.is_empty());
println!("是否已满: {}", queue.is_full());
```

## 配置热重载

v0.2.0 支持配置文件热重载，无需重启应用即可更新配置。

### 启用热重载功能

在 `Cargo.toml` 中启用 `hot-reload` feature：

```toml
[dependencies]
canlink-hal = { version = "0.2", features = ["hot-reload"] }
```

### ConfigWatcher 基本用法

```rust
use canlink_hal::hot_reload::ConfigWatcher;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建配置监视器
    let config_path = Path::new("config/canlink.toml");
    let mut watcher = ConfigWatcher::new(config_path)?;

    // 注册配置变更回调
    watcher.on_config_change(|new_config| {
        println!("配置已更新!");
        println!("  后端: {}", new_config.backend_name);
        // 应用新配置...
    });

    // 启动监视
    watcher.start()?;

    // 应用运行中...
    // 当 config/canlink.toml 文件被修改时，回调会自动触发

    // 停止监视
    watcher.stop()?;

    Ok(())
}
```

### 配置文件格式

```toml
# config/canlink.toml

[backend]
name = "tscan"
channel = 0

[bitrate]
nominal = 500000
data = 2000000  # CAN-FD 数据段波特率

[filter]
# ID 过滤器
[[filter.id]]
id = 0x123

[[filter.id]]
id = 0x200
mask = 0x7F0

# 范围过滤器
[[filter.range]]
start = 0x300
end = 0x3FF

[queue]
capacity = 1000
overflow_policy = "drop_oldest"  # drop_oldest, drop_newest, block

[monitor]
heartbeat_interval_ms = 1000
auto_reconnect = true
max_retries = 5
retry_interval_ms = 1000
```

### 动态更新过滤器

```rust
use canlink_hal::hot_reload::ConfigWatcher;
use canlink_hal::filter::FilterChain;

let mut watcher = ConfigWatcher::new("config/filters.toml")?;

watcher.on_config_change(|config| {
    // 从配置构建新的过滤器链
    let new_chain: FilterChain = config.filter.into();

    // 更新后端的过滤器
    // backend.set_filter_chain(new_chain);

    println!("过滤器已更新: {} 个过滤器", new_chain.len());
});

watcher.start()?;
```

### 注意事项

1. **文件监视**: 使用 `notify` crate 监视文件系统变更
2. **防抖动**: 内置防抖动机制，避免频繁触发回调
3. **错误处理**: 配置解析失败时会记录错误，不会中断应用
4. **线程安全**: 回调在独立线程中执行

## 周期性消息发送

v0.3.0 新增周期性消息发送功能，允许您按固定时间间隔自动发送 CAN 消息。

### 启用周期发送功能

在 `Cargo.toml` 中启用 `periodic` feature：

```toml
[dependencies]
canlink-hal = { version = "0.3", features = ["periodic"] }
tokio = { version = "1", features = ["full"] }
```

### 基本用法

```rust
use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage};
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建并初始化后端
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;

    // 创建调度器（最多 32 个并发消息）
    let scheduler = PeriodicScheduler::new(backend, 32).await?;

    // 创建周期消息（每 100ms 发送一次）
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    let periodic = PeriodicMessage::new(msg, Duration::from_millis(100))?;

    // 添加到调度器
    let id = scheduler.add(periodic).await?;
    println!("添加周期消息，ID: {}", id);

    // 运行一段时间
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 获取统计信息
    if let Some(stats) = scheduler.get_stats(id).await {
        println!("发送次数: {}", stats.send_count());
        if let Some(avg) = stats.average_interval() {
            println!("平均间隔: {:?}", avg);
        }
    }

    // 关闭调度器
    scheduler.shutdown().await?;
    Ok(())
}
```

### 动态更新

周期消息支持运行时动态更新数据和间隔：

```rust
use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage};
use std::time::Duration;

// 假设 scheduler 和 id 已创建
async fn dynamic_update(
    scheduler: &PeriodicScheduler<impl canlink_hal::CanBackendAsync>,
    id: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    // 更新消息数据（不中断发送周期）
    scheduler.update_data(id, vec![0xAA, 0xBB, 0xCC, 0xDD]).await?;

    // 更新发送间隔
    scheduler.update_interval(id, Duration::from_millis(50)).await?;

    // 暂停发送
    scheduler.set_enabled(id, false).await?;

    // 恢复发送
    scheduler.set_enabled(id, true).await?;

    Ok(())
}
```

### 多消息并发

调度器支持同时管理多个周期消息：

```rust
use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage};
use canlink_hal::CanMessage;
use std::time::Duration;

async fn multi_message_example(
    scheduler: &PeriodicScheduler<impl canlink_hal::CanBackendAsync>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 添加多个不同间隔的消息
    let msg1 = CanMessage::new_standard(0x100, &[0x01])?;
    let id1 = scheduler.add(PeriodicMessage::new(msg1, Duration::from_millis(10))?).await?;

    let msg2 = CanMessage::new_standard(0x200, &[0x02])?;
    let id2 = scheduler.add(PeriodicMessage::new(msg2, Duration::from_millis(50))?).await?;

    let msg3 = CanMessage::new_standard(0x300, &[0x03])?;
    let id3 = scheduler.add(PeriodicMessage::new(msg3, Duration::from_millis(100))?).await?;

    // 列出所有消息
    let ids = scheduler.list_ids().await;
    println!("当前周期消息: {:?}", ids);

    // 移除某个消息
    scheduler.remove(id2).await?;

    Ok(())
}
```

### 统计信息

`PeriodicStats` 提供详细的发送统计：

```rust
use canlink_hal::periodic::PeriodicStats;

fn print_stats(stats: &PeriodicStats) {
    println!("发送次数: {}", stats.send_count());

    if let Some(avg) = stats.average_interval() {
        println!("平均间隔: {:?}", avg);
    }

    if let Some(min) = stats.min_interval() {
        println!("最小间隔: {:?}", min);
    }

    if let Some(max) = stats.max_interval() {
        println!("最大间隔: {:?}", max);
    }

    if let Some(jitter) = stats.jitter() {
        println!("抖动: {:?}", jitter);
    }
}
```

### 间隔限制

- **最小间隔**: 1ms
- **最大间隔**: 10000ms (10秒)
- **最大并发消息**: 由调度器容量决定（推荐 32）

### CLI 支持

当前正式 CLI 不再提供 ISO-TP 子命令。ISO-TP 仅作为库能力保留，如需命令行操作请自行构建专用工具或使用示例代码。

## ISO-TP 传输协议

v0.3.0 新增 ISO-TP (ISO 15765-2) 传输协议支持，用于发送超过单帧大小的数据。

### 启用 ISO-TP 功能

在 `Cargo.toml` 中启用 `isotp` feature：

```toml
[dependencies]
canlink-hal = { version = "0.3", features = ["isotp"] }
tokio = { version = "1", features = ["full"] }
```

### ISO-TP 概述

ISO-TP 是一种传输协议，用于在 CAN 总线上传输大于 8 字节（CAN 2.0）或 64 字节（CAN-FD）的数据。它定义了四种帧类型：

| 帧类型 | 缩写 | 用途 |
|--------|------|------|
| Single Frame | SF | 传输 ≤7 字节数据 |
| First Frame | FF | 多帧传输的第一帧 |
| Consecutive Frame | CF | 多帧传输的后续帧 |
| Flow Control | FC | 控制发送速率 |

### 基本用法

```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig, StMin};
use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::MockBackend;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建后端
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;

    // 配置 ISO-TP 通道
    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)           // 发送 ID（诊断请求）
        .rx_id(0x7E8)           // 接收 ID（诊断响应）
        .block_size(0)          // 无块大小限制
        .st_min(StMin::Milliseconds(10))  // 帧间隔 10ms
        .timeout(Duration::from_millis(1000))
        .build()?;

    // 创建 ISO-TP 通道
    let mut channel = IsoTpChannel::new(backend, config)?;

    // 发送数据（自动处理分段）
    let request = vec![0x10, 0x01]; // UDS DiagnosticSessionControl
    channel.send(&request).await?;
    println!("请求已发送");

    // 接收响应（自动处理重组）
    let response = channel.receive().await?;
    println!("响应: {:02X?}", response);

    Ok(())
}
```

### UDS 诊断示例

ISO-TP 常用于 UDS (Unified Diagnostic Services) 诊断通信：

```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig};
use std::time::Duration;

async fn uds_read_dtc(
    channel: &mut IsoTpChannel<impl canlink_hal::CanBackendAsync>,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // 发送 ReadDTCInformation 请求
    // Service ID: 0x19, Sub-function: 0x02 (reportDTCByStatusMask)
    let request = vec![0x19, 0x02, 0xFF]; // 读取所有 DTC
    channel.send(&request).await?;

    // 接收响应
    let response = channel.receive().await?;

    // 检查响应
    if response.len() >= 2 && response[0] == 0x59 {
        println!("DTC 读取成功");
        Ok(response)
    } else if response.len() >= 3 && response[0] == 0x7F {
        let nrc = response[2];
        Err(format!("否定响应: NRC=0x{:02X}", nrc).into())
    } else {
        Err("无效响应".into())
    }
}
```

### 大数据传输

ISO-TP 自动处理大数据的分段和重组：

```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig};

async fn transfer_large_data(
    channel: &mut IsoTpChannel<impl canlink_hal::CanBackendAsync>,
) -> Result<(), Box<dyn std::error::Error>> {
    // 发送大数据（例如固件块）
    let large_data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

    println!("发送 {} 字节数据...", large_data.len());
    channel.send(&large_data).await?;
    println!("发送完成");

    Ok(())
}
```

### Flow Control 配置

Flow Control 参数影响传输效率：

```rust
use canlink_hal::isotp::{IsoTpConfig, StMin, FrameSize};
use std::time::Duration;

// 高速传输配置
let fast_config = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .block_size(0)              // 无限制，连续发送
    .st_min(StMin::Milliseconds(0))  // 无间隔
    .timeout(Duration::from_millis(5000))
    .build()?;

// 低速/兼容配置
let slow_config = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .block_size(8)              // 每 8 帧等待 FC
    .st_min(StMin::Milliseconds(20))  // 20ms 间隔
    .timeout(Duration::from_millis(5000))
    .build()?;

// CAN-FD 配置
let fd_config = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .frame_size(FrameSize::Fd64)  // 强制使用 64 字节帧
    .build()?;
```

### 地址模式

ISO-TP 支持多种地址模式：

```rust
use canlink_hal::isotp::{IsoTpConfig, AddressingMode};

// 普通地址模式（默认）
let normal = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .addressing_mode(AddressingMode::Normal)
    .build()?;

// 扩展地址模式
let extended = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .addressing_mode(AddressingMode::Extended { target_address: 0x01 })
    .build()?;

// 混合地址模式
let mixed = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .addressing_mode(AddressingMode::Mixed { address_extension: 0xF1 })
    .build()?;
```

### 错误处理

ISO-TP 定义了多种错误情况：

```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpError};

async fn handle_isotp_errors(
    channel: &mut IsoTpChannel<impl canlink_hal::CanBackendAsync>,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    // 发送请求
    if let Err(e) = channel.send(data).await {
        match e {
            IsoTpError::FcTimeout { timeout_ms } => {
                return Err(format!("等待 Flow Control 超时: {}ms", timeout_ms));
            }
            IsoTpError::RemoteOverflow => {
                return Err("远端缓冲区溢出".to_string());
            }
            IsoTpError::TooManyWaits { count, max } => {
                return Err(format!("FC(Wait) 次数过多: {}/{}", count, max));
            }
            _ => return Err(format!("发送错误: {}", e)),
        }
    }

    // 接收响应
    match channel.receive().await {
        Ok(response) => Ok(response),
        Err(IsoTpError::RxTimeout { timeout_ms }) => {
            Err(format!("接收超时: {}ms", timeout_ms))
        }
        Err(IsoTpError::SequenceMismatch { expected, actual }) => {
            Err(format!("序列号错误: 期望 {}, 实际 {}", expected, actual))
        }
        Err(e) => Err(format!("接收错误: {}", e)),
    }
}
```

### CLI 支持

当前正式 CLI 不再提供 ISO-TP 子命令。ISO-TP 仅作为库能力保留，如需命令行操作请自行构建专用工具或使用示例代码。

## 高级特性

### 日志框架（Tracing）

v0.2.0 集成了 `tracing` 框架，提供结构化日志支持。

#### 启用日志功能

```toml
[dependencies]
canlink-hal = { version = "0.2", features = ["tracing"] }
tracing-subscriber = "0.3"
```

#### 初始化日志

```rust
use tracing_subscriber;

fn main() {
    // 初始化日志订阅器
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 现在 CANLink 的日志会自动输出
    // 例如：
    // DEBUG canlink_hal::backend: Initializing backend name="tscan"
    // INFO canlink_hal::filter: Filter chain created filters=3
}
```

#### 日志级别

| 级别 | 用途 |
|------|------|
| `ERROR` | 严重错误，操作失败 |
| `WARN` | 警告，如高频消息、队列溢出 |
| `INFO` | 重要操作，如初始化、连接状态变更 |
| `DEBUG` | 调试信息，如消息收发详情 |
| `TRACE` | 详细跟踪，如每条消息的处理 |

#### 自定义日志输出

```rust
use tracing_subscriber::fmt::format::FmtSpan;

// JSON 格式输出（适合日志聚合）
tracing_subscriber::fmt()
    .json()
    .init();

// 包含 span 事件
tracing_subscriber::fmt()
    .with_span_events(FmtSpan::CLOSE)
    .init();

// 输出到文件
let file = std::fs::File::create("canlink.log")?;
tracing_subscriber::fmt()
    .with_writer(file)
    .init();
```

### 能力适配

根据硬件能力自动调整应用行为：

```rust
use canlink_hal::{CanBackend, CanMessage, CanId};

let capability = backend.get_capability()?;

// 根据 CAN-FD 支持选择消息类型
let data = vec![0; 12];
let msg = if capability.supports_canfd {
    // 使用 CAN-FD 发送完整数据
    CanMessage::new_fd(CanId::Standard(0x123), &data)?
} else {
    // 分割为多个 CAN 2.0 消息
    println!("警告: 不支持 CAN-FD，只发送前 8 字节");
    CanMessage::new_standard(0x123, &data[..8])?
};

backend.send_message(&msg)?;
```

### 错误处理

```rust
use canlink_hal::{CanBackend, CanError, BusErrorKind};

match backend.send_message(&msg) {
    Ok(_) => println!("发送成功"),

    Err(CanError::SendFailed { reason }) => {
        eprintln!("发送失败: {}", reason);
        // 实现重试逻辑
    }

    Err(CanError::BusError { kind }) => {
        match kind {
            BusErrorKind::BusOff => {
                eprintln!("总线关闭，需要重新初始化");
                // 重新初始化总线
            }
            BusErrorKind::ErrorPassive => {
                eprintln!("错误被动状态");
            }
            _ => eprintln!("总线错误: {:?}", kind),
        }
    }

    Err(CanError::Timeout { timeout_ms }) => {
        eprintln!("超时: {} ms", timeout_ms);
    }

    Err(e) => eprintln!("其他错误: {}", e),
}
```

### 重试机制

```rust
use canlink_hal::{CanBackend, CanMessage};
use std::thread;
use std::time::Duration;

fn send_with_retry(
    backend: &mut dyn CanBackend,
    msg: &CanMessage,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut attempts = 0;

    loop {
        match backend.send_message(msg) {
            Ok(_) => {
                println!("发送成功（尝试 {} 次）", attempts + 1);
                return Ok(());
            }
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(format!("发送失败，已重试 {} 次: {}", attempts, e).into());
                }
                eprintln!("发送失败，重试 {}/{}...", attempts, max_retries);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

// 使用
let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
send_with_retry(&mut backend, &msg, 5)?;
```

### 后端注册

```rust
use canlink_hal::{BackendRegistry, BackendConfig};
use canlink_mock::MockBackendFactory;
use std::sync::Arc;

// 获取全局注册表
let registry = BackendRegistry::global();

// 注册后端
let factory = Arc::new(MockBackendFactory::new());
registry.register(factory)?;

// 列出可用后端
for name in registry.list_backends() {
    println!("可用后端: {}", name);

    // 获取后端信息
    if let Ok(info) = registry.get_backend_info(&name) {
        println!("  版本: {}", info.version);
    }
}

// 创建后端实例
let config = BackendConfig::new("mock");
let mut backend = registry.create("mock", &config)?;
```

## 测试指南

### 使用 Mock Backend

Mock backend 专为测试设计：

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage, CanId};
use canlink_mock::MockBackend;

#[test]
fn test_can_communication() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // 发送消息
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    backend.send_message(&msg).unwrap();

    // 验证消息已发送
    assert!(backend.verify_message_sent(CanId::Standard(0x123)));

    // 获取记录的消息
    let recorded = backend.get_recorded_messages();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].data(), &[1, 2, 3]);
}
```

### 错误注入测试

```rust
use canlink_hal::{CanBackend, CanError, CanMessage};
use canlink_mock::MockBackend;

#[test]
fn test_error_handling() {
    let mut backend = MockBackend::new();
    let config = BackendConfig::new("mock");
    backend.initialize(&config).unwrap();
    backend.open_channel(0).unwrap();

    // 注入发送错误
    backend.error_injector_mut().inject_send_error(
        CanError::SendFailed {
            reason: "测试错误".to_string(),
        }
    );

    // 发送应该失败
    let msg = CanMessage::new_standard(0x123, &[1, 2, 3]).unwrap();
    let result = backend.send_message(&msg);
    assert!(result.is_err());

    // 失败的消息不应被记录
    assert_eq!(backend.get_recorded_messages().len(), 0);
}
```

### 预设消息测试

```rust
use canlink_hal::{CanBackend, CanMessage, CanId};
use canlink_mock::{MockBackend, MockConfig};

#[test]
fn test_receive_protocol() {
    // 创建预设响应
    let responses = vec![
        CanMessage::new_standard(0x7E8, &[0x04, 0x41, 0x0C, 0x1A, 0xF8]).unwrap(),
    ];

    let config = MockConfig::with_preset_messages(responses);
    let mut backend = MockBackend::with_config(config);
    let backend_config = BackendConfig::new("mock");
    backend.initialize(&backend_config).unwrap();
    backend.open_channel(0).unwrap();

    // 发送请求
    let request = CanMessage::new_standard(0x7DF, &[0x02, 0x01, 0x0C]).unwrap();
    backend.send_message(&request).unwrap();

    // 接收响应
    let response = backend.receive_message().unwrap().unwrap();
    assert_eq!(response.id(), CanId::Standard(0x7E8));

    // 验证请求已发送
    assert!(backend.verify_message_sent(CanId::Standard(0x7DF)));
}
```

## 最佳实践

### 1. 错误处理

**✅ 推荐**:
```rust
match backend.send_message(&msg) {
    Ok(_) => { /* 处理成功 */ }
    Err(e) => {
        log::error!("发送失败: {}", e);
        // 实现恢复逻辑
    }
}
```

**❌ 不推荐**:
```rust
backend.send_message(&msg).unwrap(); // 可能导致 panic
```

### 2. 资源清理

**✅ 推荐**:
```rust
fn use_can() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    backend.initialize(&config)?;
    backend.open_channel(0)?;

    // 使用后端...

    // 确保清理
    backend.close_channel(0)?;
    backend.close()?;
    Ok(())
}
```

### 3. 能力检查

**✅ 推荐**:
```rust
let capability = backend.get_capability()?;
if capability.supports_canfd {
    // 使用 CAN-FD
} else {
    // 降级到 CAN 2.0
}
```

**❌ 不推荐**:
```rust
// 假设支持 CAN-FD
let msg = CanMessage::new_fd(id, &data)?;
backend.send_message(&msg)?; // 可能失败
```

### 4. 线程安全

每个线程使用独立的后端实例：

```rust
use std::thread;

let mut backend1 = MockBackend::new();
let mut backend2 = MockBackend::new();

let handle1 = thread::spawn(move || {
    // backend1 在这个线程中使用
});

let handle2 = thread::spawn(move || {
    // backend2 在这个线程中使用
});
```

## 故障排除

### 问题：初始化失败

```
Error: InitializationFailed { reason: "..." }
```

**解决方案**:
1. 检查后端配置是否正确
2. 确认硬件已连接
3. 检查权限（Linux 需要 `can` 组）

### 问题：发送失败

```
Error: SendFailed { reason: "..." }
```

**解决方案**:
1. 检查通道是否已打开
2. 检查总线状态（可能是 Bus-Off）
3. 实现重试机制

### 问题：通道未找到

```
Error: ChannelNotFound { channel: 2, max: 1 }
```

**解决方案**:
1. 查询硬件能力获取可用通道数
2. 使用有效的通道号（0 到 max）

### 问题：不支持的特性

```
Error: UnsupportedFeature { feature: "CAN-FD" }
```

**解决方案**:
1. 查询硬件能力
2. 根据能力调整应用行为
3. 考虑使用不同的后端

## 下一步

- 查看 [API 文档](https://docs.rs/canlink-hal)
- 浏览 [示例代码](../examples/)
- 阅读 [Mock Backend 指南](../canlink-mock/README.md)
- 使用 [CLI 工具](../canlink-cli/README.md)
- 查看 [CHANGELOG](../CHANGELOG.md) 了解版本更新

## 版本历史

- **v0.3.0** (当前): 周期性消息发送、ISO-TP 传输协议、CLI 扩展
- **v0.2.0**: 异步 API、消息过滤、连接监控、队列管理、配置热重载
- **v0.1.0**: 核心功能、Mock 后端、TSCan 后端（LibTSCAN 路径）、CLI 工具

## 获取帮助

- 📖 [文档](https://docs.rs/canlink-hal)
- 🐛 [Issues](https://github.com/iamsevens/canlink-rs/issues)
- 💬 [Discussions](https://github.com/iamsevens/canlink-rs/discussions)

## TSCan 守护进程规避方案

`canlink-tscan` 默认启用守护进程隔离，用来规避厂商 DLL 在断开调用期间导致宿主进程卡死的问题。

可通过 `canlink-tscan.toml` 调整行为：

```toml
use_daemon = true
request_timeout_ms = 2000
disconnect_timeout_ms = 3000
restart_max_retries = 3
recv_timeout_ms = 0
# daemon_path = "C:/path/to/canlink-tscan-daemon.exe"
```

配置优先级（高 -> 低）：

1. `BackendConfig.parameters`
2. `canlink-tscan.toml`
3. 内置默认值

将 `use_daemon = false` 可显式关闭该规避方案，恢复为直接 DLL 调用。
