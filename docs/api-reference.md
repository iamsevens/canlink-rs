# CANLink API 参考手册

本文档提供 CANLink v0.2.0 的完整 API 参考。

## 目录

1. [核心 Traits](#核心-traits)
2. [消息类型](#消息类型)
3. [过滤器 API](#过滤器-api)
4. [队列 API](#队列-api)
5. [监控 API](#监控-api)
6. [配置 API](#配置-api)
7. [周期性消息发送 API](#周期性消息发送-api)
8. [ISO-TP 传输协议 API](#iso-tp-传输协议-api)
9. [错误类型](#错误类型)

---

## 核心 Traits

### CanBackend

同步 CAN 后端接口，所有后端实现必须实现此 trait。

```rust
pub trait CanBackend: Send {
    /// 返回后端名称
    fn name(&self) -> &str;

    /// 返回后端版本
    fn version(&self) -> &str;

    /// 初始化后端
    fn initialize(&mut self, config: &BackendConfig) -> Result<(), CanError>;

    /// 关闭后端
    fn close(&mut self) -> Result<(), CanError>;

    /// 获取后端状态
    fn get_state(&self) -> BackendState;

    /// 获取硬件能力
    fn get_capability(&self) -> Result<BackendCapability, CanError>;

    /// 打开指定通道
    fn open_channel(&mut self, channel: u8) -> Result<(), CanError>;

    /// 关闭指定通道
    fn close_channel(&mut self, channel: u8) -> Result<(), CanError>;

    /// 发送 CAN 消息
    fn send_message(&mut self, message: &CanMessage) -> Result<(), CanError>;

    /// 接收 CAN 消息（非阻塞）
    fn receive_message(&mut self) -> Result<Option<CanMessage>, CanError>;
}
```

**示例**:
```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_mock::MockBackend;

let mut backend = MockBackend::new();
backend.initialize(&BackendConfig::new("mock"))?;
backend.open_channel(0)?;

let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
backend.send_message(&msg)?;

if let Some(received) = backend.receive_message()? {
    println!("Received: {:?}", received);
}

backend.close_channel(0)?;
backend.close()?;
```

---

### CanBackendAsync

异步 CAN 后端接口（需要 `async` feature）。

```rust
#[cfg(feature = "async")]
pub trait CanBackendAsync: CanBackend {
    /// 异步发送消息
    async fn send_message_async(&mut self, message: &CanMessage) -> Result<(), CanError>;

    /// 异步接收消息
    async fn receive_message_async(&mut self) -> Result<Option<CanMessage>, CanError>;

    /// 设置接收超时
    fn set_receive_timeout(&mut self, timeout: Duration);

    /// 获取接收超时
    fn receive_timeout(&self) -> Duration;
}
```

**示例**:
```rust
use canlink_hal::{CanBackend, CanBackendAsync, CanMessage};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;

    backend.set_receive_timeout(Duration::from_millis(500));

    let msg = CanMessage::new_standard(0x123, &[1, 2, 3])?;
    backend.send_message_async(&msg).await?;

    match backend.receive_message_async().await? {
        Some(msg) => println!("Received: {:?}", msg),
        None => println!("Timeout"),
    }

    Ok(())
}
```

---

## 消息类型

### CanMessage

CAN 消息结构体。

```rust
pub struct CanMessage {
    id: CanId,
    data: Vec<u8>,
    timestamp: Option<Duration>,
    flags: MessageFlags,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new_standard(id: u32, data: &[u8])` | 创建标准帧（11位 ID） |
| `new_extended(id: u32, data: &[u8])` | 创建扩展帧（29位 ID） |
| `new_fd(id: CanId, data: &[u8])` | 创建 CAN-FD 帧（最多64字节） |
| `new_remote(id: CanId, dlc: u8)` | 创建远程帧 |

**访问器**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `id()` | `CanId` | 获取消息 ID |
| `data()` | `&[u8]` | 获取数据 |
| `timestamp()` | `Option<Duration>` | 获取时间戳 |
| `is_fd()` | `bool` | 是否为 CAN-FD 帧 |
| `is_remote()` | `bool` | 是否为远程帧 |
| `is_extended()` | `bool` | 是否为扩展帧 |

**示例**:
```rust
use canlink_hal::{CanMessage, CanId};

// 标准帧
let msg1 = CanMessage::new_standard(0x123, &[0xAA, 0xBB])?;
assert_eq!(msg1.id(), CanId::Standard(0x123));
assert_eq!(msg1.data(), &[0xAA, 0xBB]);

// 扩展帧
let msg2 = CanMessage::new_extended(0x12345678, &[1, 2, 3, 4])?;
assert!(msg2.is_extended());

// CAN-FD 帧
let data = vec![0; 64];
let msg3 = CanMessage::new_fd(CanId::Standard(0x200), &data)?;
assert!(msg3.is_fd());

// 远程帧
let msg4 = CanMessage::new_remote(CanId::Standard(0x300), 8)?;
assert!(msg4.is_remote());
```

---

### CanId

CAN 消息 ID 枚举。

```rust
pub enum CanId {
    Standard(u32),  // 11位 ID (0x000 - 0x7FF)
    Extended(u32),  // 29位 ID (0x00000000 - 0x1FFFFFFF)
}
```

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `raw()` | `u32` | 获取原始 ID 值 |
| `is_standard()` | `bool` | 是否为标准 ID |
| `is_extended()` | `bool` | 是否为扩展 ID |

---

## 过滤器 API

### MessageFilter Trait

消息过滤器接口。

```rust
pub trait MessageFilter: Send + Sync {
    /// 检查消息是否匹配过滤器
    fn matches(&self, message: &CanMessage) -> bool;

    /// 获取过滤器优先级（默认 0）
    fn priority(&self) -> u32 { 0 }

    /// 是否为硬件过滤器（默认 false）
    fn is_hardware(&self) -> bool { false }
}
```

---

### IdFilter

ID 过滤器，支持精确匹配和掩码匹配。

```rust
pub struct IdFilter {
    id: u32,
    mask: u32,
    extended: bool,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(id: u32)` | 精确匹配标准帧 ID |
| `with_mask(id: u32, mask: u32)` | 掩码匹配标准帧 |
| `new_extended(id: u32)` | 精确匹配扩展帧 ID |
| `with_mask_extended(id: u32, mask: u32)` | 掩码匹配扩展帧 |

**示例**:
```rust
use canlink_hal::filter::{IdFilter, MessageFilter};
use canlink_hal::CanMessage;

// 精确匹配 0x123
let filter1 = IdFilter::new(0x123);
let msg = CanMessage::new_standard(0x123, &[1])?;
assert!(filter1.matches(&msg));

// 掩码匹配 0x120-0x12F
let filter2 = IdFilter::with_mask(0x120, 0x7F0);
let msg2 = CanMessage::new_standard(0x125, &[1])?;
assert!(filter2.matches(&msg2));
```

---

### RangeFilter

范围过滤器，匹配 ID 范围内的消息。

```rust
pub struct RangeFilter {
    start_id: u32,
    end_id: u32,
    extended: bool,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(start: u32, end: u32)` | 标准帧 ID 范围 |
| `new_extended(start: u32, end: u32)` | 扩展帧 ID 范围 |

**示例**:
```rust
use canlink_hal::filter::{RangeFilter, MessageFilter};
use canlink_hal::CanMessage;

// 匹配 0x200 到 0x2FF
let filter = RangeFilter::new(0x200, 0x2FF);

let msg1 = CanMessage::new_standard(0x250, &[1])?;
let msg2 = CanMessage::new_standard(0x300, &[1])?;

assert!(filter.matches(&msg1));   // 在范围内
assert!(!filter.matches(&msg2));  // 超出范围
```

---

### FilterChain

过滤器链，组合多个过滤器（OR 逻辑）。

```rust
pub struct FilterChain {
    filters: Vec<Box<dyn MessageFilter>>,
    max_hardware_filters: usize,
}
```

**方法**:

| 方法 | 描述 |
|------|------|
| `new(max_hw: usize)` | 创建过滤器链 |
| `add_filter(filter: Box<dyn MessageFilter>)` | 添加过滤器 |
| `remove_filter(index: usize)` | 移除过滤器 |
| `clear()` | 清空所有过滤器 |
| `matches(&self, msg: &CanMessage) -> bool` | 检查消息是否匹配 |
| `len()` | 过滤器数量 |
| `is_empty()` | 是否为空 |

**示例**:
```rust
use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter};

let mut chain = FilterChain::new(8);

// 添加多个过滤器
chain.add_filter(Box::new(IdFilter::new(0x123)));
chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));

// 任一过滤器匹配即通过
let msg = CanMessage::new_standard(0x250, &[1])?;
assert!(chain.matches(&msg));

// 清空过滤器
chain.clear();
assert!(chain.is_empty());
```

---

## 队列 API

### BoundedQueue

有界消息队列。

```rust
pub struct BoundedQueue<T> {
    buffer: VecDeque<T>,
    capacity: usize,
    policy: QueueOverflowPolicy,
    stats: QueueStats,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(capacity: usize)` | 创建队列（默认 DropOldest） |
| `with_policy(capacity: usize, policy: QueueOverflowPolicy)` | 指定溢出策略 |

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `push(item: T)` | `Result<(), QueueError>` | 入队 |
| `pop()` | `Option<T>` | 出队 |
| `peek()` | `Option<&T>` | 查看队首 |
| `len()` | `usize` | 当前长度 |
| `capacity()` | `usize` | 容量 |
| `is_empty()` | `bool` | 是否为空 |
| `is_full()` | `bool` | 是否已满 |
| `stats()` | `&QueueStats` | 获取统计信息 |
| `clear()` | `()` | 清空队列 |

---

### QueueOverflowPolicy

队列溢出策略枚举。

```rust
pub enum QueueOverflowPolicy {
    /// 丢弃最旧的消息
    DropOldest,

    /// 丢弃最新的消息（拒绝入队）
    DropNewest,

    /// 阻塞等待（带超时）
    Block { timeout_ms: u64 },
}
```

---

### QueueStats

队列统计信息。

```rust
pub struct QueueStats {
    pub enqueued: u64,       // 入队总数
    pub dequeued: u64,       // 出队总数
    pub dropped: u64,        // 丢弃总数
    pub overflow_count: u64, // 溢出次数
}
```

---

## 监控 API

### ConnectionState

连接状态枚举。

```rust
pub enum ConnectionState {
    Connected,     // 已连接
    Disconnected,  // 已断开
    Reconnecting,  // 重连中
}
```

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `can_send()` | `bool` | 是否可以发送 |
| `can_receive()` | `bool` | 是否可以接收 |

---

### ConnectionMonitor

连接监控器。

```rust
pub struct ConnectionMonitor {
    state: ConnectionState,
    heartbeat_interval: Duration,
    reconnect_config: Option<ReconnectConfig>,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(heartbeat: Duration)` | 创建监控器 |
| `with_reconnect(heartbeat: Duration, config: ReconnectConfig)` | 带自动重连 |

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `state()` | `ConnectionState` | 获取当前状态 |
| `heartbeat_interval()` | `Duration` | 获取心跳间隔 |
| `auto_reconnect_enabled()` | `bool` | 是否启用自动重连 |
| `set_state(state: ConnectionState)` | `()` | 设置状态 |

---

### ReconnectConfig

重连配置。

```rust
pub struct ReconnectConfig {
    pub max_retries: u32,        // 最大重试次数（0=无限）
    pub retry_interval: Duration, // 重试间隔
    pub backoff_multiplier: f64,  // 退避因子
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `fixed_interval(retries: u32, interval: Duration)` | 固定间隔 |
| `exponential_backoff(retries: u32, initial: Duration, multiplier: f64)` | 指数退避 |

---

## 配置 API

### BackendConfig

后端配置。

```rust
pub struct BackendConfig {
    pub backend_name: String,
    pub channel: u8,
    pub bitrate: u32,
    pub data_bitrate: Option<u32>,  // CAN-FD 数据段波特率
    pub options: HashMap<String, String>,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(name: &str)` | 创建默认配置 |
| `from_toml(path: &Path)` | 从 TOML 文件加载 |

---

### ConfigWatcher

配置文件监视器（需要 `hot-reload` feature）。

```rust
#[cfg(feature = "hot-reload")]
pub struct ConfigWatcher {
    path: PathBuf,
    // ...
}
```

**方法**:

| 方法 | 描述 |
|------|------|
| `new(path: &Path)` | 创建监视器 |
| `start()` | 开始监视 |
| `stop()` | 停止监视 |
| `on_config_change(callback: F)` | 注册回调 |

---

## 错误类型

### CanError

CAN 操作错误枚举。

```rust
pub enum CanError {
    /// 初始化失败
    InitializationFailed { reason: String },

    /// 发送失败
    SendFailed { reason: String },

    /// 接收失败
    ReceiveFailed { reason: String },

    /// 通道未找到
    ChannelNotFound { channel: u8, max: u8 },

    /// 通道已打开
    ChannelAlreadyOpen { channel: u8 },

    /// 通道未打开
    ChannelNotOpen { channel: u8 },

    /// 总线错误
    BusError { kind: BusErrorKind },

    /// 超时
    Timeout { timeout_ms: u64 },

    /// 不支持的特性
    UnsupportedFeature { feature: String },

    /// 无效参数
    InvalidParameter { name: String, reason: String },

    /// 配置错误
    ConfigError { reason: String },

    /// 过滤器错误
    FilterError { reason: String },

    /// 队列错误
    QueueError { reason: String },

    /// 监控错误
    MonitorError { reason: String },
}
```

---

### BusErrorKind

总线错误类型。

```rust
pub enum BusErrorKind {
    BusOff,        // 总线关闭
    ErrorPassive,  // 错误被动
    ErrorWarning,  // 错误警告
    BitError,      // 位错误
    StuffError,    // 填充错误
    CrcError,      // CRC 错误
    FormError,     // 格式错误
    AckError,      // 应答错误
    Other(String), // 其他错误
}
```

---

## 周期性消息发送 API

v0.2.0 新增周期性消息发送功能（需要 `periodic` feature）。

### PeriodicMessage

周期性消息配置。

```rust
pub struct PeriodicMessage {
    id: u32,
    message: CanMessage,
    interval: Duration,
    enabled: bool,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(message: CanMessage, interval: Duration)` | 创建周期消息（间隔 1ms-10000ms） |

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `id()` | `u32` | 获取唯一标识符 |
| `message()` | `&CanMessage` | 获取 CAN 消息 |
| `interval()` | `Duration` | 获取发送间隔 |
| `is_enabled()` | `bool` | 是否启用 |
| `update_data(data: Vec<u8>)` | `Result<(), CanError>` | 更新消息数据 |
| `set_interval(interval: Duration)` | `Result<(), CanError>` | 更新发送间隔 |
| `set_enabled(enabled: bool)` | `()` | 启用/禁用发送 |

**示例**:
```rust
use canlink_hal::periodic::PeriodicMessage;
use canlink_hal::CanMessage;
use std::time::Duration;

let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
let mut periodic = PeriodicMessage::new(msg, Duration::from_millis(100))?;

// 动态更新数据
periodic.update_data(vec![0xAA, 0xBB, 0xCC, 0xDD])?;

// 动态更新间隔
periodic.set_interval(Duration::from_millis(50))?;

// 暂停发送
periodic.set_enabled(false);
```

---

### PeriodicScheduler

周期消息调度器，管理多个周期消息的发送。

```rust
pub struct PeriodicScheduler {
    // 内部使用 tokio 优先队列调度
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(backend: B, capacity: usize)` | 创建调度器（异步） |

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `add(message: PeriodicMessage)` | `Result<u32, CanError>` | 添加周期消息，返回 ID |
| `remove(id: u32)` | `Result<(), CanError>` | 移除周期消息 |
| `update_data(id: u32, data: Vec<u8>)` | `Result<(), CanError>` | 更新消息数据 |
| `update_interval(id: u32, interval: Duration)` | `Result<(), CanError>` | 更新发送间隔 |
| `set_enabled(id: u32, enabled: bool)` | `Result<(), CanError>` | 启用/禁用消息 |
| `get_stats(id: u32)` | `Option<PeriodicStats>` | 获取统计信息 |
| `list_ids()` | `Vec<u32>` | 列出所有消息 ID |
| `shutdown()` | `Result<(), CanError>` | 关闭调度器 |

**示例**:
```rust
use canlink_hal::periodic::{PeriodicScheduler, PeriodicMessage};
use canlink_hal::{CanMessage, BackendConfig, CanBackend};
use canlink_mock::MockBackend;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut backend = MockBackend::new();
    backend.initialize(&BackendConfig::new("mock"))?;
    backend.open_channel(0)?;

    // 创建调度器（最多 32 个并发消息）
    let scheduler = PeriodicScheduler::new(backend, 32).await?;

    // 添加周期消息
    let msg1 = CanMessage::new_standard(0x100, &[0x01, 0x02])?;
    let periodic1 = PeriodicMessage::new(msg1, Duration::from_millis(100))?;
    let id1 = scheduler.add(periodic1).await?;

    let msg2 = CanMessage::new_standard(0x200, &[0x03, 0x04])?;
    let periodic2 = PeriodicMessage::new(msg2, Duration::from_millis(50))?;
    let id2 = scheduler.add(periodic2).await?;

    // 运行一段时间
    tokio::time::sleep(Duration::from_secs(1)).await;

    // 动态更新
    scheduler.update_data(id1, vec![0xAA, 0xBB]).await?;
    scheduler.update_interval(id2, Duration::from_millis(200)).await?;

    // 获取统计
    if let Some(stats) = scheduler.get_stats(id1).await {
        println!("消息 {} 发送次数: {}", id1, stats.send_count());
    }

    // 关闭
    scheduler.shutdown().await?;
    Ok(())
}
```

---

### PeriodicStats

周期消息发送统计。

```rust
pub struct PeriodicStats {
    send_count: u64,
    last_send_time: Option<Instant>,
    total_interval: Duration,
    min_interval: Option<Duration>,
    max_interval: Option<Duration>,
}
```

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `new()` | `PeriodicStats` | 创建新统计实例 |
| `send_count()` | `u64` | 发送总次数 |
| `last_send_time()` | `Option<Instant>` | 最后发送时间 |
| `average_interval()` | `Option<Duration>` | 平均实际间隔 |
| `min_interval()` | `Option<Duration>` | 最小间隔 |
| `max_interval()` | `Option<Duration>` | 最大间隔 |
| `jitter()` | `Option<Duration>` | 抖动（max - min） |
| `reset()` | `()` | 重置统计 |

---

## ISO-TP 传输协议 API

v0.2.0 新增 ISO-TP (ISO 15765-2) 传输协议支持（需要 `isotp` feature）。

### IsoTpConfig

ISO-TP 通道配置。

```rust
pub struct IsoTpConfig {
    pub tx_id: u32,              // 发送 CAN ID
    pub rx_id: u32,              // 接收 CAN ID
    pub tx_extended: bool,       // TX ID 是否为扩展帧
    pub rx_extended: bool,       // RX ID 是否为扩展帧
    pub block_size: u8,          // Flow Control 块大小（0=无限制）
    pub st_min: StMin,           // Flow Control STmin
    pub rx_timeout: Duration,    // 接收超时
    pub tx_timeout: Duration,    // 发送超时（等待 FC）
    pub max_wait_count: u8,      // 最大 FC(Wait) 次数
    pub addressing_mode: AddressingMode,  // 地址模式
    pub max_buffer_size: usize,  // 最大缓冲区大小
    pub frame_size: FrameSize,   // 帧大小模式
    pub padding_byte: u8,        // 填充字节
    pub padding_enabled: bool,   // 是否启用填充
}
```

**Builder 方法**:

```rust
let config = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .block_size(8)
    .st_min(StMin::Milliseconds(10))
    .timeout(Duration::from_millis(1000))
    .max_wait_count(10)
    .addressing_mode(AddressingMode::Normal)
    .frame_size(FrameSize::Auto)
    .build()?;
```

---

### AddressingMode

ISO-TP 地址模式枚举。

```rust
pub enum AddressingMode {
    /// 普通地址模式 - CAN ID 直接标识端点
    Normal,
    /// 扩展地址模式 - 第一个数据字节为目标地址
    Extended { target_address: u8 },
    /// 混合地址模式 - 11位 CAN ID + 地址扩展字节
    Mixed { address_extension: u8 },
}
```

---

### FrameSize

帧大小模式枚举。

```rust
pub enum FrameSize {
    /// 自动检测（根据后端能力）
    Auto,
    /// 强制 CAN 2.0 模式（8 字节/帧）
    Classic8,
    /// 强制 CAN-FD 模式（最多 64 字节/帧）
    Fd64,
}
```

---

### StMin

Flow Control 分隔时间。

```rust
pub enum StMin {
    /// 毫秒（0-127ms）
    Milliseconds(u8),
    /// 微秒（100-900μs）
    Microseconds(u16),
}
```

---

### FlowStatus

Flow Control 状态。

```rust
pub enum FlowStatus {
    /// 继续发送
    ContinueToSend,
    /// 等待
    Wait,
    /// 溢出
    Overflow,
}
```

---

### IsoTpChannel

ISO-TP 通道，处理消息的分段和重组。

```rust
pub struct IsoTpChannel<B: CanBackendAsync> {
    backend: B,
    config: IsoTpConfig,
    state: IsoTpState,
}
```

**构造函数**:

| 方法 | 描述 |
|------|------|
| `new(backend: B, config: IsoTpConfig)` | 创建 ISO-TP 通道 |

**方法**:

| 方法 | 返回类型 | 描述 |
|------|----------|------|
| `send(data: &[u8])` | `Result<(), IsoTpError>` | 发送数据（自动分段） |
| `receive()` | `Result<Vec<u8>, IsoTpError>` | 接收数据（自动重组） |
| `abort()` | `()` | 中止当前传输 |
| `is_idle()` | `bool` | 通道是否空闲 |
| `config()` | `&IsoTpConfig` | 获取配置 |

**示例**:
```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig, StMin};
use canlink_mock::MockBackend;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = MockBackend::new();

    let config = IsoTpConfig::builder()
        .tx_id(0x7E0)
        .rx_id(0x7E8)
        .block_size(0)           // 无限制
        .st_min(StMin::Milliseconds(10))
        .timeout(Duration::from_millis(1000))
        .build()?;

    let mut channel = IsoTpChannel::new(backend, config)?;

    // 发送 UDS 诊断请求（自动处理分段）
    let request = vec![0x10, 0x01]; // DiagnosticSessionControl
    channel.send(&request).await?;

    // 接收响应（自动处理重组）
    let response = channel.receive().await?;
    println!("响应: {:02X?}", response);

    Ok(())
}
```

---

### IsoTpError

ISO-TP 错误类型。

```rust
pub enum IsoTpError {
    /// 无效帧格式
    InvalidFrame { reason: String },
    /// 无效 PCI 类型
    InvalidPci { pci: u8 },
    /// 序列号不匹配
    SequenceMismatch { expected: u8, actual: u8 },
    /// 接收超时
    RxTimeout { timeout_ms: u64 },
    /// 等待 FC 超时
    FcTimeout { timeout_ms: u64 },
    /// FC(Wait) 次数过多
    TooManyWaits { count: u8, max: u8 },
    /// 缓冲区溢出
    BufferOverflow { received: usize, max: usize },
    /// 远端报告溢出
    RemoteOverflow,
    /// 数据过大
    DataTooLarge { size: usize, max: usize },
    /// 数据为空
    EmptyData,
    /// 传输中止
    Aborted,
    /// 配置无效
    InvalidConfig { reason: String },
    /// 后端错误
    BackendError(CanError),
    /// 后端断开
    BackendDisconnected,
    /// 缓冲区分配失败
    BufferAllocationFailed { size: usize },
    /// 通道忙
    ChannelBusy { state: String },
    /// 非预期帧类型
    UnexpectedFrame { expected: String, actual: String },
}
```

---

## Feature Flags 汇总

| Feature | 描述 | 依赖 |
|---------|------|------|
| `async` | 异步 API 支持 | tokio |
| `async-tokio` | Tokio 异步运行时 | tokio |
| `async-async-std` | async-std 异步运行时 | async-std |
| `periodic` | 周期性消息发送 | tokio |
| `isotp` | ISO-TP 传输协议 | tokio |
| `tracing` | 结构化日志 | tracing |
| `hot-reload` | 配置热重载 | notify |
| `full` | 所有功能 | 以上全部 |

---

## 版本兼容性

- **MSRV**: Rust 1.75.0
- **平台**: Windows, Linux, macOS
- **异步运行时**: Tokio 1.x

---

## 版本历史

- **v0.2.0** (当前): 周期性消息发送、ISO-TP 传输协议、异步 API、消息过滤、连接监控、队列管理、配置热重载
- **v0.1.0**: 核心功能、Mock 后端、TSCan 后端（LibTSCAN 路径）、CLI 工具

---

## 另请参阅

- [用户指南](user-guide.md) - 详细使用教程
- [CHANGELOG](../CHANGELOG.md) - 版本更新历史
- [示例代码](../canlink-hal/examples/) - 完整示例
