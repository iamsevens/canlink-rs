//! 错误处理和重试策略示例
//!
//! 本示例演示如何在 `CANLink` 中实现健壮的错误处理和重试逻辑。
//!
//! ## 涵盖的主题
//!
//! - 错误类型识别和处理
//! - 自动重试机制
//! - 指数退避策略
//! - 错误恢复和降级
//! - 日志记录和监控
//!
//! ## 运行示例
//!
//! ```bash
//! cargo run --example error_handling
//! ```

use canlink_hal::{BackendConfig, CanBackend, CanError, CanMessage, CanResult};
use canlink_mock::{MockBackend, MockConfig};
use std::thread;
use std::time::{Duration, Instant};

/// 重试策略配置
#[derive(Debug, Clone)]
struct RetryPolicy {
    /// 最大重试次数
    max_retries: u32,
    /// 初始退避时间 (毫秒)
    initial_backoff_ms: u64,
    /// 最大退避时间 (毫秒)
    max_backoff_ms: u64,
    /// 退避倍数
    backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// 计算第 n 次重试的退避时间
    fn backoff_duration(&self, attempt: u32) -> Duration {
        let backoff_ms = (self.initial_backoff_ms as f64
            * self.backoff_multiplier.powi(attempt as i32))
        .min(self.max_backoff_ms as f64) as u64;

        Duration::from_millis(backoff_ms)
    }
}

/// 错误分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErrorCategory {
    /// 可重试的临时错误
    Transient,
    /// 不可重试的永久错误
    Permanent,
    /// 需要重新初始化的错误
    RequiresReinit,
}

/// 分析错误类型
fn categorize_error(error: &CanError) -> ErrorCategory {
    match error {
        // 临时错误 - 可以重试
        CanError::SendFailed { .. } => ErrorCategory::Transient,
        CanError::ReceiveTimeout => ErrorCategory::Transient,
        CanError::Timeout => ErrorCategory::Transient,

        // 需要重新初始化
        CanError::NotInitialized => ErrorCategory::RequiresReinit,
        CanError::ChannelNotOpen { .. } => ErrorCategory::RequiresReinit,

        // 永久错误 - 不应重试
        CanError::InvalidId { .. } => ErrorCategory::Permanent,
        CanError::InvalidDataLength { .. } => ErrorCategory::Permanent,
        CanError::UnsupportedFeature { .. } => ErrorCategory::Permanent,

        // 硬件错误 - 根据错误码判断
        CanError::HardwareError { code, .. } => {
            // 某些硬件错误可能是临时的
            if *code < 100 {
                ErrorCategory::Transient
            } else {
                ErrorCategory::Permanent
            }
        }

        // 其他错误默认为永久错误
        _ => ErrorCategory::Permanent,
    }
}

/// 带重试的发送消息
fn send_with_retry(
    backend: &mut dyn CanBackend,
    message: &CanMessage,
    policy: &RetryPolicy,
) -> CanResult<()> {
    let mut attempt = 0;

    loop {
        match backend.send_message(message) {
            Ok(_) => {
                if attempt > 0 {
                    println!("  ✓ 重试成功 (尝试 {}/{})", attempt + 1, policy.max_retries + 1);
                }
                return Ok(());
            }
            Err(e) => {
                let category = categorize_error(&e);

                match category {
                    ErrorCategory::Permanent => {
                        println!("  ✗ 永久错误，不重试: {:?}", e);
                        return Err(e);
                    }
                    ErrorCategory::RequiresReinit => {
                        println!("  ⚠ 需要重新初始化: {:?}", e);
                        return Err(e);
                    }
                    ErrorCategory::Transient => {
                        if attempt >= policy.max_retries {
                            println!(
                                "  ✗ 达到最大重试次数 ({}), 放弃",
                                policy.max_retries
                            );
                            return Err(e);
                        }

                        let backoff = policy.backoff_duration(attempt);
                        println!(
                            "  ⟳ 临时错误，重试 {}/{} (等待 {:?}): {:?}",
                            attempt + 1,
                            policy.max_retries,
                            backoff,
                            e
                        );

                        thread::sleep(backoff);
                        attempt += 1;
                    }
                }
            }
        }
    }
}

/// 带超时的接收消息
fn receive_with_timeout(
    backend: &mut dyn CanBackend,
    timeout: Duration,
) -> CanResult<Option<CanMessage>> {
    let start = Instant::now();

    while start.elapsed() < timeout {
        match backend.receive_message() {
            Ok(Some(msg)) => return Ok(Some(msg)),
            Ok(None) => {
                // 没有消息，继续等待
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                let category = categorize_error(&e);
                if category == ErrorCategory::Transient {
                    // 临时错误，继续尝试
                    thread::sleep(Duration::from_millis(50));
                } else {
                    return Err(e);
                }
            }
        }
    }

    Err(CanError::Timeout)
}

/// 带自动恢复的后端包装器
struct ResilientBackend {
    backend: Box<dyn CanBackend>,
    config: BackendConfig,
    retry_policy: RetryPolicy,
}

impl ResilientBackend {
    fn new(backend: Box<dyn CanBackend>, config: BackendConfig) -> Self {
        Self {
            backend,
            config,
            retry_policy: RetryPolicy::default(),
        }
    }

    /// 尝试重新初始化后端
    fn reinitialize(&mut self) -> CanResult<()> {
        println!("  ⟳ 尝试重新初始化后端...");

        // 先关闭
        let _ = self.backend.close();

        // 重新初始化
        self.backend.initialize(&self.config)?;

        println!("  ✓ 后端重新初始化成功");
        Ok(())
    }

    /// 发送消息，带自动恢复
    fn send_message_resilient(&mut self, message: &CanMessage) -> CanResult<()> {
        match send_with_retry(&mut *self.backend, message, &self.retry_policy) {
            Ok(_) => Ok(()),
            Err(e) => {
                let category = categorize_error(&e);
                if category == ErrorCategory::RequiresReinit {
                    // 尝试重新初始化
                    self.reinitialize()?;
                    // 再次尝试发送
                    send_with_retry(&mut *self.backend, message, &self.retry_policy)
                } else {
                    Err(e)
                }
            }
        }
    }
}

/// 演示不同的错误场景
fn demonstrate_error_scenarios() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 错误处理和重试策略示例 ===\n");

    // 场景 1: 临时错误 - 自动重试
    println!("--- 场景 1: 临时错误自动重试 ---");
    {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config)?;
        backend.open_channel(0)?;

        // 注入 3 次发送失败
        backend.error_injector_mut().inject_send_error_with_config(
            CanError::SendFailed {
                reason: "Bus busy".to_string(),
            },
            3, // 失败 3 次
            0, // 立即开始
        );

        let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;
        let policy = RetryPolicy::default();

        match send_with_retry(&mut backend, &msg, &policy) {
            Ok(_) => println!("✓ 消息最终发送成功\n"),
            Err(e) => println!("✗ 发送失败: {:?}\n", e),
        }
    }

    // 场景 2: 永久错误 - 不重试
    println!("--- 场景 2: 永久错误不重试 ---");
    {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config)?;
        backend.open_channel(0)?;

        // 尝试发送无效的消息
        let invalid_data = vec![0u8; 100]; // 太长
        match CanMessage::new_standard(0x123, &invalid_data) {
            Ok(msg) => {
                let policy = RetryPolicy::default();
                let _ = send_with_retry(&mut backend, &msg, &policy);
            }
            Err(e) => {
                println!("  ✗ 消息创建失败 (永久错误): {:?}", e);
                println!("  → 不会重试\n");
            }
        }
    }

    // 场景 3: 需要重新初始化
    println!("--- 场景 3: 自动重新初始化 ---");
    {
        let backend = Box::new(MockBackend::new());
        let config = BackendConfig::new("mock");
        let mut resilient = ResilientBackend::new(backend, config.clone());

        resilient.backend.initialize(&config)?;
        resilient.backend.open_channel(0)?;

        // 模拟通道关闭错误
        resilient
            .backend
            .error_injector_mut()
            .inject_send_error(CanError::ChannelNotOpen { channel: 0 });

        let msg = CanMessage::new_standard(0x123, &[1, 2, 3, 4])?;

        match resilient.send_message_resilient(&msg) {
            Ok(_) => println!("✓ 消息发送成功 (经过自动恢复)\n"),
            Err(e) => println!("✗ 发送失败: {:?}\n", e),
        }
    }

    // 场景 4: 接收超时
    println!("--- 场景 4: 接收超时处理 ---");
    {
        let mut backend = MockBackend::new();
        let config = BackendConfig::new("mock");
        backend.initialize(&config)?;
        backend.open_channel(0)?;

        println!("  等待消息 (超时 1 秒)...");
        let timeout = Duration::from_secs(1);

        match receive_with_timeout(&mut backend, timeout) {
            Ok(Some(msg)) => println!("  ✓ 收到消息: {:?}\n", msg),
            Ok(None) => println!("  ⚠ 没有消息\n"),
            Err(CanError::Timeout) => println!("  ⏱ 接收超时\n"),
            Err(e) => println!("  ✗ 接收错误: {:?}\n", e),
        }
    }

    // 场景 5: 指数退避演示
    println!("--- 场景 5: 指数退避策略 ---");
    {
        let policy = RetryPolicy {
            max_retries: 5,
            initial_backoff_ms: 100,
            max_backoff_ms: 2000,
            backoff_multiplier: 2.0,
        };

        println!("  重试策略: {:?}", policy);
        println!("  退避时间序列:");
        for i in 0..policy.max_retries {
            let backoff = policy.backoff_duration(i);
            println!("    尝试 {}: {:?}", i + 1, backoff);
        }
        println!();
    }

    println!("=== 示例完成 ===");

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    demonstrate_error_scenarios()
}
