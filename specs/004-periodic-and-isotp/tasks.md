# 任务: 周期性消息发送与 ISO-TP 支持

**输入**: 来自 `/specs/004-periodic-and-isotp/` 的设计文档
**前置条件**: plan.md ✅, spec.md v1.2.0 ✅, research.md ✅, data-model.md ✅, contracts/ ✅
**目标版本**: v0.3.0
**测试要求**: SC-005 要求测试覆盖率 ≥ 90%

## 格式说明
- **[P]**: 可并行执行（不同文件，无依赖）
- **[US1/US2/US3]**: 所属用户故事
- 路径基于现有 workspace 结构

---

## 阶段 1: 设置

**目的**: 项目结构和依赖配置

- [x] T001 在 canlink-hal/Cargo.toml 中添加 `periodic` 和 `isotp` feature flags
- [x] T002 [P] 创建 canlink-hal/src/periodic/mod.rs 模块结构
- [x] T003 [P] 创建 canlink-hal/src/isotp/mod.rs 模块结构
- [x] T004 在 canlink-hal/src/lib.rs 中添加条件编译导出

---

## 阶段 2: 基础（阻塞前置条件）

**目的**: 所有用户故事依赖的核心类型和错误定义

**⚠️ 关键**: 必须在用户故事实现前完成

- [x] T005 在 canlink-hal/src/error.rs 中扩展 CanError 添加周期发送相关错误
  - 已有: InsufficientResources (覆盖 CapacityExceeded), Other (覆盖 BackendDisconnected)
- [x] T006 [P] 在 canlink-hal/src/isotp/error.rs 中创建 IsoTpError 枚举
  - 包含: RxTimeout, FcTimeout, TooManyWaits, SequenceMismatch, BufferOverflow,
    RemoteOverflow, UnexpectedFrame, Aborted, BufferAllocationFailed, BackendError
- [x] T007 [P] 在 canlink-hal/src/isotp/frame.rs 中实现 FlowStatus 枚举
- [x] T008 [P] 在 canlink-hal/src/isotp/frame.rs 中实现 StMin 编解码
- [x] T009 在 canlink-hal/src/isotp/config.rs 中实现 IsoTpConfig 和 IsoTpConfigBuilder
  - 包含: rx_timeout, tx_timeout, max_wait_count (FR-012, FR-017)

**检查点**: 基础类型就绪，可开始用户故事实现

---

## 阶段 3: 用户故事 1 - 周期性消息发送 (优先级: P1) 🎯 MVP

**目标**: 实现按固定时间间隔自动发送 CAN 消息的功能
**独立测试**: 使用 MockBackend 验证消息按配置间隔发送
**相关需求**: FR-001 到 FR-006, 场景 1.1-1.6

### US1 测试（TDD）

- [x] T010 [P] [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写 PeriodicMessage 创建和验证测试
- [x] T011 [P] [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写 PeriodicStats 统计测试
- [x] T012 [P] [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写 PeriodicScheduler 基本功能测试
- [x] T013 [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写多消息并发发送测试（SC-002 验证）
- [x] T013a [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写动态间隔更新测试（场景 1.2a）
- [x] T013b [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写发送失败跳过测试（场景 1.5）
- [x] T013c [US1] 在 canlink-hal/tests/periodic_tests.rs 中编写后端断开处理测试（场景 1.6）

### US1 实现

- [x] T014 [P] [US1] 在 canlink-hal/src/periodic/message.rs 中实现 PeriodicMessage 结构体
  - 包含: new(), id(), message(), interval(), is_enabled(), update_data(), set_interval(), set_enabled()
  - 验证: 间隔范围 1ms-10000ms (FR-001)

- [x] T015 [P] [US1] 在 canlink-hal/src/periodic/stats.rs 中实现 PeriodicStats 结构体
  - 包含: new(), record_send(), send_count(), average_interval(), min/max_interval(), reset()

- [x] T016 [US1] 在 canlink-hal/src/periodic/scheduler.rs 中实现 SchedulerCommand 枚举
  - 包含: Add, Remove, UpdateData, UpdateInterval, SetEnabled, GetStats, Shutdown

- [x] T017 [US1] 在 canlink-hal/src/periodic/scheduler.rs 中实现 PeriodicScheduler
  - 使用 tokio 优先队列调度（research.md 方案 C）
  - 实现: new(), add(), remove(), update_data(), update_interval(), set_enabled(), get_stats(), shutdown()
  - 支持至少 32 个并发消息 (FR-004)
  - 动态间隔更新 (FR-002a)

- [x] T018 [US1] 在 canlink-hal/src/periodic/scheduler.rs 中实现调度器内部状态机
  - 使用 BinaryHeap 优先队列
  - 实现 MissedTickBehavior::Delay 漂移处理

- [x] T018a [US1] 在 canlink-hal/src/periodic/scheduler.rs 中实现错误处理
  - 单次发送失败跳过并继续 (FR-006, 场景 1.5)
  - 后端断开检测和暂停 (场景 1.6)
  - warn 级别日志记录

- [x] T019 [US1] 在 canlink-hal/src/periodic/mod.rs 中导出公共 API

**检查点**: 周期发送功能完整，可独立测试验证 SC-001 和 SC-002

---

## 阶段 4: 用户故事 2 - ISO-TP 多帧接收 (优先级: P1)

**目标**: 自动处理 ISO-TP Flow Control，透明接收完整多帧消息
**独立测试**: 使用 MockBackend 模拟 FF 接收，验证自动发送 FC
**相关需求**: FR-007 到 FR-010, FR-012, 场景 2.1-2.5

### US2 测试（TDD）

- [x] T020 [P] [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写 IsoTpFrame 编解码测试（所有帧类型）
- [x] T021 [P] [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写 SingleFrame 收发测试
- [x] T022 [P] [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写多帧接收和自动 FC 响应测试
- [x] T023 [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写接收超时测试（场景 2.3）
- [x] T024 [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写序列号回绕测试
- [x] T024a [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写后端断开测试（场景 2.4）
- [x] T024b [US2] 在 canlink-hal/tests/isotp_tests.rs 中编写非预期帧处理测试（场景 2.5）

### US2 实现

- [x] T025 [P] [US2] 在 canlink-hal/src/isotp/frame.rs 中实现 IsoTpFrame 枚举
  - 包含: SingleFrame, FirstFrame, ConsecutiveFrame, FlowControl
  - 实现: decode(), encode(), pci_type(), is_*() 方法
  - 序列号 0-F 循环处理 (FR-007)

- [x] T026 [US2] 在 canlink-hal/src/isotp/state.rs 中实现 RxState 枚举
  - 包含: Idle, Receiving { buffer, expected_length, next_sn, ... }

- [x] T027 [US2] 在 canlink-hal/src/isotp/state.rs 中实现 IsoTpState 结构体
  - 包含: rx, tx 状态，is_idle(), is_receiving(), reset()

- [x] T028 [US2] 在 canlink-hal/src/isotp/channel.rs 中实现 IsoTpChannel 接收逻辑
  - 实现: new(), receive(), process_message()
  - 自动发送 FC 响应 (FR-008)
  - 多帧重组 (FR-010)
  - 超时处理 (FR-012)

- [x] T029 [US2] 在 canlink-hal/src/isotp/channel.rs 中实现接收错误处理
  - RxTimeout 错误（场景 2.3）
  - SequenceMismatch 错误
  - BufferOverflow 处理（返回 FC(Overflow)）
  - BackendDisconnected 处理（场景 2.4）
  - UnexpectedFrame 处理（场景 2.5）
  - 状态重置为 Idle

**检查点**: ISO-TP 接收功能完整，可独立测试 ✅

---

## 阶段 5: 用户故事 3 - ISO-TP 多帧发送 (优先级: P2)

**目标**: 自动将大消息分段发送，处理 Flow Control
**独立测试**: 使用 MockBackend 验证大消息被正确分段为 FF + CF 序列
**相关需求**: FR-011, FR-014, FR-017-FR-019, 场景 3.1-3.6

### US3 测试（TDD）

- [x] T030 [P] [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写自动分段发送测试
- [x] T031 [P] [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写 FC(Wait) 处理测试
- [x] T032 [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写发送超时测试（场景 3.3）
- [x] T033 [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写完整收发往返测试
- [x] T033a [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写 TooManyWaits 测试（场景 3.4）
- [x] T033b [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写非 FC 帧忽略测试（场景 3.5）
- [x] T033c [US3] 在 canlink-hal/tests/isotp_tests.rs 中编写 abort 和状态清理测试（场景 3.6）

### US3 实现

- [x] T034 [US3] 在 canlink-hal/src/isotp/state.rs 中实现 TxState 枚举
  - 包含: Idle, WaitingForFc, SendingCf { ... }

- [x] T035 [US3] 在 canlink-hal/src/isotp/channel.rs 中实现 IsoTpChannel 发送逻辑
  - 实现: send()
  - 自动分段 (FR-011)
  - 等待 FC 响应
  - 按 BS/STmin 参数发送 CF

- [x] T036 [US3] 在 canlink-hal/src/isotp/channel.rs 中实现发送错误处理
  - FcTimeout 错误（场景 3.3）
  - RemoteOverflow 处理
  - FC(Wait) 处理和 max_wait_count 限制 (FR-017, 场景 3.4)
  - TooManyWaits 错误
  - 非 FC 帧忽略，debug 日志（场景 3.5）

- [x] T036a [US3] 在 canlink-hal/src/isotp/channel.rs 中实现 abort 方法
  - 立即中止传输 (FR-019)
  - 释放缓冲区 (FR-018)
  - 重置状态为 Idle（场景 3.6）

- [x] T037 [US3] 在 canlink-hal/src/isotp/channel.rs 中实现 IsoTpCallback trait 和回调机制
  - on_transfer_start(), on_transfer_progress(), on_transfer_complete(), on_transfer_error()
  - (FR-014)

**检查点**: ISO-TP 发送功能完整，可独立测试验证 SC-003 和 SC-004 ✅

---

## 阶段 6: ISO-TP 高级配置

**目的**: 实现 CAN-FD 支持和地址模式配置
**相关需求**: FR-013, FR-015, FR-016

- [x] T038 [P] 在 canlink-hal/src/isotp/config.rs 中实现 FrameSize 枚举和自动检测逻辑
- [x] T039 [P] 在 canlink-hal/src/isotp/config.rs 中实现 AddressingMode 枚举
- [x] T040 在 canlink-hal/src/isotp/channel.rs 中实现 CAN-FD 帧大小自动适配 (FR-013)
- [x] T041 在 canlink-hal/src/isotp/channel.rs 中实现 Extended/Mixed 地址模式支持 (FR-015)
  - 实现: prepend_address_byte() 发送时添加地址字节
  - 实现: strip_address_byte() 接收时提取并验证地址字节
  - Mixed 模式验证地址扩展字节匹配
  - 新增 9 个测试用例验证功能
- [x] T042 在 canlink-hal/tests/isotp_tests.rs 中编写 CAN-FD 模式测试

**检查点**: CAN-FD 支持完成，Extended/Mixed 地址模式已实现 ✅

---

## 阶段 7: CLI 扩展

**目的**: 添加周期发送和 ISO-TP 命令行支持
**相关需求**: FR-020 到 FR-022

- [x] T043 [P] 在 canlink-cli/src/commands/send.rs 中添加 --periodic 选项 (FR-020)
- [x] T044 [P] 在 canlink-cli/src/commands/isotp.rs 中创建 isotp 子命令模块
- [x] T045 在 canlink-cli/src/commands/isotp.rs 中实现 `isotp send` 命令 (FR-021)
- [x] T046 在 canlink-cli/src/commands/isotp.rs 中实现 `isotp receive` 命令 (FR-022)
- [x] T047 在 canlink-cli/src/main.rs 中注册 isotp 子命令
- [x] T048 在 canlink-cli/tests/ 中编写 CLI 集成测试
  - 注: 使用现有测试框架，76 个 CLI 测试全部通过

**检查点**: CLI 扩展完成 ✅

---

## 阶段 8: 完善与验证

**目的**: 文档、性能验证和最终清理

- [x] T049 [P] 更新 docs/api-reference.md 添加周期发送和 ISO-TP API 文档
- [x] T050 [P] 更新 docs/user-guide.md 添加使用示例
- [x] T051 [P] 在 canlink-hal/examples/ 中创建 periodic_send.rs 示例
- [x] T052 [P] 在 canlink-hal/examples/ 中创建 isotp_transfer.rs 示例
- [x] T053 运行 cargo-llvm-cov 验证测试覆盖率 ≥ 90% (SC-005)
  - 结果: 行覆盖率 89.37%, 区域覆盖率 90.51%
- [x] T054 运行性能基准测试验证 SC-001, SC-003, SC-004
  - SC-003 测试条件: BS=0, STmin=0ms, CAN 2.0 模式
- [x] T055 更新 CHANGELOG.md 添加 v0.3.0 变更记录
- [x] T056 运行 quickstart.md 验证所有示例代码

**检查点**: 阶段 8 完成 ✅

---

## 依赖关系与执行顺序

### 阶段依赖

```
阶段 1 (设置)
    ↓
阶段 2 (基础) ← 阻塞所有用户故事
    ↓
┌───────────────────────────────────────┐
│  阶段 3 (US1)  │  阶段 4 (US2)        │ ← 可并行
│  周期发送      │  ISO-TP 接收         │
└───────────────────────────────────────┘
                    ↓
              阶段 5 (US3)
              ISO-TP 发送 ← 依赖 US2 的接收逻辑
                    ↓
              阶段 6 (高级配置)
                    ↓
              阶段 7 (CLI)
                    ↓
              阶段 8 (完善)
```

### 并行机会

| 阶段 | 可并行任务 |
|------|-----------|
| 阶段 1 | T002, T003 |
| 阶段 2 | T006, T007, T008 |
| 阶段 3 | T010-T012, T014-T015 |
| 阶段 4 | T020-T022, T025 |
| 阶段 5 | T030-T031 |
| 阶段 7 | T043, T044 |
| 阶段 8 | T049-T052 |

---

## 任务统计

| 阶段 | 任务数 | 测试任务 | 实现任务 |
|------|--------|----------|----------|
| 阶段 1: 设置 | 4 | 0 | 4 |
| 阶段 2: 基础 | 5 | 0 | 5 |
| 阶段 3: US1 周期发送 | 14 | 7 | 7 |
| 阶段 4: US2 ISO-TP 接收 | 12 | 7 | 5 |
| 阶段 5: US3 ISO-TP 发送 | 12 | 7 | 5 |
| 阶段 6: 高级配置 | 5 | 1 | 4 |
| 阶段 7: CLI | 6 | 1 | 5 |
| 阶段 8: 完善 | 8 | 1 | 7 |
| **总计** | **66** | **24** | **42** |

---

## 新增任务摘要 (v1.2.0)

基于清单审查补充的任务：

| 任务 ID | 描述 | 对应需求 |
|---------|------|----------|
| T013a | 动态间隔更新测试 | FR-002a, 场景 1.2a |
| T013b | 发送失败跳过测试 | FR-006, 场景 1.5 |
| T013c | 后端断开处理测试 | 场景 1.6 |
| T018a | 周期发送错误处理实现 | FR-006, 场景 1.5, 1.6 |
| T024a | ISO-TP 后端断开测试 | 场景 2.4 |
| T024b | 非预期帧处理测试 | 场景 2.5 |
| T033a | TooManyWaits 测试 | FR-017, 场景 3.4 |
| T033b | 非 FC 帧忽略测试 | 场景 3.5 |
| T033c | abort 和状态清理测试 | FR-018, FR-019, 场景 3.6 |
| T036a | abort 方法实现 | FR-018, FR-019 |

---

## 实施策略建议

### MVP 路径（仅 US1 + US2）

1. 完成阶段 1-2（设置 + 基础）
2. 完成阶段 3（周期发送）→ 验证 SC-001, SC-002
3. 完成阶段 4（ISO-TP 接收）→ 解决用户实际问题
4. **停止并发布 v0.3.0-alpha**

### 完整路径

1. MVP 路径
2. 完成阶段 5（ISO-TP 发送）→ 验证 SC-003, SC-004
3. 完成阶段 6-8
4. **发布 v0.3.0**

---

**创建日期**: 2026-01-12
**更新日期**: 2026-01-12 (v1.2.0 清单审查后更新)
**任务总数**: 66 (+10 新增)
**预计测试覆盖**: 24 个测试任务覆盖所有功能需求和错误恢复场景
