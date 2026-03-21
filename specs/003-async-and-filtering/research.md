# 研究文档: 异步 API 与消息过滤

**功能分支**: `003-async-and-filtering`
**创建日期**: 2026-01-10
**状态**: 完成

---

## 研究主题

### 1. 消息过滤器设计模式

**Decision**: 使用 trait 对象 + 组合模式

**Rationale**:
- `MessageFilter` trait 定义统一接口，支持多种过滤器类型
- 使用 `FilterChain` 组合多个过滤器，按添加顺序执行
- 硬件过滤器优先，软件过滤器作为后备

**Alternatives considered**:
1. **枚举模式**: 所有过滤器类型在一个枚举中
   - 优点: 无动态分发开销
   - 缺点: 不可扩展，添加新类型需修改枚举
2. **泛型模式**: 使用泛型参数而非 trait 对象
   - 优点: 零成本抽象
   - 缺点: 无法在运行时动态添加/移除过滤器

**Implementation**:
```rust
pub trait MessageFilter: Send + Sync {
    /// 检查消息是否通过过滤器
    fn matches(&self, message: &CanMessage) -> bool;

    /// 过滤器优先级（用于排序）
    fn priority(&self) -> u32 { 0 }

    /// 是否为硬件过滤器
    fn is_hardware(&self) -> bool { false }
}
```

---

### 2. 队列溢出策略实现

**Decision**: 使用枚举 + 策略模式

**Rationale**:
- `QueueOverflowPolicy` 枚举清晰表达三种策略
- 策略在队列创建时配置，运行时不可变
- 与 `BoundedQueue` 紧密集成

**Alternatives considered**:
1. **回调模式**: 用户提供溢出处理回调
   - 优点: 最大灵活性
   - 缺点: 增加复杂性，可能引入性能问题
2. **配置文件模式**: 仅通过配置文件设置
   - 优点: 简单
   - 缺点: 无法在代码中动态配置

**Implementation**:
```rust
#[derive(Debug, Clone, Copy, Default)]
pub enum QueueOverflowPolicy {
    /// 丢弃最旧的消息（默认）
    #[default]
    DropOldest,
    /// 丢弃最新的消息
    DropNewest,
    /// 阻塞等待，带超时
    Block { timeout: Duration },
}
```

---

### 3. 日志框架选择

**Decision**: 使用 `tracing` 框架

**Rationale**:
- 与 tokio 生态系统完美兼容
- 支持结构化日志和 span（适合异步代码调试）
- 零成本抽象：未启用时无运行时开销
- 广泛的生态系统支持（tracing-subscriber, tracing-appender 等）

**Alternatives considered**:
1. **log crate**: Rust 标准日志门面
   - 优点: 更轻量，生态更广泛
   - 缺点: 不支持 span，异步代码调试困难
2. **slog**: 结构化日志
   - 优点: 功能强大
   - 缺点: API 复杂，与 tokio 集成不如 tracing

**Implementation**:
```rust
// 通过 feature flag 启用
#[cfg(feature = "tracing")]
use tracing::{debug, error, info, instrument, warn};

#[cfg(feature = "tracing")]
#[instrument(skip(self))]
pub fn send_message(&mut self, message: &CanMessage) -> CanResult<()> {
    info!(id = %message.id(), "Sending CAN message");
    // ...
}
```

---

### 4. 配置热重载实现

**Decision**: 使用 `notify` crate 监听文件变化

**Rationale**:
- `notify` 是 Rust 生态中最成熟的文件监控库
- 跨平台支持（Windows, Linux, macOS）
- 支持防抖动（debounce）避免频繁重载

**Alternatives considered**:
1. **轮询模式**: 定期检查文件修改时间
   - 优点: 简单，无外部依赖
   - 缺点: 延迟高，CPU 开销
2. **inotify/FSEvents 直接调用**: 平台特定 API
   - 优点: 最低延迟
   - 缺点: 需要为每个平台编写代码

**Implementation**:
```rust
pub struct ConfigWatcher {
    watcher: RecommendedWatcher,
    config_path: PathBuf,
    debounce_duration: Duration,
}

impl ConfigWatcher {
    pub fn new(config_path: PathBuf) -> CanResult<Self> {
        let watcher = notify::recommended_watcher(|res| {
            // 处理文件变化事件
        })?;
        // ...
    }
}
```

---

### 5. 连接监控与自动重连

**Decision**: 使用心跳检测 + 可配置重连策略

**Rationale**:
- 心跳检测可以及时发现连接断开
- 重连策略可配置（重试次数、间隔）
- 默认不自动重连，避免意外行为

**Alternatives considered**:
1. **被动检测**: 仅在操作失败时检测断开
   - 优点: 简单，无额外开销
   - 缺点: 检测延迟，用户体验差
2. **始终自动重连**: 断开后立即重连
   - 优点: 用户无感知
   - 缺点: 可能掩盖硬件问题，不适合所有场景

**Implementation**:
```rust
pub struct ConnectionMonitor {
    backend: Arc<Mutex<dyn CanBackend>>,
    heartbeat_interval: Duration,
    reconnect_config: Option<ReconnectConfig>,
}

pub struct ReconnectConfig {
    pub max_retries: u32,
    pub retry_interval: Duration,
    pub backoff_multiplier: f32,
}
```

---

### 6. 硬件过滤器抽象

**Decision**: 通过 `HardwareCapability` 查询过滤器能力，自动回退到软件过滤

**Rationale**:
- 不同硬件支持的过滤器数量和类型不同
- 透明回退：用户无需关心硬件限制
- 保持 API 一致性

**Alternatives considered**:
1. **严格模式**: 超过硬件限制时返回错误
   - 优点: 明确的行为
   - 缺点: 用户需要处理硬件差异
2. **仅软件过滤**: 不使用硬件过滤
   - 优点: 最简单
   - 缺点: 浪费硬件能力，高流量时 CPU 负载高

**Implementation**:
```rust
pub struct FilterChain {
    hardware_filters: Vec<Box<dyn MessageFilter>>,
    software_filters: Vec<Box<dyn MessageFilter>>,
    max_hardware_filters: usize,
}

impl FilterChain {
    pub fn add_filter(&mut self, filter: Box<dyn MessageFilter>) {
        if filter.is_hardware() && self.hardware_filters.len() < self.max_hardware_filters {
            self.hardware_filters.push(filter);
        } else {
            // 回退到软件过滤
            self.software_filters.push(filter);
        }
    }
}
```

---

### 7. 内存压力处理

**Decision**: 动态调整队列大小 + 触发 QueueOverflowPolicy

**Rationale**:
- 内存压力时减小队列大小，释放内存
- 使用现有的 QueueOverflowPolicy 处理溢出
- 避免 OOM，保持系统稳定

**Alternatives considered**:
1. **固定队列大小**: 不响应内存压力
   - 优点: 行为可预测
   - 缺点: 可能导致 OOM
2. **完全丢弃队列**: 内存压力时清空队列
   - 优点: 快速释放内存
   - 缺点: 数据丢失严重

**Implementation**:
```rust
impl BoundedQueue {
    pub fn adjust_capacity(&mut self, new_capacity: usize) {
        if new_capacity < self.len() {
            // 根据 policy 处理多余的消息
            while self.len() > new_capacity {
                match self.policy {
                    QueueOverflowPolicy::DropOldest => { self.pop_front(); }
                    QueueOverflowPolicy::DropNewest => { self.pop_back(); }
                    _ => break,
                }
            }
        }
        self.capacity = new_capacity;
    }
}
```

---

## 依赖分析

### 新增依赖

| 依赖 | 版本 | 用途 | 必需/可选 |
|------|------|------|----------|
| tracing | 0.1 | 日志框架 | 可选 (feature: tracing) |
| tracing-subscriber | 0.3 | 日志订阅者 | 可选 (feature: tracing) |
| notify | 6.0 | 文件监控 | 可选 (feature: hot-reload) |

### Feature Flags 设计

```toml
[features]
default = []
tracing = ["dep:tracing", "dep:tracing-subscriber"]
hot-reload = ["dep:notify"]
full = ["tracing", "hot-reload"]
```

---

## 性能考虑

### 软件过滤性能

- **目标**: < 10 μs/消息
- **策略**:
  - 使用位运算进行掩码匹配
  - 避免动态内存分配
  - 使用 `#[inline]` 优化热路径

### 队列性能

- **目标**: O(1) 入队/出队
- **策略**:
  - 使用 `VecDeque` 作为底层存储
  - 预分配容量避免重新分配

### 日志性能

- **目标**: 未启用时零开销
- **策略**:
  - 使用 feature flag 条件编译
  - 使用 `tracing` 的零成本抽象

---

## 风险缓解

| 风险 | 缓解措施 |
|------|----------|
| 硬件过滤器兼容性 | 软件过滤作为后备，透明回退 |
| 配置热重载竞态 | 使用 RwLock 保护配置，原子更新 |
| 内存泄漏 | 所有资源类型实现 Drop，使用 miri 测试 |
| 性能回归 | 建立基准测试，CI 中持续监控 |

---

## 结论

所有技术决策已完成，无 NEEDS CLARIFICATION 项。可以进入阶段 1 设计。
