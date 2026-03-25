# 快速入门指南: CAN 硬件抽象层

**版本**: 1.0.0
**日期**: 2026-01-08
**目的**: 帮助开发者快速上手使用 CAN 硬件抽象层

## 概述

CAN 硬件抽象层（`canlink-hal`）提供统一的接口来操作不同品牌的 CAN 硬件。通过这个抽象层，你可以：

- ✅ 使用相同的代码操作不同硬件（TSMaster、PEAK、Kvaser 等）
- ✅ 在没有物理硬件的情况下测试应用（使用 Mock 后端）
- ✅ 通过配置文件轻松切换硬件后端
- ✅ 查询硬件能力并适配应用行为

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
canlink-hal = "1.0"
canlink-mock = "0.2"  # Mock 后端用于测试

# 如果需要异步支持
# canlink-hal = { version = "1.0", features = ["async-tokio"] }
```

## 5 分钟快速开始

### 步骤 1: 注册后端

```rust
use canlink_hal::{BackendRegistry, CanBackend};
use canlink_mock::MockBackendFactory;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 获取全局注册表
    let registry = BackendRegistry::global();

    // 注册 Mock 后端
    registry.register(Box::new(MockBackendFactory::new()))?;

    println!("Available backends: {:?}", registry.list_backends());

    Ok(())
}
```

### 步骤 2: 创建并初始化后端

```rust
use canlink_hal::{BackendConfig, CanMessage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new()))?;

    // 创建后端实例
    let mut backend = registry.create("mock")?;

    // 配置后端
    let config = BackendConfig {
        backend_name: "mock".to_string(),
        parameters: Default::default(),
    };

    // 初始化
    backend.initialize(&config)?;

    println!("Backend initialized successfully!");

    // 使用完毕后关闭
    backend.close()?;

    Ok(())
}
```

### 步骤 3: 发送和接收消息

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new()))?;

    let mut backend = registry.create("mock")?;
    let config = BackendConfig {
        backend_name: "mock".to_string(),
        parameters: Default::default(),
    };
    backend.initialize(&config)?;

    // 发送标准 CAN 消息
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04])?;
    backend.send_message(&msg)?;
    println!("Message sent: {:?}", msg);

    // 接收消息（非阻塞）
    if let Some(received) = backend.receive_message()? {
        println!("Message received: {:?}", received);
    } else {
        println!("No message available");
    }

    backend.close()?;
    Ok(())
}
```

## 常见使用场景

### 场景 1: 使用配置文件切换后端

**创建配置文件 `canlink.toml`**:

```toml
[backend]
backend_name = "mock"

# 或者使用真实硬件
# [backend]
# backend_name = "tsmaster"
# device_index = 0
# channel = 0
# bitrate = 500000
```

**从配置文件加载**:

```rust
use canlink_hal::CanlinkConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 注册所有可用后端
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new()))?;
    // registry.register(Box::new(TsMasterBackendFactory::new()))?;

    // 从配置文件加载
    let config = CanlinkConfig::from_file("canlink.toml")?;

    // 创建配置指定的后端
    let mut backend = registry.create(&config.backend.backend_name)?;
    backend.initialize(&config.backend)?;

    // 使用后端...

    backend.close()?;
    Ok(())
}
```

### 场景 2: 查询硬件能力

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new()))?;

    let backend = registry.create("mock")?;

    // 查询硬件能力
    let capability = backend.get_capability()?;

    println!("Hardware Capability:");
    println!("  Channels: {}", capability.channel_count);
    println!("  CAN-FD: {}", capability.supports_canfd);
    println!("  Max bitrate: {} bps", capability.max_bitrate);
    println!("  Supported bitrates: {:?}", capability.supported_bitrates);

    // 根据能力调整应用行为
    if capability.supports_canfd {
        println!("Using CAN-FD mode");
        let msg = CanMessage::new_fd(
            CanId::Standard(0x123),
            &[0u8; 64]  // 64 字节数据
        )?;
        // backend.send_message(&msg)?;
    } else {
        println!("Using CAN 2.0 mode");
        let msg = CanMessage::new_standard(0x123, &[0x01, 0x02])?;
        // backend.send_message(&msg)?;
    }

    Ok(())
}
```

### 场景 3: 发送不同类型的消息

```rust
fn send_various_messages(backend: &mut Box<dyn CanBackend>) -> Result<(), Box<dyn std::error::Error>> {
    // 1. 标准 CAN 2.0A 数据帧（11 位 ID）
    let std_msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03])?;
    backend.send_message(&std_msg)?;

    // 2. 扩展 CAN 2.0B 数据帧（29 位 ID）
    let ext_msg = CanMessage::new_extended(0x12345678, &[0x04, 0x05, 0x06])?;
    backend.send_message(&ext_msg)?;

    // 3. 远程帧（RTR）
    let remote_msg = CanMessage::new_remote(CanId::Standard(0x456), 8)?;
    backend.send_message(&remote_msg)?;

    // 4. CAN-FD 消息（如果硬件支持）
    let capability = backend.get_capability()?;
    if capability.supports_canfd {
        let fd_msg = CanMessage::new_fd(
            CanId::Standard(0x789),
            &[0u8; 64]  // 最多 64 字节
        )?;
        backend.send_message(&fd_msg)?;
    }

    Ok(())
}
```

### 场景 4: 多线程使用

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new()))?;

    let mut backend = registry.create("mock")?;
    let config = BackendConfig {
        backend_name: "mock".to_string(),
        parameters: Default::default(),
    };
    backend.initialize(&config)?;

    // 使用 Arc<Mutex<>> 在多线程间共享后端
    let backend = Arc::new(Mutex::new(backend));

    // 发送线程
    let backend_tx = backend.clone();
    let tx_thread = thread::spawn(move || {
        for i in 0..10 {
            let msg = CanMessage::new_standard(0x100 + i, &[i as u8]).unwrap();
            backend_tx.lock().unwrap().send_message(&msg).unwrap();
        }
    });

    // 接收线程
    let backend_rx = backend.clone();
    let rx_thread = thread::spawn(move || {
        for _ in 0..10 {
            if let Some(msg) = backend_rx.lock().unwrap().receive_message().unwrap() {
                println!("Received: {:?}", msg);
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    tx_thread.join().unwrap();
    rx_thread.join().unwrap();

    backend.lock().unwrap().close()?;
    Ok(())
}
```

### 场景 5: 使用 Mock 后端进行测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use canlink_mock::MockBackend;

    #[test]
    fn test_message_echo() {
        let registry = BackendRegistry::global();
        registry.register(Box::new(MockBackendFactory::new())).unwrap();

        let mut backend = registry.create("mock").unwrap();
        let config = BackendConfig {
            backend_name: "mock".to_string(),
            parameters: Default::default(),
        };
        backend.initialize(&config).unwrap();

        // Mock 后端会回显发送的消息
        let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03]).unwrap();
        backend.send_message(&msg).unwrap();

        // 接收回显的消息
        let received = backend.receive_message().unwrap();
        assert!(received.is_some());
        assert_eq!(received.unwrap().id, msg.id);

        backend.close().unwrap();
    }
}
```

## 异步支持（可选）

如果启用了 `async` feature，可以使用异步 API：

```rust
use canlink_hal::CanBackendAsync;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new()))?;

    let mut backend = registry.create("mock")?;
    let config = BackendConfig {
        backend_name: "mock".to_string(),
        parameters: Default::default(),
    };
    backend.initialize(&config)?;

    // 异步发送
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03])?;
    backend.send_message_async(&msg).await?;

    // 异步接收（带超时）
    let timeout = std::time::Duration::from_secs(1);
    match backend.receive_message_async(Some(timeout)).await {
        Ok(msg) => println!("Received: {:?}", msg),
        Err(CanError::Timeout) => println!("No message within timeout"),
        Err(e) => return Err(e.into()),
    }

    backend.close()?;
    Ok(())
}
```

## 错误处理

```rust
use canlink_hal::CanError;

fn handle_errors(backend: &mut Box<dyn CanBackend>) {
    let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03]).unwrap();

    match backend.send_message(&msg) {
        Ok(()) => println!("Message sent successfully"),
        Err(CanError::SendFailed(reason)) => {
            eprintln!("Send failed: {}", reason);
        }
        Err(CanError::UnsupportedFeature(feature)) => {
            eprintln!("Hardware doesn't support: {}", feature);
        }
        Err(e) => {
            eprintln!("Other error: {}", e);
        }
    }
}
```

## 最佳实践

### 1. 使用全局注册表

```rust
// ✅ 推荐：在应用启动时注册所有后端
fn init_backends() {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new())).unwrap();
    // 注册其他后端...
}

// ❌ 不推荐：每次使用时都创建新注册表
```

### 2. 总是关闭后端

```rust
// ✅ 推荐：使用 RAII 模式
struct BackendGuard {
    backend: Box<dyn CanBackend>,
}

impl Drop for BackendGuard {
    fn drop(&mut self) {
        let _ = self.backend.close();
    }
}

// 或者使用 defer 模式
fn use_backend() -> Result<(), CanError> {
    let mut backend = create_backend()?;
    backend.initialize(&config)?;

    // 使用 defer 确保关闭
    let _guard = scopeguard::guard((), |_| {
        let _ = backend.close();
    });

    // 使用后端...

    Ok(())
}
```

### 3. 检查硬件能力

```rust
// ✅ 推荐：发送前检查能力
let capability = backend.get_capability()?;
if !capability.supports_canfd {
    return Err(CanError::UnsupportedFeature("CAN-FD".to_string()));
}

// ❌ 不推荐：直接发送，依赖错误处理
```

### 4. 使用配置文件

```rust
// ✅ 推荐：使用配置文件，便于切换后端
let config = CanlinkConfig::from_file("canlink.toml")?;
let backend = registry.create(&config.backend.backend_name)?;

// ❌ 不推荐：硬编码后端名称
let backend = registry.create("tsmaster")?;
```

## 下一步

- 📖 阅读 [API 文档](contracts/backend-trait.md)
- 🔧 查看 [数据模型](data-model.md)
- 🧪 参考 [Mock 后端示例](../../canlink-mock/examples/)
- 🚀 实现自己的硬件后端

## 常见问题

### Q: 如何实现新的硬件后端？

A: 实现 `CanBackend` trait 和 `BackendFactory` trait，参考 `canlink-mock` 的实现。

### Q: 可以同时使用多个后端吗？

A: 可以。每个后端实例是独立的，可以同时创建和使用多个后端。

### Q: Mock 后端适合生产环境吗？

A: 不适合。Mock 后端仅用于测试和开发，不应在生产环境使用。

### Q: 如何处理硬件断开？

A: 监听 `CanError::SendFailed` 或 `CanError::ReceiveFailed` 错误，实现重连逻辑。

### Q: 性能开销有多大？

A: 抽象层设计为零成本抽象，性能开销 < 5%。具体取决于硬件后端实现。

## 获取帮助

- 📝 查看完整文档：`cargo doc --open`
- 🐛 报告问题：[GitHub Issues](https://github.com/iamsevens/canlink-rs/issues)
- 💬 讨论：[GitHub Discussions](https://github.com/iamsevens/canlink-rs/discussions)
