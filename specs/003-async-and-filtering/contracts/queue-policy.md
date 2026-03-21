# API 契约: QueueOverflowPolicy 与 BoundedQueue

**功能分支**: `003-async-and-filtering`
**创建日期**: 2026-01-10
**状态**: 草稿

---

## 概述

定义消息队列的溢出策略和有界队列实现。用户可以根据应用场景选择合适的策略。

## QueueOverflowPolicy 枚举

```rust
use std::time::Duration;

/// 队列溢出策略
///
/// 当消息队列满时，决定如何处理新消息。
/// 用户可以根据应用场景选择合适的策略。
///
/// # 策略选择指南
///
/// | 策略 | 适用场景 | 特点 |
/// |------|----------|------|
/// | `DropOldest` | 实时监控、诊断 | 始终获取最新数据 |
/// | `DropNewest` | 数据记录、回放 | 保留完整历史 |
/// | `Block` | 关键消息传输 | 不丢失消息，可能阻塞 |
///
/// # 示例
///
/// ```rust
/// use canlink_hal::queue::QueueOverflowPolicy;
/// use std::time::Duration;
///
/// // 默认策略：丢弃最旧消息
/// let policy = QueueOverflowPolicy::default();
///
/// // 丢弃最新消息
/// let policy = QueueOverflowPolicy::DropNewest;
///
/// // 阻塞等待，超时 100ms
/// let policy = QueueOverflowPolicy::Block {
///     timeout: Duration::from_millis(100),
/// };
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOverflowPolicy {
    /// 丢弃最旧的消息（默认）
    ///
    /// 当队列满时，移除队列头部（最旧）的消息，
    /// 为新消息腾出空间。
    ///
    /// # 适用场景
    ///
    /// - 实时监控系统
    /// - 诊断工具
    /// - 需要最新数据的应用
    ///
    /// # 行为
    ///
    /// 1. 检测到队列满
    /// 2. 移除队列头部消息
    /// 3. 将新消息添加到队列尾部
    /// 4. 更新统计信息（dropped 计数器 +1）
    DropOldest,

    /// 丢弃最新的消息
    ///
    /// 当队列满时，拒绝新消息，保留队列中的旧消息。
    ///
    /// # 适用场景
    ///
    /// - 数据记录系统
    /// - 消息回放工具
    /// - 需要保留历史数据的应用
    ///
    /// # 行为
    ///
    /// 1. 检测到队列满
    /// 2. 丢弃新消息（不入队）
    /// 3. 更新统计信息（dropped 计数器 +1）
    DropNewest,

    /// 阻塞等待，带超时
    ///
    /// 当队列满时，等待队列有空间。
    /// 如果超时仍无空间，返回 `QueueFull` 错误。
    ///
    /// # 适用场景
    ///
    /// - 关键消息传输
    /// - 不允许丢失消息的应用
    /// - 生产者-消费者模式
    ///
    /// # 行为
    ///
    /// 1. 检测到队列满
    /// 2. 等待指定时间
    /// 3. 如果有空间，添加消息
    /// 4. 如果超时，返回 `QueueError::QueueFull`
    ///
    /// # 注意
    ///
    /// 使用此策略时，确保有消费者在消费消息，
    /// 否则可能导致发送方长时间阻塞。
    Block {
        /// 等待超时时间
        ///
        /// 建议值：100ms - 1000ms
        timeout: Duration,
    },
}

impl Default for QueueOverflowPolicy {
    fn default() -> Self {
        Self::DropOldest
    }
}
```

## BoundedQueue 结构体

```rust
use std::collections::VecDeque;

/// 有界消息队列
///
/// 线程安全的消息队列，支持配置溢出策略。
/// 使用 `VecDeque` 作为底层存储，保证 O(1) 的入队和出队操作。
///
/// # 泛型参数
///
/// * `T` - 队列元素类型，通常是 `CanMessage`
///
/// # 线程安全
///
/// `BoundedQueue` 本身不是线程安全的。
/// 在多线程环境中，请使用 `Arc<Mutex<BoundedQueue<T>>>`。
///
/// # 示例
///
/// ```rust
/// use canlink_hal::queue::{BoundedQueue, QueueOverflowPolicy};
/// use canlink_hal::message::CanMessage;
///
/// // 创建容量为 100 的队列，使用默认策略
/// let mut queue: BoundedQueue<CanMessage> = BoundedQueue::new(100);
///
/// // 创建使用自定义策略的队列
/// let mut queue: BoundedQueue<CanMessage> = BoundedQueue::with_policy(
///     100,
///     QueueOverflowPolicy::DropNewest,
/// );
///
/// // 入队
/// queue.push(message)?;
///
/// // 出队
/// if let Some(msg) = queue.pop() {
///     // 处理消息
/// }
/// ```
pub struct BoundedQueue<T> {
    inner: VecDeque<T>,
    capacity: usize,
    policy: QueueOverflowPolicy,
    stats: QueueStats,
}

/// 队列统计信息
///
/// 记录队列的操作统计，用于监控和调试。
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    /// 入队消息总数
    pub enqueued: u64,
    /// 出队消息总数
    pub dequeued: u64,
    /// 丢弃消息总数（由于溢出策略）
    pub dropped: u64,
    /// 阻塞超时次数（仅 Block 策略）
    pub timeouts: u64,
}

impl<T> BoundedQueue<T> {
    /// 创建新的有界队列
    ///
    /// 使用默认溢出策略（`DropOldest`）。
    ///
    /// # 参数
    ///
    /// * `capacity` - 队列容量（消息数量）
    ///
    /// # Panics
    ///
    /// 如果 `capacity == 0` 则 panic。
    ///
    /// # 示例
    ///
    /// ```rust
    /// let queue: BoundedQueue<CanMessage> = BoundedQueue::new(1000);
    /// ```
    pub fn new(capacity: usize) -> Self {
        Self::with_policy(capacity, QueueOverflowPolicy::default())
    }

    /// 创建使用指定策略的有界队列
    ///
    /// # 参数
    ///
    /// * `capacity` - 队列容量（消息数量）
    /// * `policy` - 溢出策略
    ///
    /// # Panics
    ///
    /// 如果 `capacity == 0` 则 panic。
    pub fn with_policy(capacity: usize, policy: QueueOverflowPolicy) -> Self {
        assert!(capacity > 0, "Queue capacity must be greater than 0");
        Self {
            inner: VecDeque::with_capacity(capacity),
            capacity,
            policy,
            stats: QueueStats::default(),
        }
    }

    /// 将元素添加到队列
    ///
    /// 根据溢出策略处理队列满的情况。
    ///
    /// # 参数
    ///
    /// * `item` - 要添加的元素
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 成功入队
    /// - `Err(QueueError::QueueFull)` - 队列满且策略为 Block 且超时
    ///
    /// # 示例
    ///
    /// ```rust
    /// match queue.push(message) {
    ///     Ok(()) => println!("Message enqueued"),
    ///     Err(QueueError::QueueFull) => println!("Queue full, message dropped"),
    /// }
    /// ```
    pub fn push(&mut self, item: T) -> Result<(), QueueError> {
        if self.inner.len() >= self.capacity {
            match self.policy {
                QueueOverflowPolicy::DropOldest => {
                    self.inner.pop_front();
                    self.stats.dropped += 1;
                }
                QueueOverflowPolicy::DropNewest => {
                    self.stats.dropped += 1;
                    return Ok(()); // 丢弃新消息，但返回 Ok
                }
                QueueOverflowPolicy::Block { timeout } => {
                    // 注意：实际实现需要条件变量支持
                    // 这里简化为立即返回错误
                    self.stats.timeouts += 1;
                    return Err(QueueError::QueueFull { timeout });
                }
            }
        }

        self.inner.push_back(item);
        self.stats.enqueued += 1;
        Ok(())
    }

    /// 从队列取出元素
    ///
    /// # 返回值
    ///
    /// - `Some(T)` - 成功取出元素
    /// - `None` - 队列为空
    pub fn pop(&mut self) -> Option<T> {
        let item = self.inner.pop_front();
        if item.is_some() {
            self.stats.dequeued += 1;
        }
        item
    }

    /// 查看队列头部元素（不移除）
    pub fn peek(&self) -> Option<&T> {
        self.inner.front()
    }

    /// 获取当前队列长度
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// 检查队列是否已满
    pub fn is_full(&self) -> bool {
        self.inner.len() >= self.capacity
    }

    /// 获取队列容量
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 获取当前溢出策略
    pub fn policy(&self) -> QueueOverflowPolicy {
        self.policy
    }

    /// 获取队列统计信息
    pub fn stats(&self) -> &QueueStats {
        &self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = QueueStats::default();
    }

    /// 清空队列
    ///
    /// 移除所有元素，但保留容量和策略设置。
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// 调整队列容量
    ///
    /// 如果新容量小于当前元素数量，根据溢出策略处理多余元素。
    ///
    /// # 参数
    ///
    /// * `new_capacity` - 新的队列容量
    ///
    /// # Panics
    ///
    /// 如果 `new_capacity == 0` 则 panic。
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 内存压力时减小队列
    /// queue.adjust_capacity(500);
    /// ```
    pub fn adjust_capacity(&mut self, new_capacity: usize) {
        assert!(new_capacity > 0, "Queue capacity must be greater than 0");

        if new_capacity < self.inner.len() {
            // 根据策略处理多余的消息
            while self.inner.len() > new_capacity {
                match self.policy {
                    QueueOverflowPolicy::DropOldest => {
                        self.inner.pop_front();
                    }
                    QueueOverflowPolicy::DropNewest => {
                        self.inner.pop_back();
                    }
                    QueueOverflowPolicy::Block { .. } => {
                        // Block 策略下，从尾部移除
                        self.inner.pop_back();
                    }
                }
                self.stats.dropped += 1;
            }
        }

        self.capacity = new_capacity;
    }
}
```

## 错误类型

```rust
use std::time::Duration;

/// 队列相关错误
#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    /// 队列已满（Block 策略超时）
    #[error("Queue full, timeout after {timeout:?}")]
    QueueFull {
        /// 等待的超时时间
        timeout: Duration,
    },

    /// 无效的容量
    #[error("Invalid capacity: {0}")]
    InvalidCapacity(usize),
}
```

## 异步版本

```rust
use tokio::sync::Mutex;
use std::sync::Arc;

/// 异步有界队列
///
/// 使用 tokio 的异步原语实现的线程安全队列。
///
/// # 示例
///
/// ```rust
/// use canlink_hal::queue::AsyncBoundedQueue;
///
/// let queue = AsyncBoundedQueue::new(1000);
///
/// // 异步入队
/// queue.push(message).await?;
///
/// // 异步出队
/// if let Some(msg) = queue.pop().await {
///     // 处理消息
/// }
/// ```
#[cfg(feature = "async")]
pub struct AsyncBoundedQueue<T> {
    inner: Arc<Mutex<BoundedQueue<T>>>,
}

#[cfg(feature = "async")]
impl<T: Send> AsyncBoundedQueue<T> {
    /// 创建新的异步有界队列
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BoundedQueue::new(capacity))),
        }
    }

    /// 创建使用指定策略的异步有界队列
    pub fn with_policy(capacity: usize, policy: QueueOverflowPolicy) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BoundedQueue::with_policy(capacity, policy))),
        }
    }

    /// 异步入队
    pub async fn push(&self, item: T) -> Result<(), QueueError> {
        let mut guard = self.inner.lock().await;
        guard.push(item)
    }

    /// 异步出队
    pub async fn pop(&self) -> Option<T> {
        let mut guard = self.inner.lock().await;
        guard.pop()
    }

    /// 获取当前队列长度
    pub async fn len(&self) -> usize {
        let guard = self.inner.lock().await;
        guard.len()
    }

    /// 获取队列统计信息
    pub async fn stats(&self) -> QueueStats {
        let guard = self.inner.lock().await;
        guard.stats().clone()
    }
}
```

## 配置文件支持

```rust
use serde::Deserialize;

/// 队列配置（从 TOML 加载）
#[derive(Debug, Clone, Deserialize)]
pub struct QueueConfig {
    /// 队列容量（消息数量）
    #[serde(default = "default_capacity")]
    pub capacity: usize,

    /// 溢出策略
    #[serde(default)]
    pub overflow_policy: QueueOverflowPolicyConfig,
}

/// 溢出策略配置
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

impl From<QueueOverflowPolicyConfig> for QueueOverflowPolicy {
    fn from(config: QueueOverflowPolicyConfig) -> Self {
        match config {
            QueueOverflowPolicyConfig::DropOldest => Self::DropOldest,
            QueueOverflowPolicyConfig::DropNewest => Self::DropNewest,
            QueueOverflowPolicyConfig::Block { timeout_ms } => Self::Block {
                timeout: Duration::from_millis(timeout_ms),
            },
        }
    }
}
```

**配置示例**:

```toml
[queue]
capacity = 2000

[queue.overflow_policy]
type = "drop_oldest"
```

```toml
[queue]
capacity = 5000

[queue.overflow_policy]
type = "block"
timeout_ms = 500
```

## 测试要求

### 单元测试

1. **基本操作测试**
   - push/pop 操作
   - peek 操作
   - len/is_empty/is_full 检查

2. **溢出策略测试**
   - DropOldest: 验证最旧消息被丢弃
   - DropNewest: 验证新消息被丢弃
   - Block: 验证超时返回错误

3. **容量调整测试**
   - 增大容量
   - 减小容量（触发溢出策略）

4. **统计信息测试**
   - enqueued 计数
   - dequeued 计数
   - dropped 计数
   - timeouts 计数

### 性能测试

```rust
#[bench]
fn bench_queue_push_pop(b: &mut Bencher) {
    let mut queue = BoundedQueue::new(1000);
    let message = CanMessage::new_standard(0x123, &[1, 2, 3, 4]);

    b.iter(|| {
        queue.push(message.clone()).unwrap();
        queue.pop()
    });
}

// 目标: O(1) 操作，< 1 μs
```

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-01-10 | 初始版本 |
