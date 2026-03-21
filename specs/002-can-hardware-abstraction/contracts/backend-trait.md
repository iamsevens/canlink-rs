# API 契约: CanBackend Trait

**版本**: 1.0.0
**日期**: 2026-01-08
**目的**: 定义所有硬件后端必须实现的统一接口（FR-001）

## 概述

`CanBackend` trait 是硬件抽象层的核心接口，所有硬件后端（TSMaster、PEAK、Kvaser、Mock 等）必须实现此 trait。

## Trait 定义

```rust
/// CAN 硬件后端接口
///
/// # 线程安全
///
/// 此 trait 的方法要求外部同步。如果需要从多个线程访问同一个后端实例，
/// 调用者必须使用 `Mutex` 或 `RwLock` 提供同步保护。
///
/// # 生命周期
///
/// 后端实例的生命周期：
/// 1. 创建（通过 `BackendFactory::create()`）
/// 2. 初始化（`initialize()`）
/// 3. 运行（调用 `send_message()`, `receive_message()` 等）
/// 4. 关闭（`close()`）
pub trait CanBackend: Send {
    // ========== 生命周期管理 ==========

    /// 初始化后端
    ///
    /// # 参数
    /// - `config`: 后端配置参数
    ///
    /// # 返回
    /// - `Ok(())`: 初始化成功
    /// - `Err(CanError)`: 初始化失败
    ///
    /// # 错误
    /// - `CanError::InitializationFailed`: 硬件初始化失败
    /// - `CanError::ConfigError`: 配置参数无效
    ///
    /// # 前置条件
    /// - 后端处于 `Uninitialized` 状态
    ///
    /// # 后置条件
    /// - 成功：后端处于 `Running` 状态
    /// - 失败：后端保持 `Uninitialized` 状态
    ///
    /// # 示例
    /// ```rust
    /// let mut backend = create_backend();
    /// backend.initialize(&config)?;
    /// ```
    fn initialize(&mut self, config: &BackendConfig) -> Result<(), CanError>;

    /// 关闭后端，释放资源
    ///
    /// # 资源清理
    ///
    /// 此方法必须释放以下资源：
    /// - 关闭所有打开的 CAN 通道
    /// - 清空消息发送和接收队列
    /// - 释放硬件连接和驱动资源
    /// - 释放内存缓冲区
    ///
    /// # 未发送消息处理
    ///
    /// 调用 close 时，所有未发送的消息将被丢弃。
    /// 不保证消息传递完成。如需确保消息发送，
    /// 应在调用 close 前等待发送完成。
    ///
    /// # 幂等性
    ///
    /// 此方法是幂等的，可以安全地多次调用。
    /// 重复调用不会产生错误，也不会有副作用。
    ///
    /// # 返回
    /// - `Ok(())`: 关闭成功
    /// - `Err(CanError)`: 关闭失败（但资源仍会尽力释放）
    ///
    /// # 前置条件
    /// - 无（可以在任何状态下调用）
    ///
    /// # 后置条件
    /// - 后端处于 `Closed` 状态
    /// - 所有资源已释放
    /// - 后续调用 close 不会有任何效果
    ///
    /// # 示例
    /// ```rust
    /// backend.close()?;
    /// // 可以安全地再次调用
    /// backend.close()?; // 不会产生错误
    /// ```
    fn close(&mut self) -> Result<(), CanError>;

    // ========== 硬件能力查询 ==========

    /// 查询硬件能力
    ///
    /// # 返回
    /// - `Ok(HardwareCapability)`: 硬件能力描述
    /// - `Err(CanError)`: 查询失败
    ///
    /// # 性能要求
    /// - 响应时间 < 1ms（SC-004）
    ///
    /// # 示例
    /// ```rust
    /// let capability = backend.get_capability()?;
    /// if capability.supports_canfd {
    ///     println!("CAN-FD is supported");
    /// }
    /// ```
    fn get_capability(&self) -> Result<HardwareCapability, CanError>;

    // ========== 消息收发 ==========

    /// 发送 CAN 消息
    ///
    /// # 参数
    /// - `message`: 要发送的消息
    ///
    /// # 返回
    /// - `Ok(())`: 消息已成功发送到总线
    /// - `Err(CanError)`: 发送失败
    ///
    /// # 错误
    /// - `CanError::SendFailed`: 发送失败（如总线 Bus-Off）
    /// - `CanError::UnsupportedFeature`: 硬件不支持该消息类型（如 CAN-FD）
    /// - `CanError::InvalidDataLength`: 数据长度超出限制
    ///
    /// # 前置条件
    /// - 后端处于 `Running` 状态
    /// - 消息格式有效
    ///
    /// # 后置条件
    /// - 成功：消息已发送到总线
    /// - 失败：消息未发送，后端状态不变
    ///
    /// # 性能要求
    /// - 支持 1000 消息/秒吞吐量
    /// - 抽象层开销 < 5%
    ///
    /// # 示例
    /// ```rust
    /// let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03])?;
    /// backend.send_message(&msg)?;
    /// ```
    fn send_message(&mut self, message: &CanMessage) -> Result<(), CanError>;

    /// 接收 CAN 消息（非阻塞）
    ///
    /// # 返回
    /// - `Ok(Some(message))`: 接收到消息
    /// - `Ok(None)`: 当前无消息可接收
    /// - `Err(CanError)`: 接收失败
    ///
    /// # 错误
    /// - `CanError::ReceiveFailed`: 接收失败
    ///
    /// # 前置条件
    /// - 后端处于 `Running` 状态
    ///
    /// # 后置条件
    /// - 成功：返回的消息已从接收队列中移除
    /// - 失败：接收队列状态不变
    ///
    /// # 示例
    /// ```rust
    /// if let Some(msg) = backend.receive_message()? {
    ///     println!("Received: {:?}", msg);
    /// }
    /// ```
    fn receive_message(&mut self) -> Result<Option<CanMessage>, CanError>;

    // ========== 通道管理 ==========

    /// 打开指定的 CAN 通道
    ///
    /// # 参数
    /// - `channel`: 通道索引（从 0 开始）
    ///
    /// # 返回
    /// - `Ok(())`: 通道已打开
    /// - `Err(CanError)`: 打开失败
    ///
    /// # 错误
    /// - `CanError::ChannelNotFound`: 通道不存在
    ///
    /// # 前置条件
    /// - 后端处于 `Running` 状态
    /// - 通道索引有效（< `capability.channel_count`）
    ///
    /// # 示例
    /// ```rust
    /// backend.open_channel(0)?;
    /// ```
    fn open_channel(&mut self, channel: u8) -> Result<(), CanError>;

    /// 关闭指定的 CAN 通道
    ///
    /// # 参数
    /// - `channel`: 通道索引
    ///
    /// # 返回
    /// - `Ok(())`: 通道已关闭
    /// - `Err(CanError)`: 关闭失败
    ///
    /// # 示例
    /// ```rust
    /// backend.close_channel(0)?;
    /// ```
    fn close_channel(&mut self, channel: u8) -> Result<(), CanError>;

    // ========== 版本信息 ==========

    /// 获取后端版本
    ///
    /// # 返回
    /// 后端的语义版本号
    ///
    /// # 示例
    /// ```rust
    /// let version = backend.version();
    /// println!("Backend version: {}", version.version);
    /// ```
    fn version(&self) -> BackendVersion;

    /// 获取后端名称
    ///
    /// # 返回
    /// 后端的唯一标识名称（如 "tsmaster", "mock"）
    ///
    /// # 示例
    /// ```rust
    /// let name = backend.name();
    /// println!("Using backend: {}", name);
    /// ```
    fn name(&self) -> &str;
}
```

## 可选异步扩展

```rust
#[cfg(feature = "async")]
use async_trait::async_trait;

/// CAN 硬件后端异步接口（可选）
///
/// 启用 `async` feature 后可用。
#[cfg(feature = "async")]
#[async_trait]
pub trait CanBackendAsync: CanBackend {
    /// 异步发送 CAN 消息
    async fn send_message_async(&mut self, message: &CanMessage) -> Result<(), CanError>;

    /// 异步接收 CAN 消息（阻塞直到有消息或超时）
    ///
    /// # 参数
    /// - `timeout`: 超时时间（None 表示无限等待）
    async fn receive_message_async(
        &mut self,
        timeout: Option<Duration>,
    ) -> Result<CanMessage, CanError>;
}
```

## 实现要求

### 必须实现的方法
所有方法都必须实现，不允许使用 `unimplemented!()` 或 `todo!()`。

### 错误处理
- 所有错误必须使用 `CanError` 类型
- 错误消息必须清晰描述失败原因
- 必须包含足够的上下文信息用于调试

### 性能要求
- `send_message()` 和 `receive_message()` 是性能关键路径
- 抽象层开销必须 < 5%（SC-005）
- `get_capability()` 响应时间 < 1ms（SC-004）

### 线程安全
- 后端实例必须实现 `Send` trait
- 方法使用 `&mut self`，要求外部同步
- 不允许在方法内部使用锁（性能考虑）

### 资源管理
- `close()` 必须释放所有资源，即使发生错误
- 必须释放的资源包括：
  - 所有打开的 CAN 通道
  - 消息发送和接收队列
  - 硬件连接和驱动资源
  - 内存缓冲区
- 未发送的消息在 close 时被丢弃，不保证传递
- `close()` 方法必须是幂等的，可以安全地多次调用
- 实现 `Drop` trait 以确保资源清理（调用 close）

## 测试要求

### 单元测试
每个后端实现必须提供以下测试：
- 初始化和关闭
- 消息发送和接收
- 能力查询
- 错误处理

### 集成测试
- 与 `BackendRegistry` 的集成
- 配置加载和解析
- 版本兼容性检查

### 性能测试
- 1000 消息/秒吞吐量测试
- 抽象层开销测量

## 示例实现

参见 `canlink-mock` crate 中的 `MockBackend` 实现。

## 版本兼容性

- 主版本号变更：破坏性 API 变更
- 次版本号变更：向后兼容的功能添加
- 补丁版本号变更：向后兼容的错误修复

后端版本与抽象层版本的主版本号必须相同才能加载。
