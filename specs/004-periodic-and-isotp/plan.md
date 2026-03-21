# 实施计划: 周期性消息发送与 ISO-TP 支持

**分支**: `004-periodic-and-isotp` | **日期**: 2026-01-12 | **规范**: [spec.md](spec.md)
**输入**: 来自 `/specs/004-periodic-and-isotp/spec.md` 的功能规范
**目标版本**: v0.3.0

## 摘要

本计划实现 CANLink-RS v0.3.0 的两个核心功能：

1. **周期性消息发送** - 完成 001 规范中 FR-016 的遗留功能，支持按固定时间间隔（1ms-10000ms）自动发送 CAN 消息，使用 tokio 定时器实现精确调度。

2. **ISO-TP 基础支持** - 实现 ISO 15765-2 传输层协议，包括：
   - 帧编解码（SF/FF/CF/FC）
   - 自动 Flow Control 响应
   - 多帧消息重组和分段
   - 支持 CAN 2.0（8字节）和 CAN-FD（64字节）模式

技术方法：
- 使用 tokio 异步运行时进行周期调度
- 状态机模式管理 ISO-TP 会话
- 单会话模式（v0.3.0），多会话支持延后
- 通过 feature flag (`isotp`) 可选启用 ISO-TP

## 技术背景

**语言/版本**: Rust 1.75+ (edition 2021)
**主要依赖**: tokio (异步运行时和定时器), canlink-hal (后端抽象)
**存储**: N/A (内存缓冲区，最大 4095 字节)
**测试**: cargo test, criterion (性能基准)
**目标平台**: Windows, Linux, macOS
**项目类型**: 多 crate workspace (库优先)
**性能目标**:
- 周期发送精度: 95% 在 ±5% 误差内
- ISO-TP 吞吐量: ≥ 理论值 × 0.80
- ISO-TP 成功率: > 99.9%
**约束条件**:
- 周期间隔: 1ms - 10000ms
- ISO-TP 缓冲区: 最大 4095 字节
- 同时周期消息: ≥ 32 个
- 超时时间: 默认 1000ms
**规模/范围**: 单会话 ISO-TP，32+ 周期消息

## 章程检查

*门控: 必须在阶段 0 研究前通过. 阶段 1 设计后重新检查.*

### I. 库优先架构 ✅

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 功能作为独立库实现 | ✅ | 周期发送在 canlink-hal，ISO-TP 作为新模块 |
| 库可独立测试 | ✅ | 使用 MockBackend 测试，无需硬件 |
| 明确的公共 API | ✅ | PeriodicScheduler, IsoTpChannel 等 |
| 明确的功能边界 | ✅ | 周期发送和 ISO-TP 是独立功能模块 |

### II. CLI 优先接口 ✅

| 检查项 | 状态 | 说明 |
|--------|------|------|
| CLI 暴露核心功能 | ✅ | FR-016/017/018 定义 CLI 命令 |
| 支持 JSON 输出 | ✅ | 继承现有 CLI 框架 |
| 支持人类可读输出 | ✅ | 继承现有 CLI 框架 |
| 标准选项 | ✅ | --help, --version 已有 |

### III. 测试覆盖要求 ✅

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 单元测试 | ✅ | 所有公共 API 需测试 |
| 集成测试 | ✅ | ISO-TP 完整流程测试 |
| 文档测试 | ✅ | rustdoc 示例 |
| 覆盖率目标 | ✅ | SC-005: ≥ 90% |

### IV. 硬件抽象与可移植性 ✅

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 通过抽象层访问硬件 | ✅ | 使用 CanBackend trait |
| 支持模拟后端 | ✅ | MockBackend 可测试 ISO-TP |
| 核心逻辑与硬件分离 | ✅ | ISO-TP 状态机独立于后端 |

### V. 协议正确性与安全性 ✅

| 检查项 | 状态 | 说明 |
|--------|------|------|
| 标准合规性 | ✅ | ISO 15765-2:2016 |
| 类型系统防止无效状态 | ✅ | IsoTpFrame 枚举，FlowStatus 枚举 |
| 边界检查 | ✅ | 缓冲区大小限制，超时处理 |
| 文档引用规范 | ✅ | 帧格式文档化 |

**章程检查结果**: 全部通过，可进入阶段 0 研究

## 项目结构

### 文档(此功能)

```
specs/004-periodic-and-isotp/
├── spec.md              # 功能规范 (已完成)
├── plan.md              # 此文件
├── research.md          # 阶段 0 输出
├── data-model.md        # 阶段 1 输出
├── quickstart.md        # 阶段 1 输出
├── contracts/           # 阶段 1 输出
│   ├── periodic.rs      # 周期发送 API 契约
│   └── isotp.rs         # ISO-TP API 契约
└── tasks.md             # 阶段 2 输出
```

### 源代码(仓库根目录)

```
canlink-hal/
├── src/
│   ├── lib.rs
│   ├── backend.rs
│   ├── message.rs
│   ├── filter.rs          # v0.2.0
│   ├── queue.rs           # v0.2.0
│   ├── monitor.rs         # v0.2.0
│   ├── periodic/          # 新增: 周期发送
│   │   ├── mod.rs
│   │   ├── scheduler.rs   # PeriodicScheduler
│   │   ├── message.rs     # PeriodicMessage
│   │   └── stats.rs       # PeriodicStats
│   └── isotp/             # 新增: ISO-TP (feature gated)
│       ├── mod.rs
│       ├── frame.rs       # IsoTpFrame 编解码
│       ├── channel.rs     # IsoTpChannel
│       ├── config.rs      # IsoTpConfig
│       └── state.rs       # IsoTpState 状态机
└── tests/
    ├── periodic_tests.rs  # 周期发送测试
    └── isotp_tests.rs     # ISO-TP 测试

canlink-cli/
└── src/
    ├── commands/
    │   ├── send.rs        # 扩展: --periodic 选项
    │   └── isotp.rs       # 新增: isotp 子命令
    └── ...
```

**结构决策**:
- 周期发送作为 canlink-hal 的新模块 `periodic/`
- ISO-TP 作为 canlink-hal 的新模块 `isotp/`，通过 feature flag 启用
- CLI 扩展现有 send 命令并添加 isotp 子命令
- 遵循现有 workspace 结构，不新增 crate

## 复杂度跟踪

*无章程违规，无需证明*

## 实施阶段

基于规范定义的实施顺序：

1. **阶段 1**: 周期性消息发送 (FR-001 到 FR-006)
2. **阶段 2**: ISO-TP 帧编解码 (FR-007)
3. **阶段 3**: ISO-TP 接收和自动 FC (FR-008 到 FR-010, FR-012)
4. **阶段 4**: ISO-TP 发送 (FR-011, FR-017)
5. **阶段 5**: ISO-TP 高级配置 (FR-013 到 FR-016, FR-018, FR-019)
6. **阶段 6**: CLI 扩展 (FR-020 到 FR-022)

> **注**: 详细任务分解见 [tasks.md](tasks.md)，采用 8 阶段划分（设置→基础→US1→US2→US3→高级配置→CLI→完善）

## 风险缓解

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| 定时精度受 OS 调度影响 | 中 | 中 | 使用 tokio 高精度定时器，提供精度统计 |
| ISO-TP 兼容性差异 | 中 | 中 | 可配置参数，支持调整 FC 参数和超时 |
| 内存使用过高 | 低 | 中 | 限制周期消息数量和 ISO-TP 缓冲区大小 |

---

**计划版本**: 1.0.0
**创建日期**: 2026-01-12
**章程版本**: 1.0.0
