# 快速入门: 周期性消息发送与 ISO-TP

本指南帮助您快速上手 CANLink v0.3.0 的周期发送和 ISO-TP 功能。

## 目录

1. [安装配置](#安装配置)
2. [周期性消息发送](#周期性消息发送)
3. [ISO-TP 基础用法](#iso-tp-基础用法)
4. [CLI 命令](#cli-命令)
5. [常见问题](#常见问题)

---

## 安装配置

### Cargo.toml

```toml
[dependencies]
canlink-hal = { version = "0.3", features = ["periodic", "isotp"] }
canlink-mock = "0.3"  # 用于测试
tokio = { version = "1", features = ["full"] }
```

### Feature Flags

| Feature | 描述 | 依赖 |
|---------|------|------|
| `periodic` | 周期性消息发送 | tokio |
| `isotp` | ISO-TP 协议支持 | - |
| `full` | 所有功能 | tokio, tracing, notify |

---

## 周期性消息发送

### 基本用法

```rust
use canlink_hal::{BackendConfig, CanBackend, CanMessage};
use canlink_hal::periodic::{run_scheduler, PeriodicScheduler, PeriodicMessage};
use canlink_mock::MockBackend;
use std::time::Duration;
use tokio::task::LocalSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用 LocalSet 因为 MockBackend 不是 Send
    let local = LocalSet::new();

    local.run_until(async {
        // 1. 创建并初始化后端
        let mut backend = MockBackend::new();
        backend.initialize(&BackendConfig::new("mock")).unwrap();
        backend.open_channel(0).unwrap();

        // 2. 创建周期调度器
        let (scheduler, command_rx) = PeriodicScheduler::new(64);

        // 3. 启动调度器循环
        tokio::task::spawn_local(run_scheduler(backend, command_rx, 32));

        // 4. 创建周期消息 (100ms 间隔)
        let msg = CanMessage::new_standard(0x123, &[0x01, 0x02, 0x03, 0x04]).unwrap();
        let periodic = PeriodicMessage::new(msg, Duration::from_millis(100)).unwrap();

        // 5. 添加到调度器
        let id = scheduler.add(periodic).await.unwrap();
        println!("Added periodic message with ID: {}", id);

        // 6. 让它运行一段时间
        tokio::time::sleep(Duration::from_secs(1)).await;

        // 7. 查看统计
        if let Ok(Some(stats)) = scheduler.get_stats(id).await {
            println!("Messages sent: {}", stats.send_count());
            if let Some(avg) = stats.average_interval() {
                println!("Average interval: {:?}", avg);
            }
        }

        // 8. 停止
        scheduler.shutdown().await.unwrap();
    }).await;

    Ok(())
}
```

### 动态更新数据

```rust
// 更新消息数据（不中断发送周期）
scheduler.update_data(id, vec![0x05, 0x06, 0x07, 0x08]).await?;

// 更新发送间隔
scheduler.update_interval(id, Duration::from_millis(200)).await?;

// 暂停发送
scheduler.set_enabled(id, false).await?;

// 恢复发送
scheduler.set_enabled(id, true).await?;
```

### 多消息周期发送

```rust
// 添加多个不同间隔的周期消息
let heartbeat = PeriodicMessage::new(
    CanMessage::new_standard(0x100, &[0x00])?,
    Duration::from_millis(50),  // 50ms 心跳
)?;

let status = PeriodicMessage::new(
    CanMessage::new_standard(0x200, &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08])?,
    Duration::from_millis(500), // 500ms 状态
)?;

let slow_data = PeriodicMessage::new(
    CanMessage::new_standard(0x300, &[0xAA, 0xBB])?,
    Duration::from_secs(1),     // 1s 慢速数据
)?;

let id1 = scheduler.add(heartbeat).await?;
let id2 = scheduler.add(status).await?;
let id3 = scheduler.add(slow_data).await?;

// 列出所有周期消息
let ids = scheduler.list_ids().await;
println!("Active periodic messages: {:?}", ids);
```

---

## ISO-TP 基础用法

### 发送大数据

```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig, StMin};
use canlink_hal::{BackendConfig, CanBackend};
use canlink_mock::MockBackend;
use std::time::Duration;
use tokio::task::LocalSet;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local = LocalSet::new();

    local.run_until(async {
        let mut backend = MockBackend::new();
        backend.initialize(&BackendConfig::new("mock")).unwrap();
        backend.open_channel(0).unwrap();

        // 配置 ISO-TP 通道
        let config = IsoTpConfig::builder()
            .tx_id(0x7E0)           // 发送 ID (诊断请求)
            .rx_id(0x7E8)           // 接收 ID (诊断响应)
            .block_size(0)          // 无块大小限制
            .st_min(StMin::Milliseconds(10))
            .timeout(Duration::from_millis(1000))
            .build()
            .unwrap();

        let mut channel = IsoTpChannel::new(backend, config).unwrap();

        // 发送大于 7 字节的数据（自动分段）
        let data = vec![0x22, 0xF1, 0x90]; // UDS: Read Data By Identifier
        channel.send(&data).await.unwrap();
        println!("Sent {} bytes", data.len());
    }).await;

    Ok(())
}
```

### 接收大数据

```rust
// 接收响应（自动重组）
match channel.receive().await {
    Ok(response) => {
        println!("Received {} bytes: {:02X?}", response.len(), response);
    }
    Err(e) => {
        eprintln!("Receive error: {}", e);
    }
}
```

### 完整的请求-响应示例

```rust
use canlink_hal::isotp::{IsoTpChannel, IsoTpConfig, IsoTpError};
use canlink_hal::CanBackendAsync;

async fn uds_request<B: CanBackendAsync>(
    channel: &mut IsoTpChannel<B>,
    request: &[u8],
) -> Result<Vec<u8>, IsoTpError> {
    // 发送请求
    channel.send(request).await?;

    // 接收响应
    let response = channel.receive().await?;

    // 检查否定响应
    if response.len() >= 3 && response[0] == 0x7F {
        eprintln!("Negative response: service=0x{:02X}, NRC=0x{:02X}",
                  response[1], response[2]);
    }

    Ok(response)
}
```

### 使用回调监控传输

```rust
use canlink_hal::isotp::{IsoTpCallback, TransferDirection, IsoTpError};

struct MyCallback;

impl IsoTpCallback for MyCallback {
    fn on_transfer_start(&self, direction: TransferDirection, total_length: usize) {
        println!("{:?} started: {} bytes", direction, total_length);
    }

    fn on_transfer_progress(&self, direction: TransferDirection, transferred: usize, total: usize) {
        let percent = (transferred as f64 / total as f64) * 100.0;
        println!("{:?} progress: {:.1}%", direction, percent);
    }

    fn on_transfer_complete(&self, direction: TransferDirection, data: &[u8]) {
        println!("{:?} complete: {} bytes", direction, data.len());
    }

    fn on_transfer_error(&self, direction: TransferDirection, error: &IsoTpError) {
        eprintln!("{:?} error: {}", direction, error);
    }
}

// 设置回调
channel.set_callback(Box::new(MyCallback));
```

### CAN-FD 模式

```rust
use canlink_hal::isotp::FrameSize;

// 强制使用 CAN-FD 模式（64 字节/帧）
let config = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .frame_size(FrameSize::Fd64)  // 强制 CAN-FD
    .build()?;

// 或者自动检测
let config = IsoTpConfig::builder()
    .tx_id(0x7E0)
    .rx_id(0x7E8)
    .frame_size(FrameSize::Auto)  // 根据后端能力自动选择
    .build()?;
```

---

## CLI 命令

### 周期发送

```bash
# 以 100ms 间隔周期发送消息
canlink send --id 0x123 --data "01 02 03 04" --periodic 100

# 发送 10 次后停止
canlink send --id 0x123 --data "01 02 03 04" --periodic 100 --count 10

# 使用扩展帧 ID
canlink send --id 0x12345678 --extended --data "AA BB" --periodic 500
```

### ISO-TP 发送

```bash
# 发送 ISO-TP 消息
canlink isotp send --tx-id 0x7E0 --rx-id 0x7E8 --data "22 F1 90"

# 发送并等待响应
canlink isotp send --tx-id 0x7E0 --rx-id 0x7E8 --data "22 F1 90" --wait-response

# 设置超时
canlink isotp send --tx-id 0x7E0 --rx-id 0x7E8 --data "22 F1 90" --timeout 2000
```

### ISO-TP 接收

```bash
# 接收 ISO-TP 消息
canlink isotp receive --rx-id 0x7E8 --tx-id 0x7E0

# 持续接收
canlink isotp receive --rx-id 0x7E8 --tx-id 0x7E0 --continuous

# JSON 输出
canlink isotp receive --rx-id 0x7E8 --tx-id 0x7E0 --format json
```

---

## 常见问题

### Q: 周期发送精度不够怎么办？

A: 周期发送精度受操作系统调度影响。建议：
1. 使用 `PeriodicStats` 监控实际间隔
2. 对于高精度需求，考虑使用实时操作系统
3. 避免在同一进程中运行 CPU 密集型任务

```rust
// 检查精度
let stats = scheduler.get_stats(id).await?.unwrap();
if let (Some(min), Some(max)) = (stats.min_interval(), stats.max_interval()) {
    let jitter = max - min;
    println!("Jitter: {:?}", jitter);
}
```

### Q: ISO-TP 接收超时怎么处理？

A: 超时通常表示对方未响应或 Flow Control 配置不匹配。

```rust
match channel.receive().await {
    Ok(data) => { /* 处理数据 */ }
    Err(IsoTpError::RxTimeout { timeout_ms }) => {
        eprintln!("Timeout after {}ms - check if ECU is responding", timeout_ms);
        // 可能需要重试或检查连接
    }
    Err(IsoTpError::FcTimeout { timeout_ms }) => {
        eprintln!("FC timeout - ECU may not support ISO-TP");
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### Q: 如何处理 ISO-TP 序列号错误？

A: 序列号错误表示帧丢失或乱序。

```rust
match channel.receive().await {
    Err(IsoTpError::SequenceMismatch { expected, actual }) => {
        eprintln!("Sequence error: expected {}, got {}", expected, actual);
        // 重置通道并重试
        channel.reset();
    }
    // ...
}
```

### Q: 如何在测试中模拟 ISO-TP 响应？

A: 使用 MockBackend 的预设消息功能。

```rust
use canlink_mock::{MockBackend, MockConfig};

// 预设 ISO-TP 响应帧
let responses = vec![
    // Single Frame 响应
    CanMessage::new_standard(0x7E8, &[0x03, 0x62, 0xF1, 0x90])?,
];

let config = MockConfig::with_preset_messages(responses);
let backend = MockBackend::with_config(config);

// 现在 channel.receive() 会返回预设的响应
```

---

## 下一步

- 查看 [API 参考](../../docs/api-reference.md) 了解完整 API
- 阅读 [研究文档](research.md) 了解技术决策
- 查看 [数据模型](data-model.md) 了解类型定义
- 运行示例: `cargo run --example periodic_send`

---

**版本**: 0.3.0
**更新日期**: 2026-01-12
