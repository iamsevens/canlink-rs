# 数据模型: 异步 API 与消息过滤

**功能分支**: `003-async-and-filtering`
**创建日期**: 2026-01-10
**状态**: 草稿

---

## 实体概览

```
┌─────────────────────────────────────────────────────────────────┐
│                        应用层                                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     FilterChain                                  │
│  ┌─────────────────┐  ┌─────────────────┐                       │
│  │ HardwareFilters │  │ SoftwareFilters │                       │
│  │ (优先使用)       │  │ (自动回退)       │                       │
│  └─────────────────┘  └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     BoundedQueue                                 │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ QueueOverflowPolicy: DropOldest | DropNewest | Block    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   ConnectionMonitor                              │
│  ┌─────────────────┐  ┌─────────────────┐                       │
│  │ HeartbeatCheck  │  │ ReconnectConfig │                       │
│  │ (心跳检测)       │  │ (可选自动重连)   │                       │
│  └─────────────────┘  └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 核心实体

### 1. MessageFilter (消息过滤器)

**用途**: 定义消息过滤的统一接口

```rust
/// 消息过滤器 trait
///
/// 所有过滤器（硬件和软件）都必须实现此 trait。
/// 过滤器按优先级排序，硬件过滤器优先执行。
pub trait MessageFilter: Send + Sync {
    /// 检查消息是否通过过滤器
    ///
    /// # 返回值
    /// - `true`: 消息通过过滤器，应该被处理
    /// - `false`: 消息被过滤，应该被丢弃
    fn matches(&self, message: &CanMessage) -> bool;

    /// 过滤器优先级（用于排序）
    ///
    /// 数值越小优先级越高。默认为 0。
    fn priority(&self) -> u32 { 0 }

    /// 是否为硬件过滤器
    ///
    /// 硬件过滤器由硬件执行，性能更高。
    /// 当硬件过滤器数量超过硬件限制时，自动回退到软件过滤。
    fn is_hardware(&self) -> bool { false }
}
```

**字段说明**:
| 方法 | 返回类型 | 说明 |
|------|----------|------|
| `matches` | `bool` | 检查消息是否匹配过滤条件 |
| `priority` | `u32` | 过滤器优先级，数值越小越优先 |
| `is_hardware` | `bool` | 标识是否为硬件过滤器 |

**验证规则**:
- 过滤器必须是线程安全的（`Send + Sync`）
- `matches` 方法应该是无副作用的纯函数
- 性能目标：软件过滤 < 10 μs/消息

---

### 2. IdFilter (ID 过滤器)

**用途**: 基于 CAN ID 的过滤器实现

```rust
/// ID 过滤器
///
/// 支持精确匹配和掩码匹配两种模式。
#[derive(Debug, Clone)]
pub struct IdFilter {
    /// 过滤器 ID
    pub id: u32,
    /// 掩码（用于范围匹配）
    ///
    /// 匹配规则: (message.id & mask) == (id & mask)
    pub mask: u32,
    /// 是否匹配扩展帧
    pub extended: bool,
}
```

**字段说明**:
| 字段 | 类型 | 说明 | 验证规则 |
|------|------|------|----------|
| `id` | `u32` | 过滤器 ID | 标准帧: 0-0x7FF, 扩展帧: 0-0x1FFFFFFF |
| `mask` | `u32` | 掩码 | 与 id 范围相同 |
| `extended` | `bool` | 是否扩展帧 | - |

**匹配算法**:
```rust
fn matches(&self, message: &CanMessage) -> bool {
    let msg_id = message.id().raw();
    let msg_extended = message.id().is_extended();

    if self.extended != msg_extended {
        return false;
    }

    (msg_id & self.mask) == (self.id & self.mask)
}
```

---

### 3. RangeFilter (范围过滤器)

**用途**: 基于 ID 范围的过滤器

```rust
/// 范围过滤器
///
/// 匹配指定范围内的所有 CAN ID。
#[derive(Debug, Clone)]
pub struct RangeFilter {
    /// 起始 ID（包含）
    pub start_id: u32,
    /// 结束 ID（包含）
    pub end_id: u32,
    /// 是否匹配扩展帧
    pub extended: bool,
}
```

**验证规则**:
- `start_id <= end_id`
- ID 范围必须在有效范围内

---

### 4. FilterChain (过滤器链)

**用途**: 管理和执行多个过滤器

```rust
/// 过滤器链
///
/// 组合多个过滤器，按优先级顺序执行。
/// 硬件过滤器优先，超出硬件限制时自动回退到软件过滤。
pub struct FilterChain {
    /// 硬件过滤器列表
    hardware_filters: Vec<Box<dyn MessageFilter>>,
    /// 软件过滤器列表
    software_filters: Vec<Box<dyn MessageFilter>>,
    /// 硬件支持的最大过滤器数量
    max_hardware_filters: usize,
}
```

**字段说明**:
| 字段 | 类型 | 说明 |
|------|------|------|
| `hardware_filters` | `Vec<Box<dyn MessageFilter>>` | 硬件过滤器列表 |
| `software_filters` | `Vec<Box<dyn MessageFilter>>` | 软件过滤器列表 |
| `max_hardware_filters` | `usize` | 硬件支持的最大过滤器数量 |

**行为规则**:
- 添加过滤器时，优先放入硬件过滤器列表
- 硬件过滤器满时，自动回退到软件过滤器
- 执行时先检查硬件过滤器，再检查软件过滤器
- 任一过滤器匹配即通过（OR 逻辑）

---

### 5. QueueOverflowPolicy (队列溢出策略)

**用途**: 定义队列满时的处理策略

```rust
/// 队列溢出策略
///
/// 当消息队列满时，决定如何处理新消息。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QueueOverflowPolicy {
    /// 丢弃最旧的消息（默认）
    ///
    /// 移除队列头部的消息，为新消息腾出空间。
    /// 适用于实时性要求高的场景。
    #[default]
    DropOldest,

    /// 丢弃最新的消息
    ///
    /// 拒绝新消息，保留队列中的旧消息。
    /// 适用于需要保留历史数据的场景。
    DropNewest,

    /// 阻塞等待，带超时
    ///
    /// 等待队列有空间，超时后返回错误。
    /// 适用于不允许丢失消息的场景。
    Block {
        /// 等待超时时间
        timeout: Duration,
    },
}
```

**使用场景**:
| 策略 | 适用场景 | 优点 | 缺点 |
|------|----------|------|------|
| `DropOldest` | 实时监控、诊断 | 始终获取最新数据 | 可能丢失历史数据 |
| `DropNewest` | 数据记录、回放 | 保留完整历史 | 可能错过新数据 |
| `Block` | 关键消息传输 | 不丢失消息 | 可能阻塞发送方 |

---

### 6. BoundedQueue (有界队列)

**用途**: 带容量限制的消息队列

```rust
/// 有界消息队列
///
/// 线程安全的消息队列，支持配置溢出策略。
pub struct BoundedQueue<T> {
    /// 内部存储
    inner: VecDeque<T>,
    /// 队列容量
    capacity: usize,
    /// 溢出策略
    policy: QueueOverflowPolicy,
    /// 统计信息
    stats: QueueStats,
}

/// 队列统计信息
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// 入队消息总数
    pub enqueued: u64,
    /// 出队消息总数
    pub dequeued: u64,
    /// 丢弃消息总数
    pub dropped: u64,
    /// 阻塞超时次数
    pub timeouts: u64,
}
```

**字段说明**:
| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| `capacity` | `usize` | 队列容量（消息数量） | 1000 |
| `policy` | `QueueOverflowPolicy` | 溢出策略 | `DropOldest` |
| `stats` | `QueueStats` | 统计信息 | - |

**验证规则**:
- `capacity > 0`
- 容量调整时根据 policy 处理多余消息

---

### 7. ConnectionMonitor (连接监控器)

**用途**: 监控后端连接状态

```rust
/// 连接监控器
///
/// 通过心跳检测监控连接状态，支持可选的自动重连。
pub struct ConnectionMonitor {
    /// 后端引用
    backend: Arc<Mutex<dyn CanBackend>>,
    /// 心跳检测间隔
    heartbeat_interval: Duration,
    /// 重连配置（可选）
    reconnect_config: Option<ReconnectConfig>,
    /// 当前连接状态
    state: ConnectionState,
}

/// 重连配置
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔
    pub retry_interval: Duration,
    /// 退避乘数
    pub backoff_multiplier: f32,
}

/// 连接状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// 已连接
    Connected,
    /// 已断开
    Disconnected,
    /// 重连中
    Reconnecting,
}
```

**字段说明**:
| 字段 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| `heartbeat_interval` | `Duration` | 心跳间隔 | 1 秒 |
| `reconnect_config` | `Option<ReconnectConfig>` | 重连配置 | `None`（禁用） |

**行为规则**:
- 默认不自动重连（`reconnect_config = None`）
- 心跳失败时触发断开事件
- 重连使用指数退避策略

---

### 8. FilterConfig (过滤器配置)

**用途**: 从配置文件加载过滤器

```rust
/// 过滤器配置
///
/// 支持从 TOML 配置文件加载。
#[derive(Debug, Clone, Deserialize)]
pub struct FilterConfig {
    /// ID 过滤器列表
    #[serde(default)]
    pub id_filters: Vec<IdFilterConfig>,
    /// 范围过滤器列表
    #[serde(default)]
    pub range_filters: Vec<RangeFilterConfig>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IdFilterConfig {
    pub id: u32,
    #[serde(default = "default_mask")]
    pub mask: u32,
    #[serde(default)]
    pub extended: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RangeFilterConfig {
    pub start_id: u32,
    pub end_id: u32,
    #[serde(default)]
    pub extended: bool,
}
```

**配置示例**:
```toml
[[filters.id_filters]]
id = 0x123
mask = 0x7FF
extended = false

[[filters.range_filters]]
start_id = 0x200
end_id = 0x2FF
extended = false
```

---

### 9. QueueConfig (队列配置)

**用途**: 队列参数配置

```rust
/// 队列配置
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    /// 队列容量（消息数量）
    #[serde(default = "default_capacity")]
    pub capacity: usize,
    /// 溢出策略
    #[serde(default)]
    pub overflow_policy: QueueOverflowPolicyConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QueueOverflowPolicyConfig {
    #[default]
    DropOldest,
    DropNewest,
    Block {
        #[serde(default = "default_timeout_ms")]
        timeout_ms: u64,
    },
}

fn default_capacity() -> usize { 1000 }
fn default_timeout_ms() -> u64 { 1000 }
```

**配置示例**:
```toml
[queue]
capacity = 2000

[queue.overflow_policy]
type = "block"
timeout_ms = 500
```

---

### 10. MonitorConfig (监控配置)

**用途**: 连接监控参数配置

```rust
/// 监控配置
#[derive(Debug, Clone, Deserialize)]
pub struct MonitorConfig {
    /// 心跳间隔（毫秒）
    #[serde(default = "default_heartbeat_ms")]
    pub heartbeat_interval_ms: u64,
    /// 自动重连配置（可选）
    pub reconnect: Option<ReconnectConfigFile>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReconnectConfigFile {
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    #[serde(default = "default_retry_interval_ms")]
    pub retry_interval_ms: u64,
    /// 退避乘数
    #[serde(default = "default_backoff")]
    pub backoff_multiplier: f32,
}

fn default_heartbeat_ms() -> u64 { 1000 }
fn default_max_retries() -> u32 { 3 }
fn default_retry_interval_ms() -> u64 { 1000 }
fn default_backoff() -> f32 { 2.0 }
```

**配置示例**:
```toml
[monitor]
heartbeat_interval_ms = 500

[monitor.reconnect]
max_retries = 5
retry_interval_ms = 2000
backoff_multiplier = 1.5
```

---

## 实体关系图

```
┌─────────────────┐     implements     ┌─────────────────┐
│  MessageFilter  │◄──────────────────│    IdFilter     │
│     (trait)     │                    └─────────────────┘
└─────────────────┘
        ▲                              ┌─────────────────┐
        │          implements          │   RangeFilter   │
        └──────────────────────────────└─────────────────┘

        │
        │ contains
        ▼
┌─────────────────┐
│   FilterChain   │
└─────────────────┘
        │
        │ feeds into
        ▼
┌─────────────────┐     uses      ┌─────────────────────┐
│  BoundedQueue   │──────────────►│ QueueOverflowPolicy │
└─────────────────┘               └─────────────────────┘
        │
        │ monitored by
        ▼
┌─────────────────┐     uses      ┌─────────────────┐
│ConnectionMonitor│──────────────►│ ReconnectConfig │
└─────────────────┘               └─────────────────┘
```

---

## 状态转换

### ConnectionState 状态机

```
                    ┌─────────────┐
                    │             │
         ┌─────────►│  Connected  │◄─────────┐
         │          │             │          │
         │          └──────┬──────┘          │
         │                 │                 │
         │    heartbeat    │                 │
         │    失败         │                 │  重连成功
         │                 ▼                 │
         │          ┌─────────────┐          │
         │          │             │          │
         │          │Disconnected │──────────┤
         │          │             │          │
         │          └──────┬──────┘          │
         │                 │                 │
         │    启用自动     │                 │
         │    重连         │                 │
         │                 ▼                 │
         │          ┌─────────────┐          │
         │          │             │          │
         └──────────│Reconnecting │──────────┘
        重连失败    │             │
        (达到上限)  └─────────────┘
```

---

## 配置文件完整示例

```toml
# canlink.toml - 异步 API 与消息过滤配置

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

---

## 性能考虑

### 内存布局

| 实体 | 预估大小 | 说明 |
|------|----------|------|
| `IdFilter` | 12 bytes | 3 个 u32 字段 |
| `RangeFilter` | 12 bytes | 3 个 u32 字段 |
| `QueueOverflowPolicy` | 16 bytes | enum + Duration |
| `BoundedQueue<CanMessage>` | ~80 bytes + 数据 | 不含消息数据 |
| `ConnectionMonitor` | ~128 bytes | 含 Arc 引用 |

### 性能目标

| 操作 | 目标 | 说明 |
|------|------|------|
| 软件过滤 | < 10 μs/消息 | 使用位运算优化 |
| 队列入队 | O(1) | VecDeque 实现 |
| 队列出队 | O(1) | VecDeque 实现 |
| 心跳检测 | < 1 ms | 简单状态检查 |
