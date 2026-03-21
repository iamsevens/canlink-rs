# 数据模型: 周期性消息发送与 ISO-TP 支持

**功能**: 004-periodic-and-isotp
**日期**: 2026-01-12
**阶段**: Phase 1 Design

---

## 1. 周期发送数据模型

### 1.1 核心类型

```rust
// ==================== periodic/message.rs ====================

use crate::CanMessage;
use std::time::{Duration, Instant};

/// 周期发送消息配置
#[derive(Debug, Clone)]
pub struct PeriodicMessage {
    /// 唯一标识符
    id: u32,
    /// 要发送的 CAN 消息
    message: CanMessage,
    /// 发送间隔
    interval: Duration,
    /// 是否启用
    enabled: bool,
}

impl PeriodicMessage {
    /// 创建新的周期消息
    ///
    /// # Arguments
    /// * `message` - 要周期发送的 CAN 消息
    /// * `interval` - 发送间隔 (1ms - 10000ms)
    ///
    /// # Errors
    /// 返回 `InvalidParameter` 如果间隔超出范围
    pub fn new(message: CanMessage, interval: Duration) -> Result<Self, CanError>;

    /// 获取消息 ID
    pub fn id(&self) -> u32;

    /// 获取 CAN 消息引用
    pub fn message(&self) -> &CanMessage;

    /// 获取发送间隔
    pub fn interval(&self) -> Duration;

    /// 是否启用
    pub fn is_enabled(&self) -> bool;

    /// 更新消息数据
    pub fn update_data(&mut self, data: Vec<u8>) -> Result<(), CanError>;

    /// 更新发送间隔
    pub fn set_interval(&mut self, interval: Duration) -> Result<(), CanError>;

    /// 启用/禁用
    pub fn set_enabled(&mut self, enabled: bool);
}
```

### 1.2 统计信息

```rust
// ==================== periodic/stats.rs ====================

use std::time::{Duration, Instant};

/// 周期发送统计信息
#[derive(Debug, Clone, Default)]
pub struct PeriodicStats {
    /// 发送次数
    send_count: u64,
    /// 上次发送时间
    last_send_time: Option<Instant>,
    /// 实际间隔累计（用于计算平均值）
    total_interval: Duration,
    /// 间隔样本数
    interval_samples: u64,
    /// 最小实际间隔
    min_interval: Option<Duration>,
    /// 最大实际间隔
    max_interval: Option<Duration>,
}

impl PeriodicStats {
    /// 创建新的统计实例
    pub fn new() -> Self;

    /// 记录一次发送
    pub fn record_send(&mut self, now: Instant);

    /// 获取发送次数
    pub fn send_count(&self) -> u64;

    /// 获取上次发送时间
    pub fn last_send_time(&self) -> Option<Instant>;

    /// 获取平均实际间隔
    pub fn average_interval(&self) -> Option<Duration>;

    /// 获取最小间隔
    pub fn min_interval(&self) -> Option<Duration>;

    /// 获取最大间隔
    pub fn max_interval(&self) -> Option<Duration>;

    /// 重置统计
    pub fn reset(&mut self);
}
```

### 1.3 调度器

```rust
// ==================== periodic/scheduler.rs ====================

use crate::{CanBackendAsync, CanError};
use std::time::Duration;
use tokio::sync::mpsc;

/// 调度器命令
#[derive(Debug)]
pub enum SchedulerCommand {
    /// 添加周期消息
    Add(PeriodicMessage),
    /// 移除周期消息
    Remove { id: u32 },
    /// 更新消息数据
    UpdateData { id: u32, data: Vec<u8> },
    /// 更新发送间隔
    UpdateInterval { id: u32, interval: Duration },
    /// 启用/禁用消息
    SetEnabled { id: u32, enabled: bool },
    /// 获取统计信息
    GetStats { id: u32, reply: oneshot::Sender<Option<PeriodicStats>> },
    /// 停止调度器
    Shutdown,
}

/// 周期发送调度器
pub struct PeriodicScheduler {
    /// 命令发送端
    command_tx: mpsc::Sender<SchedulerCommand>,
    /// 调度器任务句柄
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PeriodicScheduler {
    /// 创建并启动调度器
    ///
    /// # Arguments
    /// * `backend` - CAN 后端（必须实现 CanBackendAsync）
    /// * `capacity` - 最大周期消息数量（默认 32）
    pub async fn new<B: CanBackendAsync + 'static>(
        backend: B,
        capacity: usize,
    ) -> Result<Self, CanError>;

    /// 添加周期消息
    pub async fn add(&self, message: PeriodicMessage) -> Result<u32, CanError>;

    /// 移除周期消息
    pub async fn remove(&self, id: u32) -> Result<(), CanError>;

    /// 更新消息数据
    pub async fn update_data(&self, id: u32, data: Vec<u8>) -> Result<(), CanError>;

    /// 更新发送间隔
    pub async fn update_interval(&self, id: u32, interval: Duration) -> Result<(), CanError>;

    /// 启用/禁用消息
    pub async fn set_enabled(&self, id: u32, enabled: bool) -> Result<(), CanError>;

    /// 获取统计信息
    pub async fn get_stats(&self, id: u32) -> Result<Option<PeriodicStats>, CanError>;

    /// 获取所有消息 ID
    pub async fn list_ids(&self) -> Result<Vec<u32>, CanError>;

    /// 停止调度器
    pub async fn shutdown(self) -> Result<(), CanError>;
}
```

### 1.4 内部调度状态

```rust
// ==================== periodic/scheduler.rs (internal) ====================

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use tokio::time::Instant;

/// 调度条目（内部使用）
struct ScheduledEntry {
    /// 下次发送时间
    next_send: Instant,
    /// 消息 ID
    message_id: u32,
}

impl Ord for ScheduledEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 反向排序，最早的在堆顶
        other.next_send.cmp(&self.next_send)
    }
}

/// 调度器内部状态
struct SchedulerState {
    /// 消息映射
    messages: HashMap<u32, PeriodicMessage>,
    /// 统计信息映射
    stats: HashMap<u32, PeriodicStats>,
    /// 优先队列
    schedule: BinaryHeap<ScheduledEntry>,
    /// 下一个消息 ID
    next_id: u32,
    /// 最大容量
    capacity: usize,
}
```

---

## 2. ISO-TP 数据模型

### 2.1 帧类型

```rust
// ==================== isotp/frame.rs ====================

/// ISO-TP 帧类型
#[derive(Debug, Clone, PartialEq)]
pub enum IsoTpFrame {
    /// 单帧 (Single Frame)
    SingleFrame {
        /// 数据长度 (1-7 for CAN 2.0, 1-62 for CAN-FD)
        data_length: u8,
        /// 数据
        data: Vec<u8>,
    },

    /// 首帧 (First Frame)
    FirstFrame {
        /// 总数据长度 (8-4095)
        total_length: u16,
        /// 首帧数据 (6 bytes for CAN 2.0, 62 for CAN-FD)
        data: Vec<u8>,
    },

    /// 连续帧 (Consecutive Frame)
    ConsecutiveFrame {
        /// 序列号 (0-15, wraps)
        sequence_number: u8,
        /// 数据 (7 bytes for CAN 2.0, 63 for CAN-FD)
        data: Vec<u8>,
    },

    /// 流控帧 (Flow Control)
    FlowControl {
        /// 流状态
        flow_status: FlowStatus,
        /// 块大小 (0 = 无限制)
        block_size: u8,
        /// 最小分隔时间
        st_min: StMin,
    },
}

impl IsoTpFrame {
    /// 从 CAN 消息解码
    pub fn decode(data: &[u8]) -> Result<Self, IsoTpError>;

    /// 编码为 CAN 消息数据
    pub fn encode(&self) -> Vec<u8>;

    /// 获取帧类型 PCI
    pub fn pci_type(&self) -> u8;

    /// 是否为单帧
    pub fn is_single_frame(&self) -> bool;

    /// 是否为首帧
    pub fn is_first_frame(&self) -> bool;

    /// 是否为连续帧
    pub fn is_consecutive_frame(&self) -> bool;

    /// 是否为流控帧
    pub fn is_flow_control(&self) -> bool;
}
```

### 2.2 流控状态

```rust
// ==================== isotp/frame.rs ====================

/// Flow Control 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FlowStatus {
    /// 继续发送 (Clear To Send)
    ContinueToSend = 0x00,
    /// 等待
    Wait = 0x01,
    /// 溢出/中止
    Overflow = 0x02,
}

impl FlowStatus {
    /// 从字节解码
    pub fn from_byte(byte: u8) -> Result<Self, IsoTpError>;

    /// 编码为字节
    pub fn to_byte(self) -> u8;
}

/// STmin (Separation Time minimum) 编码
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StMin {
    /// 毫秒 (0-127)
    Milliseconds(u8),
    /// 微秒 (100-900, step 100)
    Microseconds(u16),
}

impl StMin {
    /// 从字节解码
    pub fn from_byte(byte: u8) -> Self;

    /// 编码为字节
    pub fn to_byte(self) -> u8;

    /// 转换为 Duration
    pub fn to_duration(self) -> Duration;

    /// 从 Duration 创建（选择最接近的编码）
    pub fn from_duration(duration: Duration) -> Self;
}
```

### 2.3 配置

```rust
// ==================== isotp/config.rs ====================

use std::time::Duration;

/// ISO-TP 地址模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddressingMode {
    /// 标准地址模式
    #[default]
    Normal,
    /// 扩展地址模式
    Extended { target_address: u8 },
    /// 混合地址模式
    Mixed { address_extension: u8 },
}

/// 帧大小模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameSize {
    /// 自动检测（根据后端能力）
    #[default]
    Auto,
    /// 强制 CAN 2.0 模式 (8 字节/帧)
    Classic8,
    /// 强制 CAN-FD 模式 (最大 64 字节/帧)
    Fd64,
}

/// ISO-TP 通道配置
#[derive(Debug, Clone)]
pub struct IsoTpConfig {
    /// 发送 CAN ID
    pub tx_id: u32,
    /// 接收 CAN ID
    pub rx_id: u32,
    /// 发送 ID 是否为扩展帧
    pub tx_extended: bool,
    /// 接收 ID 是否为扩展帧
    pub rx_extended: bool,
    /// Flow Control 块大小 (0 = 无限制)
    pub block_size: u8,
    /// Flow Control STmin
    pub st_min: StMin,
    /// 接收超时
    pub rx_timeout: Duration,
    /// 发送超时（等待 FC）
    pub tx_timeout: Duration,
    /// FC(Wait) 最大等待次数（默认 10）
    pub max_wait_count: u8,
    /// 地址模式
    pub addressing_mode: AddressingMode,
    /// 最大缓冲区大小
    pub max_buffer_size: usize,
    /// 帧大小模式
    pub frame_size: FrameSize,
    /// 填充字节（用于填充不足 8/64 字节的帧）
    pub padding_byte: u8,
    /// 是否启用填充
    pub padding_enabled: bool,
}

impl Default for IsoTpConfig {
    fn default() -> Self {
        Self {
            tx_id: 0,
            rx_id: 0,
            tx_extended: false,
            rx_extended: false,
            block_size: 0,           // 无限制
            st_min: StMin::Milliseconds(10),
            rx_timeout: Duration::from_millis(1000),
            tx_timeout: Duration::from_millis(1000),
            max_wait_count: 10,      // 默认 10 次
            addressing_mode: AddressingMode::Normal,
            max_buffer_size: 4095,   // ISO-TP 标准最大值
            frame_size: FrameSize::Auto,
            padding_byte: 0xCC,
            padding_enabled: true,
        }
    }
}

impl IsoTpConfig {
    /// 创建配置构建器
    pub fn builder() -> IsoTpConfigBuilder;

    /// 验证配置
    pub fn validate(&self) -> Result<(), IsoTpError>;
}

/// 配置构建器
#[derive(Debug, Default)]
pub struct IsoTpConfigBuilder {
    config: IsoTpConfig,
}

impl IsoTpConfigBuilder {
    pub fn tx_id(mut self, id: u32) -> Self;
    pub fn rx_id(mut self, id: u32) -> Self;
    pub fn extended_ids(mut self, extended: bool) -> Self;
    pub fn block_size(mut self, bs: u8) -> Self;
    pub fn st_min(mut self, st_min: StMin) -> Self;
    pub fn timeout(mut self, timeout: Duration) -> Self;
    pub fn addressing_mode(mut self, mode: AddressingMode) -> Self;
    pub fn frame_size(mut self, size: FrameSize) -> Self;
    pub fn build(self) -> Result<IsoTpConfig, IsoTpError>;
}
```

### 2.4 状态机

```rust
// ==================== isotp/state.rs ====================

use std::time::Instant;

/// ISO-TP 接收状态
#[derive(Debug)]
pub enum RxState {
    /// 空闲，等待 SF 或 FF
    Idle,

    /// 正在接收多帧消息
    Receiving {
        /// 接收缓冲区
        buffer: Vec<u8>,
        /// 期望的总长度
        expected_length: usize,
        /// 下一个期望的序列号
        next_sequence: u8,
        /// 当前块中已接收的帧数
        block_count: u8,
        /// 接收开始时间
        start_time: Instant,
        /// 上一帧接收时间
        last_frame_time: Instant,
    },
}

/// ISO-TP 发送状态
#[derive(Debug)]
pub enum TxState {
    /// 空闲
    Idle,

    /// 等待 Flow Control
    WaitingForFc {
        /// 发送缓冲区
        buffer: Vec<u8>,
        /// 已发送的字节偏移
        offset: usize,
        /// 下一个序列号
        next_sequence: u8,
        /// 发送开始时间
        start_time: Instant,
        /// FF 发送时间
        fc_wait_start: Instant,
    },

    /// 正在发送连续帧
    SendingCf {
        /// 发送缓冲区
        buffer: Vec<u8>,
        /// 已发送的字节偏移
        offset: usize,
        /// 下一个序列号
        next_sequence: u8,
        /// 当前块中已发送的帧数
        block_count: u8,
        /// 块大小限制 (0 = 无限制)
        block_size: u8,
        /// STmin
        st_min: Duration,
        /// 发送开始时间
        start_time: Instant,
        /// 上一帧发送时间
        last_frame_time: Instant,
    },
}

/// ISO-TP 通道状态
#[derive(Debug)]
pub struct IsoTpState {
    /// 接收状态
    pub rx: RxState,
    /// 发送状态
    pub tx: TxState,
}

impl IsoTpState {
    /// 创建新状态（空闲）
    pub fn new() -> Self;

    /// 是否空闲
    pub fn is_idle(&self) -> bool;

    /// 是否正在接收
    pub fn is_receiving(&self) -> bool;

    /// 是否正在发送
    pub fn is_sending(&self) -> bool;

    /// 重置为空闲状态
    pub fn reset(&mut self);
}
```

### 2.5 通道

```rust
// ==================== isotp/channel.rs ====================

use crate::{CanBackendAsync, CanMessage};

/// ISO-TP 传输状态回调
pub trait IsoTpCallback: Send + Sync {
    /// 传输开始
    fn on_transfer_start(&self, direction: TransferDirection, total_length: usize);

    /// 传输进行中
    fn on_transfer_progress(&self, direction: TransferDirection, bytes_transferred: usize, total: usize);

    /// 传输完成
    fn on_transfer_complete(&self, direction: TransferDirection, data: &[u8]);

    /// 传输错误
    fn on_transfer_error(&self, direction: TransferDirection, error: &IsoTpError);
}

/// 传输方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferDirection {
    Send,
    Receive,
}

/// ISO-TP 通道
pub struct IsoTpChannel<B: CanBackendAsync> {
    /// CAN 后端
    backend: B,
    /// 配置
    config: IsoTpConfig,
    /// 状态
    state: IsoTpState,
    /// 回调（可选）
    callback: Option<Box<dyn IsoTpCallback>>,
    /// 实际帧大小（根据后端能力确定）
    frame_data_size: usize,
}

impl<B: CanBackendAsync> IsoTpChannel<B> {
    /// 创建新的 ISO-TP 通道
    pub async fn new(backend: B, config: IsoTpConfig) -> Result<Self, IsoTpError>;

    /// 设置回调
    pub fn set_callback(&mut self, callback: Box<dyn IsoTpCallback>);

    /// 发送数据（自动分段）
    ///
    /// # Arguments
    /// * `data` - 要发送的数据 (1-4095 字节)
    ///
    /// # Returns
    /// 发送成功返回 Ok(())，失败返回错误
    pub async fn send(&mut self, data: &[u8]) -> Result<(), IsoTpError>;

    /// 接收数据（自动重组）
    ///
    /// # Returns
    /// 接收到完整消息返回 Ok(data)，超时或错误返回 Err
    pub async fn receive(&mut self) -> Result<Vec<u8>, IsoTpError>;

    /// 处理接收到的 CAN 消息
    ///
    /// 用于手动处理模式，将 CAN 消息传入 ISO-TP 层
    pub async fn process_message(&mut self, message: &CanMessage) -> Result<Option<Vec<u8>>, IsoTpError>;

    /// 获取当前状态
    pub fn state(&self) -> &IsoTpState;

    /// 获取配置
    pub fn config(&self) -> &IsoTpConfig;

    /// 中止当前传输
    pub fn abort(&mut self);

    /// 重置通道
    pub fn reset(&mut self);
}
```

### 2.6 错误类型

```rust
// ==================== isotp/error.rs ====================

use thiserror::Error;

/// ISO-TP 错误类型
#[derive(Debug, Error)]
pub enum IsoTpError {
    /// 无效的帧格式
    #[error("Invalid frame format: {reason}")]
    InvalidFrame { reason: String },

    /// 无效的 PCI 类型
    #[error("Invalid PCI type: 0x{pci:02X}")]
    InvalidPci { pci: u8 },

    /// 序列号错误
    #[error("Sequence number mismatch: expected {expected}, got {actual}")]
    SequenceMismatch { expected: u8, actual: u8 },

    /// 接收超时
    #[error("Receive timeout after {timeout_ms}ms")]
    RxTimeout { timeout_ms: u64 },

    /// 发送超时（等待 FC）
    #[error("Timeout waiting for Flow Control after {timeout_ms}ms")]
    FcTimeout { timeout_ms: u64 },

    /// 连续 FC(Wait) 超过最大次数
    #[error("Too many FC(Wait) responses: {count} exceeds max {max}")]
    TooManyWaits { count: u8, max: u8 },

    /// 缓冲区溢出
    #[error("Buffer overflow: received {received} bytes, max {max}")]
    BufferOverflow { received: usize, max: usize },

    /// 对方报告溢出
    #[error("Remote reported overflow")]
    RemoteOverflow,

    /// 数据太大
    #[error("Data too large: {size} bytes, max {max}")]
    DataTooLarge { size: usize, max: usize },

    /// 数据为空
    #[error("Data is empty")]
    EmptyData,

    /// 传输中止
    #[error("Transfer aborted")]
    Aborted,

    /// 无效配置
    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    /// 后端错误
    #[error("Backend error: {0}")]
    BackendError(#[from] crate::CanError),

    /// 后端断开连接
    #[error("Backend disconnected")]
    BackendDisconnected,

    /// 缓冲区分配失败
    #[error("Buffer allocation failed: requested {size} bytes")]
    BufferAllocationFailed { size: usize },

    /// 通道忙
    #[error("Channel busy: {state}")]
    ChannelBusy { state: String },

    /// 意外的帧类型
    #[error("Unexpected frame type: expected {expected}, got {actual}")]
    UnexpectedFrame { expected: String, actual: String },
}
```

---

## 3. 类型关系图

```
┌─────────────────────────────────────────────────────────────────┐
│                        Periodic Module                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐     ┌──────────────────┐                  │
│  │ PeriodicMessage │────▶│  PeriodicStats   │                  │
│  └────────┬────────┘     └──────────────────┘                  │
│           │                                                     │
│           │ manages                                             │
│           ▼                                                     │
│  ┌─────────────────────┐     ┌────────────────────┐            │
│  │ PeriodicScheduler   │────▶│ SchedulerCommand   │            │
│  └─────────┬───────────┘     └────────────────────┘            │
│            │                                                    │
│            │ uses                                               │
│            ▼                                                    │
│  ┌─────────────────────┐                                       │
│  │  CanBackendAsync    │                                       │
│  └─────────────────────┘                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                         ISO-TP Module                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐       │
│  │ IsoTpFrame  │     │ FlowStatus  │     │   StMin     │       │
│  └──────┬──────┘     └─────────────┘     └─────────────┘       │
│         │                   │                   │               │
│         └───────────────────┴───────────────────┘               │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────┐       │
│  │                    IsoTpConfig                       │       │
│  │  ┌─────────────────┐  ┌───────────┐  ┌───────────┐  │       │
│  │  │ AddressingMode  │  │ FrameSize │  │  Timeouts │  │       │
│  │  └─────────────────┘  └───────────┘  └───────────┘  │       │
│  └──────────────────────────┬──────────────────────────┘       │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────┐       │
│  │                   IsoTpChannel                       │       │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │       │
│  │  │ IsoTpState  │  │   Backend   │  │  Callback   │  │       │
│  │  │ ┌─────────┐ │  └─────────────┘  └─────────────┘  │       │
│  │  │ │ RxState │ │                                    │       │
│  │  │ │ TxState │ │                                    │       │
│  │  │ └─────────┘ │                                    │       │
│  │  └─────────────┘                                    │       │
│  └─────────────────────────────────────────────────────┘       │
│                             │                                   │
│                             ▼                                   │
│  ┌─────────────────────────────────────────────────────┐       │
│  │                    IsoTpError                        │       │
│  └─────────────────────────────────────────────────────┘       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. 模块导出

```rust
// ==================== lib.rs ====================

// 周期发送模块
pub mod periodic;
pub use periodic::{PeriodicMessage, PeriodicScheduler, PeriodicStats, SchedulerCommand};

// ISO-TP 模块 (feature gated)
#[cfg(feature = "isotp")]
pub mod isotp;
#[cfg(feature = "isotp")]
pub use isotp::{
    AddressingMode, FlowStatus, FrameSize, IsoTpChannel, IsoTpConfig,
    IsoTpConfigBuilder, IsoTpError, IsoTpFrame, IsoTpState, StMin,
    TransferDirection, IsoTpCallback,
};
```

---

**数据模型版本**: 1.0.0
**创建日期**: 2026-01-12
