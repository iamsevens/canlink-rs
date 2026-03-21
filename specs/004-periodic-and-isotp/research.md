# 研究文档: 周期性消息发送与 ISO-TP 支持

**功能**: 004-periodic-and-isotp
**日期**: 2026-01-12
**阶段**: Phase 0 Research

---

## 1. 周期性消息发送研究

### 1.1 现有实现分析

#### Rust 生态中的定时器方案

| 方案 | 精度 | 异步支持 | 优点 | 缺点 |
|------|------|----------|------|------|
| `tokio::time::interval` | ~1ms | ✅ | 与现有异步架构集成 | 依赖 tokio 运行时 |
| `std::thread::sleep` | ~1ms | ❌ | 无依赖 | 阻塞线程，不适合多任务 |
| `async-std::task::sleep` | ~1ms | ✅ | 轻量级 | 需要额外依赖 |
| `spin_sleep` | ~100μs | ❌ | 高精度 | CPU 占用高 |

**决策**: 使用 `tokio::time::interval`
- 项目已依赖 tokio（v0.2.0 异步 API）
- 支持多个定时器并发运行
- 自动处理漂移补偿（MissedTickBehavior）

#### tokio interval 漂移处理

```rust
use tokio::time::{interval, MissedTickBehavior};

let mut interval = interval(Duration::from_millis(100));
// 选项:
// - Burst: 尽快补发错过的 tick（默认）
// - Delay: 从当前时间重新计算
// - Skip: 跳过错过的 tick
interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
```

对于周期发送，推荐 `Delay` 模式，避免消息突发。

### 1.2 调度器设计

#### 方案 A: 每消息独立任务

```rust
// 每个周期消息一个 tokio task
for msg in periodic_messages {
    tokio::spawn(async move {
        let mut interval = interval(msg.interval);
        loop {
            interval.tick().await;
            backend.send_message(&msg.message).await;
        }
    });
}
```

**优点**: 简单，消息间完全独立
**缺点**: 大量任务开销，难以统一管理

#### 方案 B: 单任务轮询

```rust
// 单个任务管理所有周期消息
tokio::spawn(async move {
    loop {
        let now = Instant::now();
        for msg in &mut periodic_messages {
            if msg.next_send <= now {
                backend.send_message(&msg.message).await;
                msg.next_send = now + msg.interval;
            }
        }
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
});
```

**优点**: 资源占用低，统一管理
**缺点**: 轮询开销，精度受限于轮询间隔

#### 方案 C: 优先队列调度（推荐）

```rust
use std::collections::BinaryHeap;

struct ScheduledMessage {
    next_send: Instant,
    message: PeriodicMessage,
}

// 使用优先队列，只等待最近的发送时间
tokio::spawn(async move {
    let mut heap: BinaryHeap<Reverse<ScheduledMessage>> = ...;
    loop {
        if let Some(Reverse(next)) = heap.peek() {
            tokio::time::sleep_until(next.next_send.into()).await;
            let Reverse(mut msg) = heap.pop().unwrap();
            backend.send_message(&msg.message.message).await;
            msg.next_send = Instant::now() + msg.message.interval;
            heap.push(Reverse(msg));
        }
    }
});
```

**优点**: 高效，只在需要时唤醒，支持大量消息
**缺点**: 实现稍复杂

**决策**: 采用方案 C（优先队列调度）

### 1.3 数据更新策略

周期发送中动态更新消息数据的方案：

#### 方案 1: Arc<RwLock<Data>>

```rust
struct PeriodicMessage {
    id: CanId,
    data: Arc<RwLock<Vec<u8>>>,
    interval: Duration,
}
```

**优点**: 线程安全，读写分离
**缺点**: 锁竞争可能影响发送精度

#### 方案 2: 原子替换（推荐）

```rust
struct PeriodicMessage {
    id: CanId,
    data: Arc<ArcSwap<Vec<u8>>>,  // 使用 arc-swap crate
    interval: Duration,
}
```

**优点**: 无锁读取，更新时原子替换
**缺点**: 需要额外依赖

#### 方案 3: Channel 通知

```rust
// 通过 channel 发送更新命令
enum SchedulerCommand {
    UpdateData { id: u32, data: Vec<u8> },
    Stop { id: u32 },
    Add { msg: PeriodicMessage },
}
```

**优点**: 清晰的命令模式，易于扩展
**缺点**: 异步通信开销

**决策**: 采用方案 3（Channel 通知），与调度器架构一致

---

## 2. ISO-TP 协议研究

### 2.1 ISO 15765-2 协议概述

ISO-TP（ISO 15765-2）是 CAN 总线上的传输层协议，用于传输超过单帧容量的数据。

#### 帧类型

| 类型 | PCI | 用途 | 数据容量 (CAN 2.0) |
|------|-----|------|-------------------|
| Single Frame (SF) | 0x0N | 单帧传输 ≤7 字节 | 1-7 字节 |
| First Frame (FF) | 0x1X XX | 多帧传输首帧 | 6 字节 |
| Consecutive Frame (CF) | 0x2N | 多帧传输后续帧 | 7 字节 |
| Flow Control (FC) | 0x3X | 流控制 | N/A |

#### PCI 编码详解

```
Single Frame:
  Byte 0: 0x0N (N = data length, 1-7)
  Bytes 1-7: Data

First Frame:
  Byte 0: 0x1L (L = high nibble of length)
  Byte 1: LL (low byte of length)
  Bytes 2-7: First 6 bytes of data
  Total length = (L << 8) | LL (max 4095)

Consecutive Frame:
  Byte 0: 0x2N (N = sequence number, 0-F, wraps)
  Bytes 1-7: Data

Flow Control:
  Byte 0: 0x3S (S = Flow Status)
  Byte 1: BS (Block Size, 0 = no limit)
  Byte 2: STmin (Separation Time minimum)
```

#### Flow Status 值

| 值 | 名称 | 含义 |
|----|------|------|
| 0x00 | ContinueToSend (CTS) | 继续发送 |
| 0x01 | Wait | 暂停，等待下一个 FC |
| 0x02 | Overflow | 缓冲区溢出，中止传输 |

#### STmin 编码

| 值范围 | 含义 |
|--------|------|
| 0x00-0x7F | 0-127 毫秒 |
| 0x80-0xF0 | 保留 |
| 0xF1-0xF9 | 100-900 微秒 |
| 0xFA-0xFF | 保留 |

### 2.2 CAN-FD 模式下的 ISO-TP

CAN-FD 支持最大 64 字节/帧，ISO-TP 帧格式相应扩展：

| 帧类型 | CAN 2.0 数据容量 | CAN-FD 数据容量 |
|--------|-----------------|-----------------|
| SF | 1-7 字节 | 1-62 字节 |
| FF | 6 字节 | 62 字节 |
| CF | 7 字节 | 63 字节 |

CAN-FD 模式下 FF 的长度字段扩展：
- 如果 length ≤ 4095: 使用标准 2 字节编码
- 如果 length > 4095: 使用 6 字节编码（Byte 0-1 = 0x10 00, Bytes 2-5 = 32位长度）

**v0.3.0 范围**: 仅支持 ≤ 4095 字节，不实现扩展长度

### 2.3 状态机设计

#### 接收状态机

```
                    ┌─────────────┐
                    │    Idle     │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌─────────┐  ┌─────────┐  ┌─────────┐
        │ SF Recv │  │ FF Recv │  │ Invalid │
        └────┬────┘  └────┬────┘  └────┬────┘
             │            │            │
             │            ▼            │
             │      ┌─────────┐        │
             │      │ Send FC │        │
             │      └────┬────┘        │
             │            │            │
             │            ▼            │
             │      ┌─────────┐        │
             │      │Receiving│◄───────┤
             │      │   CFs   │        │
             │      └────┬────┘        │
             │            │            │
             │    ┌───────┼───────┐    │
             │    │       │       │    │
             │    ▼       ▼       ▼    │
             │ Complete Timeout Error  │
             │    │       │       │    │
             └────┴───────┴───────┴────┘
                          │
                          ▼
                    ┌─────────────┐
                    │    Idle     │
                    └─────────────┘
```

#### 发送状态机

```
                    ┌─────────────┐
                    │    Idle     │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
        ┌─────────┐  ┌─────────┐  ┌─────────┐
        │ Send SF │  │ Send FF │  │  Error  │
        │ (≤7 B)  │  │ (>7 B)  │  │(>4095 B)│
        └────┬────┘  └────┬────┘  └────┬────┘
             │            │            │
             │            ▼            │
             │      ┌─────────┐        │
             │      │Wait FC  │        │
             │      └────┬────┘        │
             │            │            │
             │    ┌───────┼───────┐    │
             │    │       │       │    │
             │    ▼       ▼       ▼    │
             │   CTS    Wait  Overflow │
             │    │       │       │    │
             │    ▼       │       │    │
             │ ┌──────┐   │       │    │
             │ │Send  │   │       │    │
             │ │CFs   │◄──┘       │    │
             │ └──┬───┘           │    │
             │    │               │    │
             │    ▼               │    │
             │ Complete           │    │
             │    │               │    │
             └────┴───────────────┴────┘
                          │
                          ▼
                    ┌─────────────┐
                    │    Idle     │
                    └─────────────┘
```

### 2.4 Rust 生态中的 ISO-TP 实现

| Crate | 状态 | 特点 | 问题 |
|-------|------|------|------|
| `isotp-rs` | 活跃 | 纯 Rust，无依赖 | 仅编解码，无状态机 |
| `can-isotp` | 不活跃 | Linux SocketCAN 绑定 | 仅 Linux |
| `automotive_diag` | 活跃 | 包含 ISO-TP | 与 UDS 耦合 |

**决策**: 自行实现 ISO-TP 模块
- 与 canlink-hal 后端抽象集成
- 支持 CAN 2.0 和 CAN-FD
- 可独立测试（使用 MockBackend）

### 2.5 地址模式

ISO-TP 支持多种地址模式：

| 模式 | 描述 | CAN ID 使用 |
|------|------|-------------|
| Normal | 标准地址 | 11/29 位 CAN ID 直接标识 |
| Extended | 扩展地址 | CAN ID + 1 字节目标地址 |
| Mixed | 混合地址 | 11 位 CAN ID + 1 字节地址扩展 |

**v0.3.0 范围**: 实现 Normal 模式，Extended/Mixed 作为配置选项预留

---

## 3. 技术决策总结

### 3.1 周期发送

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 定时器 | tokio::time::interval | 与现有异步架构一致 |
| 调度策略 | 优先队列 | 高效，支持大量消息 |
| 数据更新 | Channel 命令 | 清晰的命令模式 |
| 漂移处理 | MissedTickBehavior::Delay | 避免消息突发 |

### 3.2 ISO-TP

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 实现方式 | 自行实现 | 与后端抽象集成 |
| 状态机 | 枚举 + match | Rust 惯用模式 |
| 缓冲区 | Vec<u8>，最大 4095 | ISO-TP 标准限制 |
| 会话模式 | 单会话 | v0.3.0 简化实现 |
| 帧大小 | 自动检测 | 根据后端能力选择 |

### 3.3 Feature Flags

```toml
[features]
default = []
isotp = []  # ISO-TP 支持
periodic = ["tokio"]  # 周期发送（需要 tokio）
full = ["isotp", "periodic", "async", "tracing", "hot-reload"]
```

---

## 4. 参考资料

1. ISO 15765-2:2016 - Road vehicles — Diagnostic communication over Controller Area Network (DoCAN) — Part 2: Transport protocol and network layer services
2. [tokio::time 文档](https://docs.rs/tokio/latest/tokio/time/)
3. [CAN-FD 规范](https://www.bosch-semiconductors.com/media/ubk_semiconductors/pdf_1/canliteratur/can_fd_spec.pdf)
4. [isotp-rs crate](https://crates.io/crates/isotp-rs)

---

**研究版本**: 1.0.0
**完成日期**: 2026-01-12
