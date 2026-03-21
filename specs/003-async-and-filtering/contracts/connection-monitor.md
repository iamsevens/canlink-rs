# API 契约: ConnectionMonitor

**功能分支**: `003-async-and-filtering`
**创建日期**: 2026-01-10
**状态**: 草稿

---

## 概述

`ConnectionMonitor` 提供后端连接状态监控功能，支持心跳检测和可选的自动重连。

## ConnectionState 枚举

```rust
/// 连接状态
///
/// 表示后端连接的当前状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// 已连接
    ///
    /// 后端正常工作，可以发送和接收消息。
    Connected,

    /// 已断开
    ///
    /// 后端连接已断开，需要重新初始化。
    Disconnected,

    /// 重连中
    ///
    /// 正在尝试重新连接（仅当启用自动重连时）。
    Reconnecting,
}

impl ConnectionState {
    /// 检查是否可以发送消息
    pub fn can_send(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// 检查是否可以接收消息
    pub fn can_receive(&self) -> bool {
        matches!(self, Self::Connected)
    }
}
```

## ReconnectConfig 结构体

```rust
use std::time::Duration;

/// 重连配置
///
/// 配置自动重连的行为。默认情况下自动重连是禁用的。
///
/// # 示例
///
/// ```rust
/// use canlink_hal::monitor::ReconnectConfig;
/// use std::time::Duration;
///
/// let config = ReconnectConfig {
///     max_retries: 5,
///     retry_interval: Duration::from_secs(2),
///     backoff_multiplier: 1.5,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// 最大重试次数
    ///
    /// 达到此次数后停止重连，状态变为 `Disconnected`。
    /// 设置为 0 表示无限重试。
    pub max_retries: u32,

    /// 初始重试间隔
    ///
    /// 第一次重连尝试前的等待时间。
    pub retry_interval: Duration,

    /// 退避乘数
    ///
    /// 每次重连失败后，等待时间乘以此值。
    /// 设置为 1.0 表示固定间隔。
    ///
    /// # 示例
    ///
    /// - `backoff_multiplier = 2.0`
    /// - 第 1 次重试: 等待 1 秒
    /// - 第 2 次重试: 等待 2 秒
    /// - 第 3 次重试: 等待 4 秒
    pub backoff_multiplier: f32,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_interval: Duration::from_secs(1),
            backoff_multiplier: 2.0,
        }
    }
}
```

## ConnectionMonitor 结构体

```rust
use std::sync::Arc;
use tokio::sync::Mutex;

/// 连接监控器
///
/// 监控后端连接状态，支持心跳检测和可选的自动重连。
///
/// # 功能
///
/// - **心跳检测**: 定期检查后端连接状态
/// - **状态通知**: 连接状态变化时触发回调
/// - **自动重连**: 可选功能，断开后自动尝试重连
///
/// # 线程安全
///
/// `ConnectionMonitor` 是线程安全的，可以在多线程环境中使用。
///
/// # 示例
///
/// ```rust
/// use canlink_hal::monitor::{ConnectionMonitor, ConnectionState};
/// use std::time::Duration;
///
/// // 创建监控器（不启用自动重连）
/// let monitor = ConnectionMonitor::new(backend, Duration::from_secs(1));
///
/// // 启动监控
/// monitor.start();
///
/// // 注册状态变化回调
/// monitor.on_state_change(|old, new| {
///     println!("State changed: {:?} -> {:?}", old, new);
/// });
///
/// // 获取当前状态
/// let state = monitor.state();
/// ```
pub struct ConnectionMonitor {
    /// 后端引用
    backend: Arc<Mutex<dyn CanBackend + Send>>,
    /// 心跳检测间隔
    heartbeat_interval: Duration,
    /// 重连配置（None 表示禁用自动重连）
    reconnect_config: Option<ReconnectConfig>,
    /// 当前连接状态
    state: Arc<AtomicConnectionState>,
    /// 状态变化回调
    callbacks: Arc<Mutex<Vec<Box<dyn Fn(ConnectionState, ConnectionState) + Send + Sync>>>>,
    /// 监控任务句柄
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConnectionMonitor {
    /// 创建新的连接监控器
    ///
    /// 默认不启用自动重连。
    ///
    /// # 参数
    ///
    /// * `backend` - 要监控的后端
    /// * `heartbeat_interval` - 心跳检测间隔
    ///
    /// # 示例
    ///
    /// ```rust
    /// let monitor = ConnectionMonitor::new(
    ///     backend,
    ///     Duration::from_secs(1),
    /// );
    /// ```
    pub fn new(
        backend: Arc<Mutex<dyn CanBackend + Send>>,
        heartbeat_interval: Duration,
    ) -> Self {
        Self {
            backend,
            heartbeat_interval,
            reconnect_config: None,
            state: Arc::new(AtomicConnectionState::new(ConnectionState::Connected)),
            callbacks: Arc::new(Mutex::new(Vec::new())),
            task_handle: None,
        }
    }

    /// 创建启用自动重连的连接监控器
    ///
    /// # 参数
    ///
    /// * `backend` - 要监控的后端
    /// * `heartbeat_interval` - 心跳检测间隔
    /// * `reconnect_config` - 重连配置
    ///
    /// # 示例
    ///
    /// ```rust
    /// let monitor = ConnectionMonitor::with_reconnect(
    ///     backend,
    ///     Duration::from_secs(1),
    ///     ReconnectConfig::default(),
    /// );
    /// ```
    pub fn with_reconnect(
        backend: Arc<Mutex<dyn CanBackend + Send>>,
        heartbeat_interval: Duration,
        reconnect_config: ReconnectConfig,
    ) -> Self {
        Self {
            backend,
            heartbeat_interval,
            reconnect_config: Some(reconnect_config),
            state: Arc::new(AtomicConnectionState::new(ConnectionState::Connected)),
            callbacks: Arc::new(Mutex::new(Vec::new())),
            task_handle: None,
        }
    }

    /// 启动监控
    ///
    /// 开始心跳检测任务。如果已经启动，此方法无效。
    ///
    /// # 注意
    ///
    /// 此方法需要在 tokio 运行时中调用。
    pub async fn start(&mut self) {
        if self.task_handle.is_some() {
            return; // 已经启动
        }

        let backend = self.backend.clone();
        let interval = self.heartbeat_interval;
        let state = self.state.clone();
        let callbacks = self.callbacks.clone();
        let reconnect_config = self.reconnect_config.clone();

        let handle = tokio::spawn(async move {
            Self::monitor_loop(backend, interval, state, callbacks, reconnect_config).await;
        });

        self.task_handle = Some(handle);
    }

    /// 停止监控
    ///
    /// 停止心跳检测任务。
    pub async fn stop(&mut self) {
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }
    }

    /// 获取当前连接状态
    pub fn state(&self) -> ConnectionState {
        self.state.load()
    }

    /// 注册状态变化回调
    ///
    /// 当连接状态变化时，回调函数会被调用。
    ///
    /// # 参数
    ///
    /// * `callback` - 回调函数，参数为 (旧状态, 新状态)
    ///
    /// # 示例
    ///
    /// ```rust
    /// monitor.on_state_change(|old, new| {
    ///     if new == ConnectionState::Disconnected {
    ///         eprintln!("Connection lost!");
    ///     }
    /// });
    /// ```
    pub async fn on_state_change<F>(&self, callback: F)
    where
        F: Fn(ConnectionState, ConnectionState) + Send + Sync + 'static,
    {
        let mut callbacks = self.callbacks.lock().await;
        callbacks.push(Box::new(callback));
    }

    /// 手动触发重连
    ///
    /// 即使未启用自动重连，也可以手动触发重连尝试。
    ///
    /// # 返回值
    ///
    /// - `Ok(())` - 重连成功
    /// - `Err(MonitorError)` - 重连失败
    pub async fn reconnect(&self) -> Result<(), MonitorError> {
        let mut backend = self.backend.lock().await;

        // 先关闭
        let _ = backend.close();

        // 重新初始化
        backend.initialize().map_err(|e| MonitorError::ReconnectFailed {
            reason: e.to_string(),
        })?;

        self.set_state(ConnectionState::Connected).await;
        Ok(())
    }

    /// 内部：监控循环
    async fn monitor_loop(
        backend: Arc<Mutex<dyn CanBackend + Send>>,
        interval: Duration,
        state: Arc<AtomicConnectionState>,
        callbacks: Arc<Mutex<Vec<Box<dyn Fn(ConnectionState, ConnectionState) + Send + Sync>>>>,
        reconnect_config: Option<ReconnectConfig>,
    ) {
        let mut interval_timer = tokio::time::interval(interval);

        loop {
            interval_timer.tick().await;

            // 心跳检测
            let is_connected = {
                let backend = backend.lock().await;
                backend.state() == BackendState::Running
            };

            let current_state = state.load();

            if is_connected {
                if current_state != ConnectionState::Connected {
                    Self::set_state_internal(&state, &callbacks, ConnectionState::Connected).await;
                }
            } else {
                if current_state == ConnectionState::Connected {
                    Self::set_state_internal(&state, &callbacks, ConnectionState::Disconnected).await;

                    // 如果启用了自动重连
                    if let Some(ref config) = reconnect_config {
                        Self::attempt_reconnect(&backend, &state, &callbacks, config).await;
                    }
                }
            }
        }
    }

    /// 内部：尝试重连
    async fn attempt_reconnect(
        backend: &Arc<Mutex<dyn CanBackend + Send>>,
        state: &Arc<AtomicConnectionState>,
        callbacks: &Arc<Mutex<Vec<Box<dyn Fn(ConnectionState, ConnectionState) + Send + Sync>>>>,
        config: &ReconnectConfig,
    ) {
        Self::set_state_internal(state, callbacks, ConnectionState::Reconnecting).await;

        let mut retry_count = 0;
        let mut current_interval = config.retry_interval;

        loop {
            if config.max_retries > 0 && retry_count >= config.max_retries {
                // 达到最大重试次数
                Self::set_state_internal(state, callbacks, ConnectionState::Disconnected).await;
                break;
            }

            tokio::time::sleep(current_interval).await;

            // 尝试重连
            let result = {
                let mut backend = backend.lock().await;
                let _ = backend.close();
                backend.initialize()
            };

            if result.is_ok() {
                Self::set_state_internal(state, callbacks, ConnectionState::Connected).await;
                break;
            }

            retry_count += 1;
            current_interval = Duration::from_secs_f32(
                current_interval.as_secs_f32() * config.backoff_multiplier
            );
        }
    }

    /// 内部：设置状态并触发回调
    async fn set_state_internal(
        state: &Arc<AtomicConnectionState>,
        callbacks: &Arc<Mutex<Vec<Box<dyn Fn(ConnectionState, ConnectionState) + Send + Sync>>>>,
        new_state: ConnectionState,
    ) {
        let old_state = state.swap(new_state);
        if old_state != new_state {
            let callbacks = callbacks.lock().await;
            for callback in callbacks.iter() {
                callback(old_state, new_state);
            }
        }
    }

    async fn set_state(&self, new_state: ConnectionState) {
        Self::set_state_internal(&self.state, &self.callbacks, new_state).await;
    }
}
```

## 错误类型

```rust
/// 监控相关错误
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    /// 重连失败
    #[error("Reconnect failed: {reason}")]
    ReconnectFailed { reason: String },

    /// 监控未启动
    #[error("Monitor not started")]
    NotStarted,

    /// 后端错误
    #[error("Backend error: {0}")]
    BackendError(#[from] CanError),
}
```

## 事件类型

```rust
/// 连接事件
///
/// 用于更细粒度的事件通知。
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    /// 连接建立
    Connected,

    /// 连接断开
    Disconnected {
        /// 断开原因
        reason: Option<String>,
    },

    /// 开始重连
    ReconnectStarted {
        /// 当前重试次数
        attempt: u32,
    },

    /// 重连成功
    ReconnectSucceeded {
        /// 总重试次数
        attempts: u32,
    },

    /// 重连失败
    ReconnectFailed {
        /// 总重试次数
        attempts: u32,
        /// 失败原因
        reason: String,
    },

    /// 心跳超时
    HeartbeatTimeout,
}
```

## 配置文件支持

```rust
use serde::Deserialize;

/// 监控配置（从 TOML 加载）
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

impl From<MonitorConfig> for (Duration, Option<ReconnectConfig>) {
    fn from(config: MonitorConfig) -> Self {
        let heartbeat = Duration::from_millis(config.heartbeat_interval_ms);
        let reconnect = config.reconnect.map(|r| ReconnectConfig {
            max_retries: r.max_retries,
            retry_interval: Duration::from_millis(r.retry_interval_ms),
            backoff_multiplier: r.backoff_multiplier,
        });
        (heartbeat, reconnect)
    }
}
```

**配置示例**:

```toml
# 仅心跳检测，不自动重连
[monitor]
heartbeat_interval_ms = 1000
```

```toml
# 启用自动重连
[monitor]
heartbeat_interval_ms = 500

[monitor.reconnect]
max_retries = 5
retry_interval_ms = 2000
backoff_multiplier = 1.5
```

## 使用示例

### 基本使用（不自动重连）

```rust
use canlink_hal::monitor::{ConnectionMonitor, ConnectionState};
use std::time::Duration;

#[tokio::main]
async fn main() {
    let backend = create_backend();
    let mut monitor = ConnectionMonitor::new(
        Arc::new(Mutex::new(backend)),
        Duration::from_secs(1),
    );

    // 注册回调
    monitor.on_state_change(|old, new| {
        println!("Connection state: {:?} -> {:?}", old, new);
        if new == ConnectionState::Disconnected {
            eprintln!("Warning: Connection lost!");
        }
    }).await;

    // 启动监控
    monitor.start().await;

    // ... 应用逻辑 ...

    // 停止监控
    monitor.stop().await;
}
```

### 启用自动重连

```rust
use canlink_hal::monitor::{ConnectionMonitor, ReconnectConfig};

let mut monitor = ConnectionMonitor::with_reconnect(
    Arc::new(Mutex::new(backend)),
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
            eprintln!("Connection lost, will attempt reconnect...");
        }
        ConnectionState::Reconnecting => {
            println!("Reconnecting...");
        }
        ConnectionState::Connected => {
            if old == ConnectionState::Reconnecting {
                println!("Reconnected successfully!");
            }
        }
    }
}).await;

monitor.start().await;
```

## 测试要求

### 单元测试

1. **状态转换测试**
   - Connected -> Disconnected
   - Disconnected -> Reconnecting
   - Reconnecting -> Connected
   - Reconnecting -> Disconnected（达到重试上限）

2. **回调测试**
   - 状态变化时回调被调用
   - 多个回调都被调用
   - 回调参数正确

3. **重连测试**
   - 重连成功
   - 重连失败（达到上限）
   - 退避间隔正确

4. **心跳测试**
   - 心跳间隔正确
   - 心跳失败触发断开

### 集成测试

```rust
#[tokio::test]
async fn test_reconnect_on_disconnect() {
    let mock_backend = MockBackend::new();
    let mut monitor = ConnectionMonitor::with_reconnect(
        Arc::new(Mutex::new(mock_backend)),
        Duration::from_millis(100),
        ReconnectConfig {
            max_retries: 3,
            retry_interval: Duration::from_millis(50),
            backoff_multiplier: 1.0,
        },
    );

    let state_changes = Arc::new(Mutex::new(Vec::new()));
    let state_changes_clone = state_changes.clone();

    monitor.on_state_change(move |old, new| {
        // 记录状态变化
    }).await;

    monitor.start().await;

    // 模拟断开
    // ...

    // 验证重连
    // ...
}
```

## 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-01-10 | 初始版本 |
