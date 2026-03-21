# 规范 004: 周期性消息发送与 ISO-TP 支持

**功能分支**: `004-periodic-and-isotp`
**创建时间**: 2026-01-12
**状态**: 草稿
**优先级**: P1
**版本目标**: v0.3.0

---

## 概述

本规范定义了 CANLink-RS v0.3.0 的两个核心功能：
1. **周期性消息发送** - 完成 001 规范中 FR-016 的遗留功能
2. **ISO-TP 基础支持** - 实现 ISO 15765-2 传输层协议，支持多帧消息的自动分段和重组

### 目标

1. 实现周期性消息发送功能，支持按固定时间间隔自动发送消息
2. 实现 ISO-TP 协议的基础支持，包括帧编解码和自动 Flow Control 响应
3. 解决多帧接收时需要手动发送触发帧的问题

### 背景

在使用 TSMaster 进行多帧消息接收时，发现接收完第一帧后无法自动接收后续帧，需要发送 Flow Control 帧才能继续接收。这是 ISO-TP 协议的标准行为，本规范将实现自动 FC 响应机制。

---

## 用户故事

### US1: 周期性消息发送 (优先级: P1)

**作为** 需要定期发送心跳或状态消息的工程师
**我希望** 配置消息按固定时间间隔自动发送
**以便** 我不需要手动管理发送定时器

#### 优先级原因
周期性发送是 001 规范中定义的 FR-016，是基础 CAN 功能的重要组成部分，广泛用于心跳、状态广播等场景。

#### 独立测试
可以使用 MockBackend 测试周期性发送逻辑，验证消息按配置的间隔发送。

#### 验收场景

**场景 1.1**: 基本周期发送
```
Given 已初始化的 CAN 后端
When 我配置消息 ID=0x123 以 100ms 间隔周期发送
Then 消息应该每 100ms 自动发送一次
And 发送间隔误差 < 5%（即 95-105ms 范围内）
```

**场景 1.2**: 动态数据更新
```
Given 正在周期发送的消息
When 我更新消息的数据内容
Then 下一次发送应该使用新的数据
And 不中断发送周期
```

**场景 1.2a**: 动态间隔更新
```
Given 正在周期发送的消息（当前间隔 100ms）
When 我更新发送间隔为 200ms
Then 下一次发送应该使用新的间隔
And 不中断当前发送周期（当前周期完成后生效）
```

**场景 1.3**: 停止周期发送
```
Given 正在周期发送的消息
When 我停止该消息的周期发送
Then 消息应该立即停止发送
And 资源应该被正确释放
```

**场景 1.4**: 多消息周期发送
```
Given 已初始化的 CAN 后端
When 我配置多个消息以不同间隔周期发送
Then 每个消息应该按各自的间隔独立发送
And 互不干扰
```

**场景 1.5**: 周期发送失败处理
```
Given 正在周期发送的消息
When 单次发送失败（如后端暂时不可用）
Then 系统应该跳过本次发送
And 记录警告日志（warn 级别）
And 继续下一个周期的发送
And 不影响其他周期消息
```

**场景 1.6**: 后端断开恢复
```
Given 正在周期发送的消息
When 后端断开连接
Then 系统应该暂停所有周期发送
And 返回 BackendDisconnected 错误（通过回调或状态查询）
When 后端重新连接
Then 用户需要手动重新启动周期发送
```

---

### US2: ISO-TP 多帧接收 (优先级: P1)

**作为** 需要接收大于 8 字节数据的工程师
**我希望** 系统自动处理 ISO-TP 协议的 Flow Control
**以便** 我可以透明地接收完整的多帧消息

#### 优先级原因
这是用户实际遇到的问题：多帧接收时需要手动发送 FC 帧。自动 FC 响应是使用 ISO-TP 的基本需求。

#### 独立测试
可以使用 MockBackend 模拟 First Frame 接收，验证系统自动发送 Flow Control 帧。

#### 验收场景

**场景 2.1**: 自动 Flow Control 响应
```
Given 已启用 ISO-TP 自动响应的后端
When 接收到 First Frame (FF)
Then 系统应该自动发送 Flow Control (FC) 帧
And FC 帧包含正确的 FS/BS/STmin 参数
```

**场景 2.2**: 完整多帧接收
```
Given 已启用 ISO-TP 的后端
When 发送方发送一个 100 字节的消息
Then 系统应该接收 FF + 多个 CF
And 自动重组为完整消息
And 返回完整的 100 字节数据
```

**场景 2.3**: 接收超时处理
```
Given 正在接收多帧消息
When 连续帧超时未到达 (默认 1000ms)
Then 系统应该中止接收
And 返回 RxTimeout 错误
And 释放接收缓冲区
And 重置状态为 Idle
```

**场景 2.4**: 接收中后端断开
```
Given 正在接收多帧消息
When 后端断开连接
Then 系统应该中止接收
And 返回 BackendDisconnected 错误
And 释放接收缓冲区
And 重置状态为 Idle
```

**场景 2.5**: 接收中收到非预期帧
```
Given 正在接收多帧消息（等待 CF）
When 收到非预期帧类型（如 SF 或新的 FF）
Then 系统应该中止当前接收
And 返回 UnexpectedFrame 错误
And 如果是新的 FF，返回 FC(Overflow) 表示忙
```

---

### US3: ISO-TP 多帧发送 (优先级: P2)

**作为** 需要发送大于 8 字节数据的工程师
**我希望** 系统自动将大消息分段发送
**以便** 我可以透明地发送任意长度的数据

#### 优先级原因
多帧发送是 ISO-TP 的另一半功能，与多帧接收配合使用，但优先级略低于接收（用户当前问题是接收）。

#### 独立测试
可以使用 MockBackend 验证大消息被正确分段为 FF + CF 序列。

#### 验收场景

**场景 3.1**: 自动分段发送
```
Given 已启用 ISO-TP 的后端
When 我发送一个 100 字节的消息
Then 系统应该自动分段为 FF + CF 序列
And 等待接收方的 FC 帧
And 按 FC 指定的参数发送 CF
```

**场景 3.2**: Flow Control 等待
```
Given 正在发送多帧消息
When 接收到 FC (FS=Wait)
Then 系统应该暂停发送
And 等待下一个 FC 帧
```

**场景 3.3**: 发送超时处理
```
Given 正在等待 FC 帧
When FC 超时未到达 (默认 1000ms)
Then 系统应该中止发送
And 返回 FcTimeout 错误
And 重置状态为 Idle
```

**场景 3.4**: FC(Wait) 处理与限制
```
Given 正在发送多帧消息
When 连续收到 FC(Wait) 超过最大次数（默认 10 次）
Then 系统应该中止发送
And 返回 TooManyWaits 错误
And 重置状态为 Idle
```

**场景 3.5**: 发送中收到非预期帧
```
Given 正在发送多帧消息（等待 FC）
When 收到非 FC 帧类型
Then 系统应该忽略该帧
And 继续等待 FC
And 记录调试日志（debug 级别）
```

**场景 3.6**: 传输中止与状态清理
```
Given 正在进行 ISO-TP 传输（发送或接收）
When 用户调用 abort() 方法
Then 系统应该立即中止传输
And 释放所有缓冲区
And 重置状态为 Idle
And 返回 Aborted 错误（如果有等待的操作）
```

---

### 边界情况

- **周期间隔为 0 或负数**: 返回 `InvalidParameter` 错误，拒绝无效配置
- **大量周期发送消息**: 系统支持至少 32 个同时周期发送（FR-004），超出限制返回 `CapacityExceeded` 错误
- **ISO-TP 接收缓冲区满**: v0.3.0 为单会话模式，收到新 First Frame 时返回 FC(Overflow)
- **格式错误的 ISO-TP 帧**: 静默丢弃，记录警告日志（warn 级别，格式: `"Invalid ISO-TP frame: {reason}"`），继续等待有效帧
- **CAN-FD 模式下 ISO-TP**: 自动检测后端能力，CAN 2.0 使用 8 字节/帧，CAN-FD 使用最大 64 字节/帧
- **FC 帧指定 BS=0**: 按 ISO-TP 标准处理，表示无块大小限制，连续发送所有 CF
- **STmin 微秒级间隔**: 支持 ISO-TP 标准的 STmin 编码（0x00-0x7F 为 ms，0xF1-0xF9 为 100-900μs）
- **ISO-TP 缓冲区分配失败**: 返回 `BufferAllocationFailed` 错误，不发送 FC
- **CAN 总线错误（Bus-Off）**: 由后端层处理，ISO-TP 层收到 `BackendError` 后中止传输并重置状态

---

## 功能需求

### 周期性消息发送 (FR-001 到 FR-006)

- **FR-001**: 系统必须支持配置消息的周期发送间隔（最小 1ms，最大 10000ms）
- **FR-002**: 系统必须支持动态更新周期发送消息的数据内容
- **FR-002a**: 系统必须支持动态更新周期发送消息的发送间隔
- **FR-003**: 系统必须支持启动和停止单个消息的周期发送
- **FR-004**: 系统必须支持同时周期发送多个消息（至少 32 个）
- **FR-005**: 系统必须提供周期发送的统计信息（发送次数、实际间隔）
- **FR-006**: 系统必须在单次发送失败时跳过并继续下一周期（不重试）

### ISO-TP 协议支持 (FR-007 到 FR-019)

- **FR-007**: 系统必须支持 ISO-TP 帧类型的编解码：
  - Single Frame (SF): PCI = 0x0X
  - First Frame (FF): PCI = 0x1X XX
  - Consecutive Frame (CF): PCI = 0x2X（序列号 0-F 循环）
  - Flow Control (FC): PCI = 0x3X
- **FR-008**: 系统必须支持自动发送 Flow Control 帧响应 First Frame
- **FR-009**: 系统必须支持配置 FC 参数：
  - FS (Flow Status): CTS/Wait/Overflow
  - BS (Block Size): 0-255
  - STmin (Separation Time): 0-127ms 或 100-900μs
- **FR-010**: 系统必须支持多帧消息的自动重组
- **FR-011**: 系统必须支持多帧消息的自动分段
- **FR-012**: 系统必须支持配置接收超时（rx_timeout）和发送超时（tx_timeout），默认均为 1000ms
- **FR-013**: 系统必须支持 CAN 2.0 和 CAN-FD 两种模式的 ISO-TP
- **FR-014**: 系统必须提供 ISO-TP 传输的状态回调（开始、进行中、完成、错误）
- **FR-015**: 系统必须支持配置 ISO-TP 地址模式（Normal/Extended/Mixed）
- **FR-016**: ISO-TP 功能必须通过 feature flag 可选启用
- **FR-017**: 系统必须支持配置 FC(Wait) 最大等待次数（默认 10 次）
- **FR-018**: 系统必须在传输中止后正确清理状态和释放缓冲区
- **FR-019**: 系统必须支持手动中止正在进行的传输（abort 方法）

### CLI 扩展 (FR-020 到 FR-022)

- **FR-020**: CLI 必须支持周期发送命令 `canlink send --periodic <interval_ms>`
- **FR-021**: CLI 必须支持 ISO-TP 发送命令 `canlink isotp send <data>`
- **FR-022**: CLI 必须支持 ISO-TP 接收命令 `canlink isotp receive`

---

## 关键实体

### 周期发送相关

1. **PeriodicMessage**: 周期发送消息配置
   - message: CanMessage - 要发送的消息
   - interval: Duration - 发送间隔
   - enabled: bool - 是否启用
   - stats: PeriodicStats - 统计信息

2. **PeriodicScheduler**: 周期发送调度器
   - 管理多个 PeriodicMessage
   - 使用 tokio 定时器实现精确调度

3. **PeriodicStats**: 周期发送统计
   - send_count: u64 - 发送次数
   - last_send_time: Instant - 上次发送时间
   - average_interval: Duration - 平均实际间隔

### ISO-TP 相关

4. **IsoTpFrame**: ISO-TP 帧类型枚举
   - SingleFrame { data_length, data }
   - FirstFrame { total_length, data }
   - ConsecutiveFrame { sequence_number, data }
   - FlowControl { flow_status, block_size, st_min }

5. **IsoTpConfig**: ISO-TP 配置
   - tx_id: u32 - 发送 CAN ID
   - rx_id: u32 - 接收 CAN ID
   - block_size: u8 - FC 的 BS 参数
   - st_min: StMin - FC 的 STmin 参数
   - rx_timeout: Duration - 接收超时时间（默认 1000ms）
   - tx_timeout: Duration - 发送超时时间（默认 1000ms）
   - max_wait_count: u8 - FC(Wait) 最大等待次数（默认 10）
   - addressing_mode: AddressingMode - 地址模式
   - max_buffer_size: usize - 最大缓冲区大小（默认 4095 字节）
   - frame_size: FrameSize - 帧大小模式（Auto/Classic8/FD64）

6. **IsoTpChannel**: ISO-TP 通道
   - 管理单个 ISO-TP 会话
   - 处理帧的编解码和状态机

7. **IsoTpState**: ISO-TP 状态机
   - Idle - 空闲
   - Receiving { buffer, expected_length, next_sn } - 接收中
   - Sending { buffer, offset, waiting_fc } - 发送中

8. **FlowStatus**: Flow Control 状态枚举
   - ContinueToSend (0x00)
   - Wait (0x01)
   - Overflow (0x02)

9. **AddressingMode**: 地址模式枚举
   - Normal - 标准地址（11位/29位 CAN ID）
   - Extended - 扩展地址（额外 1 字节地址）
   - Mixed - 混合地址

10. **FrameSize**: 帧大小模式枚举
    - Auto - 自动检测（根据后端 CAN-FD 能力选择）
    - Classic8 - 强制使用 CAN 2.0 模式（8 字节/帧）
    - FD64 - 强制使用 CAN-FD 模式（最大 64 字节/帧）

---

## 成功标准

### SC-001: 周期发送精度
- **指标**: 周期发送间隔误差 < 5%
- **测量方法**: 配置 100ms 间隔，测量 100 次发送的实际间隔
- **成功标准**: 95% 的发送间隔在 95-105ms 范围内

### SC-002: 周期发送容量
- **指标**: 支持至少 32 个消息同时周期发送
- **测量方法**: 配置 32 个不同间隔的周期消息，验证全部正常发送
- **成功标准**: 所有消息按配置间隔发送，无遗漏

### SC-003: ISO-TP 吞吐量
- **指标**: ISO-TP 传输速率达到理论最大值的 80%
- **测量方法**: 发送 4095 字节数据，测量传输时间
- **测试条件**: BS=0（无块大小限制），STmin=0ms，CAN 2.0 模式（8字节/帧）
- **理论值计算**: 4095 字节需要 1 FF + 585 CF = 586 帧，理论时间 = 586 × 帧间隔
- **成功标准**: 传输速率 ≥ 理论值 × 0.80

### SC-004: ISO-TP 可靠性
- **指标**: ISO-TP 传输成功率 > 99.9%
- **测量方法**: 执行 1000 次多帧传输，统计成功次数
- **成功标准**: 成功率 ≥ 99.9%

### SC-005: 测试覆盖率
- **指标**: 新功能测试覆盖率 ≥ 90%
- **测量方法**: 使用 cargo-llvm-cov
- **成功标准**: 覆盖率 ≥ 90%

---

## 非功能性需求

### NFR-001: 内存约束
- **周期发送内存**: 每个 PeriodicMessage 约 64 字节 + 消息数据（最大 8/64 字节）
- **周期发送总内存**: 32 个消息 × 80 字节 ≈ 2.5 KB 上限
- **ISO-TP 缓冲区**: 动态分配，按需增长，最大 4095 字节
- **ISO-TP 总内存**: 单会话模式下约 4.5 KB 上限（含状态和元数据）

### NFR-002: 超时精度
- **超时检测精度**: 依赖 tokio 运行时，典型精度 1-10ms
- **可接受误差**: 超时触发时间在配置值的 ±10% 范围内

---

## 假设

1. 目标平台支持 Rust 1.75+
2. 异步运行时使用 tokio（与 003 保持一致），periodic feature 依赖 tokio
3. ISO-TP 实现遵循 ISO 15765-2:2016 标准
4. CAN-FD 模式下 ISO-TP 最大数据长度为 64 字节/帧
5. 默认 FC 参数：BS=0（无限制），STmin=10ms
6. 默认超时时间：rx_timeout=1000ms（接收），tx_timeout=1000ms（等待 FC）
7. 默认 FC(Wait) 最大等待次数：10 次

---

## 范围外

以下功能不在 v0.3.0 范围内：

1. UDS (ISO 14229) 诊断协议 - 将在后续版本实现
2. OBD-II 协议支持
3. ISO-TP 的完整错误恢复机制（如重传）
4. **多路复用 ISO-TP 通道** - v0.3.0 为单会话模式，多会话支持计划在后续版本
5. ISO-TP 的安全扩展

---

## 依赖关系

### 内部依赖
- 002-can-hardware-abstraction: 基础后端接口
- 003-async-and-filtering: 异步 API 和队列管理

### 外部依赖
- tokio: 异步运行时和定时器
- 无新增外部依赖

---

## 风险与缓解措施

### R-001: 定时精度
- **风险**: 操作系统调度可能影响周期发送精度
- **可能性**: 中
- **影响**: 中
- **缓解**: 使用 tokio 的高精度定时器，提供精度统计供用户监控

### R-002: ISO-TP 兼容性
- **风险**: 不同 ECU 的 ISO-TP 实现可能有差异
- **可能性**: 中
- **影响**: 中
- **缓解**: 提供可配置的参数，支持调整 FC 参数和超时时间

### R-003: 内存使用
- **风险**: 大量周期消息或大 ISO-TP 缓冲区可能消耗过多内存
- **可能性**: 低
- **影响**: 中
- **缓解**: 限制最大周期消息数量，限制 ISO-TP 缓冲区大小

---

## 架构说明

### 周期发送架构

```
┌─────────────────────────────────────────────┐
│              Application                     │
│  periodic_scheduler.add(msg, 100ms)         │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│          PeriodicScheduler                   │
│  ┌─────────────────────────────────────┐    │
│  │ tokio::time::interval(100ms)        │    │
│  │ → send_message()                    │    │
│  └─────────────────────────────────────┘    │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│             CanBackend                       │
└─────────────────────────────────────────────┘
```

### ISO-TP 架构

```
┌─────────────────────────────────────────────┐
│              Application                     │
│  isotp.send(large_data)                     │
│  isotp.receive() -> large_data              │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│            IsoTpChannel                      │
│  ┌─────────────────────────────────────┐    │
│  │ State Machine:                      │    │
│  │ - Encode SF/FF/CF                   │    │
│  │ - Decode SF/FF/CF/FC                │    │
│  │ - Auto FC response                  │    │
│  │ - Reassembly buffer                 │    │
│  └─────────────────────────────────────┘    │
└─────────────────┬───────────────────────────┘
                  │
┌─────────────────▼───────────────────────────┐
│             CanBackend                       │
│  send_message() / receive_message()         │
└─────────────────────────────────────────────┘
```

### ISO-TP 帧格式

```
Single Frame (SF):
┌────────┬────────────────────────────────────┐
│ 0x0N   │ Data (N bytes, N ≤ 7)              │
└────────┴────────────────────────────────────┘

First Frame (FF):
┌────────┬────────┬───────────────────────────┐
│ 0x1L   │ LL     │ Data (6 bytes)            │
└────────┴────────┴───────────────────────────┘
  L = high nibble of length, LL = low byte

Consecutive Frame (CF):
┌────────┬────────────────────────────────────┐
│ 0x2N   │ Data (7 bytes)                     │
└────────┴────────────────────────────────────┘
  N = sequence number (0-F, wraps)

Flow Control (FC):
┌────────┬────────┬────────┬──────────────────┐
│ 0x3S   │ BS     │ STmin  │ Padding          │
└────────┴────────┴────────┴──────────────────┘
  S = Flow Status (0=CTS, 1=Wait, 2=Overflow)
  BS = Block Size (0 = no limit)
  STmin = Separation Time minimum
```

---

## 实施顺序

1. **阶段 1**: 周期性消息发送 (FR-001 到 FR-006)
2. **阶段 2**: ISO-TP 帧编解码 (FR-007)
3. **阶段 3**: ISO-TP 接收和自动 FC (FR-008 到 FR-010, FR-012)
4. **阶段 4**: ISO-TP 发送 (FR-011, FR-017)
5. **阶段 5**: ISO-TP 高级配置 (FR-013 到 FR-016, FR-018, FR-019)
6. **阶段 6**: CLI 扩展 (FR-020 到 FR-022)

---

## Clarifications

### Session 2026-01-12

- Q: ISO-TP 最大缓冲区大小？ → A: 4095 字节（ISO-TP 标准最大值，完整兼容）
- Q: 无效周期间隔（0 或负数）如何处理？ → A: 返回 InvalidParameter 错误
- Q: 格式错误的 ISO-TP 帧如何处理？ → A: 静默丢弃，记录警告日志（warn 级别），继续等待
- Q: CAN-FD 模式下 ISO-TP 帧大小？ → A: 自动检测（根据后端 CAN-FD 能力选择 8 或 64 字节）
- Q: 同时接收多个 ISO-TP 会话？ → A: v0.3.0 为单会话模式，新 FF 返回 FC(Overflow)；多会话支持记录到后续 ROADMAP

### Session 2026-01-12 (Checklist Review)

- Q: 周期发送失败如何处理？ → A: 跳过本次发送，记录 warn 日志，继续下一周期（场景 1.5）
- Q: 后端断开时周期发送如何恢复？ → A: 暂停发送，返回错误，需用户手动重启（场景 1.6）
- Q: FC(Wait) 最大等待次数？ → A: 默认 10 次，可配置（FR-017，场景 3.4）
- Q: 传输中止后状态如何清理？ → A: 释放缓冲区，重置为 Idle（场景 3.6，FR-018）
- Q: 发送间隔误差标准？ → A: 统一为相对误差 < 5%（场景 1.1，SC-001）
- Q: 接收中收到非预期帧？ → A: 中止当前接收，返回 UnexpectedFrame 错误（场景 2.5）
- Q: 发送中收到非 FC 帧？ → A: 忽略，继续等待 FC，记录 debug 日志（场景 3.5）
- Q: ISO-TP 缓冲区分配策略？ → A: 动态分配，按需增长（NFR-001）
- Q: 吞吐量测试条件？ → A: BS=0, STmin=0ms, CAN 2.0 模式（SC-003）

---

**文档版本**: 1.2.0
**创建日期**: 2026-01-12
**最后更新**: 2026-01-12
**澄清会话**: 2026-01-12 (5 questions), 2026-01-12 Checklist Review (9 questions)
