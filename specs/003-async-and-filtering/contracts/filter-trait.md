# API 契约: MessageFilter Trait

**功能分支**: `003-async-and-filtering`
**创建日期**: 2026-01-10
**状态**: 草稿

---

## 概述

`MessageFilter` trait 定义了消息过滤的统一接口。所有过滤器（硬件和软件）都必须实现此 trait。

## Trait 定义

```rust
/// 消息过滤器 trait
///
/// 所有过滤器都必须实现此 trait。过滤器用于筛选 CAN 消息，
/// 只有通过过滤器的消息才会被传递给应用层。
///
/// # 线程安全
///
/// 过滤器必须是线程安全的（`Send + Sync`），因为它们可能在多线程环境中使用。
///
/// # 性能要求
///
/// - 软件过滤器的 `matches` 方法应在 10 μs 内完成
/// - `matches` 方法应该是无副作用的纯函数
///
/// # 示例
///
/// ```rust
/// use canlink_hal::filter::MessageFilter;
/// use canlink_hal::message::CanMessage;
///
/// struct MyFilter {
///     target_id: u32,
/// }
///
/// impl MessageFilter for MyFilter {
///     fn matches(&self, message: &CanMessage) -> bool {
///         message.id().raw() == self.target_id
///     }
/// }
/// ```
pub trait MessageFilter: Send + Sync {
    /// 检查消息是否通过过滤器
    ///
    /// # 参数
    ///
    /// * `message` - 要检查的 CAN 消息
    ///
    /// # 返回值
    ///
    /// - `true`: 消息通过过滤器，应该被处理
    /// - `false`: 消息被过滤，应该被丢弃
    ///
    /// # 性能
    ///
    /// 此方法应该尽可能快速执行，目标是 < 10 μs。
    /// 避免在此方法中进行内存分配或 I/O 操作。
    fn matches(&self, message: &CanMessage) -> bool;

    /// 过滤器优先级（用于排序）
    ///
    /// 数值越小优先级越高。在 FilterChain 中，
    /// 过滤器按优先级从小到大排序执行。
    ///
    /// # 默认值
    ///
    /// 默认返回 0（最高优先级）。
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 高优先级过滤器
    /// fn priority(&self) -> u32 { 0 }
    ///
    /// // 低优先级过滤器
    /// fn priority(&self) -> u32 { 100 }
    /// ```
    fn priority(&self) -> u32 {
        0
    }

    /// 是否为硬件过滤器
    ///
    /// 硬件过滤器由 CAN 控制器硬件执行，性能更高。
    /// 当硬件过滤器数量超过硬件限制时，FilterChain 会
    /// 自动将多余的过滤器回退到软件过滤。
    ///
    /// # 默认值
    ///
    /// 默认返回 `false`（软件过滤器）。
    ///
    /// # 注意
    ///
    /// 只有硬件后端支持的过滤器类型才应返回 `true`。
    /// 如果硬件不支持，即使返回 `true` 也会被回退到软件过滤。
    fn is_hardware(&self) -> bool {
        false
    }
}
```

## 内置过滤器实现

### IdFilter

```rust
/// ID 过滤器
///
/// 基于 CAN ID 的过滤器，支持精确匹配和掩码匹配。
///
/// # 匹配规则
///
/// 消息通过过滤器当且仅当：
/// 1. 消息的帧类型（标准/扩展）与过滤器匹配
/// 2. `(message.id & mask) == (filter.id & mask)`
///
/// # 示例
///
/// ```rust
/// use canlink_hal::filter::IdFilter;
///
/// // 精确匹配 ID 0x123
/// let filter = IdFilter::new(0x123);
///
/// // 掩码匹配：匹配 0x120-0x12F
/// let filter = IdFilter::with_mask(0x120, 0x7F0);
///
/// // 扩展帧过滤
/// let filter = IdFilter::new_extended(0x12345678);
/// ```
#[derive(Debug, Clone)]
pub struct IdFilter {
    id: u32,
    mask: u32,
    extended: bool,
}

impl IdFilter {
    /// 创建精确匹配的标准帧过滤器
    ///
    /// # 参数
    ///
    /// * `id` - 要匹配的 CAN ID (0x000-0x7FF)
    ///
    /// # Panics
    ///
    /// 如果 `id > 0x7FF` 则 panic。
    pub fn new(id: u32) -> Self;

    /// 创建带掩码的标准帧过滤器
    ///
    /// # 参数
    ///
    /// * `id` - 过滤器 ID
    /// * `mask` - 掩码，1 表示该位需要匹配
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 匹配 0x100-0x1FF（高 3 位为 001）
    /// let filter = IdFilter::with_mask(0x100, 0x700);
    /// ```
    pub fn with_mask(id: u32, mask: u32) -> Self;

    /// 创建精确匹配的扩展帧过滤器
    ///
    /// # 参数
    ///
    /// * `id` - 要匹配的扩展 CAN ID (0x00000000-0x1FFFFFFF)
    pub fn new_extended(id: u32) -> Self;

    /// 创建带掩码的扩展帧过滤器
    pub fn with_mask_extended(id: u32, mask: u32) -> Self;
}

impl MessageFilter for IdFilter {
    fn matches(&self, message: &CanMessage) -> bool {
        let msg_id = message.id().raw();
        let msg_extended = message.id().is_extended();

        if self.extended != msg_extended {
            return false;
        }

        (msg_id & self.mask) == (self.id & self.mask)
    }

    fn is_hardware(&self) -> bool {
        true  // ID 过滤器通常由硬件支持
    }
}
```

### RangeFilter

```rust
/// 范围过滤器
///
/// 匹配指定 ID 范围内的所有消息。
///
/// # 示例
///
/// ```rust
/// use canlink_hal::filter::RangeFilter;
///
/// // 匹配 ID 0x200 到 0x2FF
/// let filter = RangeFilter::new(0x200, 0x2FF);
/// ```
#[derive(Debug, Clone)]
pub struct RangeFilter {
    start_id: u32,
    end_id: u32,
    extended: bool,
}

impl RangeFilter {
    /// 创建标准帧范围过滤器
    ///
    /// # 参数
    ///
    /// * `start_id` - 起始 ID（包含）
    /// * `end_id` - 结束 ID（包含）
    ///
    /// # Panics
    ///
    /// 如果 `start_id > end_id` 或 ID 超出范围则 panic。
    pub fn new(start_id: u32, end_id: u32) -> Self;

    /// 创建扩展帧范围过滤器
    pub fn new_extended(start_id: u32, end_id: u32) -> Self;
}

impl MessageFilter for RangeFilter {
    fn matches(&self, message: &CanMessage) -> bool {
        let msg_id = message.id().raw();
        let msg_extended = message.id().is_extended();

        if self.extended != msg_extended {
            return false;
        }

        msg_id >= self.start_id && msg_id <= self.end_id
    }

    // 范围过滤器通常不被硬件支持
    fn is_hardware(&self) -> bool {
        false
    }
}
```

## FilterChain API

```rust
/// 过滤器链
///
/// 管理多个过滤器，支持硬件过滤器自动回退。
///
/// # 过滤逻辑
///
/// 消息通过过滤器链当且仅当：
/// - 过滤器链为空（无过滤器时全部通过）
/// - 或任一过滤器返回 `true`（OR 逻辑）
///
/// # 硬件过滤器回退
///
/// 当添加的硬件过滤器数量超过 `max_hardware_filters` 时，
/// 多余的过滤器会自动回退到软件过滤器列表。
///
/// # 示例
///
/// ```rust
/// use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter};
///
/// let mut chain = FilterChain::new(4);  // 最多 4 个硬件过滤器
///
/// // 添加过滤器
/// chain.add_filter(Box::new(IdFilter::new(0x123)));
/// chain.add_filter(Box::new(RangeFilter::new(0x200, 0x2FF)));
///
/// // 检查消息
/// if chain.matches(&message) {
///     // 消息通过过滤
/// }
/// ```
pub struct FilterChain {
    hardware_filters: Vec<Box<dyn MessageFilter>>,
    software_filters: Vec<Box<dyn MessageFilter>>,
    max_hardware_filters: usize,
}

impl FilterChain {
    /// 创建新的过滤器链
    ///
    /// # 参数
    ///
    /// * `max_hardware_filters` - 硬件支持的最大过滤器数量
    pub fn new(max_hardware_filters: usize) -> Self;

    /// 添加过滤器
    ///
    /// 如果过滤器是硬件过滤器且未超过限制，添加到硬件列表；
    /// 否则添加到软件列表。
    ///
    /// # 参数
    ///
    /// * `filter` - 要添加的过滤器
    pub fn add_filter(&mut self, filter: Box<dyn MessageFilter>);

    /// 移除所有过滤器
    pub fn clear(&mut self);

    /// 检查消息是否通过过滤器链
    ///
    /// # 返回值
    ///
    /// - 如果过滤器链为空，返回 `true`
    /// - 如果任一过滤器匹配，返回 `true`
    /// - 否则返回 `false`
    pub fn matches(&self, message: &CanMessage) -> bool;

    /// 获取硬件过滤器数量
    pub fn hardware_filter_count(&self) -> usize;

    /// 获取软件过滤器数量
    pub fn software_filter_count(&self) -> usize;

    /// 获取总过滤器数量
    pub fn total_filter_count(&self) -> usize;
}
```

## 错误类型

```rust
/// 过滤器相关错误
#[derive(Debug, thiserror::Error)]
pub enum FilterError {
    /// 无效的 ID 值
    #[error("Invalid CAN ID: {id}, must be <= {max}")]
    InvalidId { id: u32, max: u32 },

    /// 无效的 ID 范围
    #[error("Invalid ID range: start ({start}) > end ({end})")]
    InvalidRange { start: u32, end: u32 },

    /// 无效的掩码
    #[error("Invalid mask: {mask}")]
    InvalidMask { mask: u32 },
}
```

## 测试要求

### 单元测试

1. **IdFilter 测试**
   - 精确匹配测试
   - 掩码匹配测试
   - 标准帧/扩展帧区分测试
   - 边界值测试（ID = 0, ID = MAX）

2. **RangeFilter 测试**
   - 范围内匹配测试
   - 范围边界测试
   - 范围外不匹配测试

3. **FilterChain 测试**
   - 空链测试（全部通过）
   - 单过滤器测试
   - 多过滤器 OR 逻辑测试
   - 硬件过滤器回退测试
   - 优先级排序测试

### 性能测试

```rust
#[bench]
fn bench_id_filter_matches(b: &mut Bencher) {
    let filter = IdFilter::new(0x123);
    let message = CanMessage::new_standard(0x123, &[1, 2, 3, 4]);

    b.iter(|| {
        filter.matches(&message)
    });
}

// 目标: < 10 μs/消息
```

## 线程安全说明

- `MessageFilter` trait 要求 `Send + Sync`
- `FilterChain` 本身不是线程安全的，需要外部同步
- 推荐使用 `Arc<RwLock<FilterChain>>` 在多线程环境中共享

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-01-10 | 初始版本 |
