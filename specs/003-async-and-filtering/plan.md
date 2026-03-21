# 实施计划: 异步 API 与消息过滤

**分支**: `003-async-and-filtering` | **日期**: 2026-01-10 | **规范**: [spec.md](./spec.md)
**输入**: 来自 `/specs/003-async-and-filtering/spec.md` 的功能规范

## 摘要

本计划实现 CANLink-RS v0.2.0 的核心功能：消息过滤和系统健壮性改进。异步 API (US1) 已在 002 规范中完成，本计划聚焦于：

1. **消息过滤** (US2): MessageFilter trait、硬件/软件过滤、动态过滤器管理
2. **系统健壮性** (US3): 日志框架、队列管理、连接监控、配置热重载
3. **v0.1.0 延迟项**: 完成 8 项从 v0.1.0 延迟的功能

## 技术背景

**语言/版本**: Rust 1.75+ (edition 2021)
**主要依赖**:
- tokio 1.35 (异步运行时，已在 002 中集成)
- tracing (日志框架，新增)
- notify (文件监控，用于配置热重载)

**存储**: N/A (内存队列)
**测试**: cargo test, cargo-llvm-cov (覆盖率 ≥ 90%)
**目标平台**: Windows (x64), Linux (x64), macOS (x64, ARM64)
**项目类型**: Rust workspace (多 crate)
**性能目标**:
- 软件过滤延迟 < 10 μs/消息
- 异步 API 吞吐量 ≥ 同步 API × 0.95
**约束条件**:
- 默认队列大小 1000 条消息
- 内存使用波动 < 10%（1 小时运行）
**规模/范围**: 扩展现有 canlink-hal 和 canlink-mock crate

## 章程检查

*门控: 必须在阶段 0 研究前通过. 阶段 1 设计后重新检查.*

| 原则 | 状态 | 说明 |
|------|------|------|
| I. 库优先架构 | ✅ 通过 | 功能在 canlink-hal crate 中实现，通过 feature flags 控制 |
| II. CLI 优先接口 | ✅ 通过 | canlink-cli 已存在，将扩展支持过滤器命令 |
| III. 测试覆盖要求 | ✅ 通过 | SC-005 要求 ≥ 90% 覆盖率 |
| IV. 硬件抽象与可移植性 | ✅ 通过 | 过滤器通过 trait 抽象，支持硬件/软件实现 |
| V. 协议正确性与安全性 | ✅ 通过 | 使用 Rust 类型系统确保过滤器配置有效性 |

## 项目结构

### 文档(此功能)

```
specs/003-async-and-filtering/
├── plan.md              # 此文件
├── spec.md              # 功能规范
├── research.md          # 阶段 0 输出
├── data-model.md        # 阶段 1 输出
├── quickstart.md        # 阶段 1 输出
├── contracts/           # 阶段 1 输出
│   ├── filter-trait.md
│   ├── queue-policy.md
│   └── connection-monitor.md
└── tasks.md             # 阶段 2 输出
```

### 源代码(仓库根目录)

```
canlink-hal/
├── src/
│   ├── lib.rs           # 导出新模块
│   ├── backend.rs       # 已有，扩展过滤器支持和 switch_backend
│   ├── filter/          # 新增：过滤器模块
│   │   ├── mod.rs
│   │   ├── traits.rs    # MessageFilter trait
│   │   ├── id_filter.rs # IdFilter 实现（单 ID 和掩码过滤）
│   │   ├── range_filter.rs # RangeFilter 实现（ID 范围过滤）
│   │   ├── chain.rs     # FilterChain 过滤器链
│   │   └── config.rs    # FilterConfig
│   ├── queue/           # 新增：队列管理模块
│   │   ├── mod.rs
│   │   ├── policy.rs    # QueueOverflowPolicy
│   │   ├── bounded.rs   # BoundedQueue 实现
│   │   └── config.rs    # QueueConfig
│   ├── monitor/         # 新增：连接监控模块
│   │   ├── mod.rs
│   │   ├── state.rs     # ConnectionState 枚举
│   │   ├── reconnect.rs # ReconnectConfig
│   │   ├── connection.rs # ConnectionMonitor
│   │   └── config.rs    # MonitorConfig
│   ├── hot_reload.rs    # 配置热重载（feature = "hot-reload"）
│   ├── logging.rs       # 日志模块（feature = "tracing"）
│   └── resource.rs      # 资源管理文档
├── tests/
│   ├── id_filter_test.rs
│   ├── range_filter_test.rs
│   ├── chain_test.rs
│   ├── filter_integration_test.rs
│   ├── policy_test.rs
│   ├── bounded_test.rs
│   ├── connection_test.rs
│   └── robustness_test.rs
├── benches/
│   ├── filter_bench.rs
│   └── queue_bench.rs
└── Cargo.toml           # 添加 tracing, notify 依赖

canlink-mock/
├── src/
│   ├── backend.rs       # 扩展：支持过滤器和断开模拟
│   └── filter.rs        # 新增：MockFilter 实现
└── tests/
    └── filter_test.rs

canlink-cli/
├── src/
│   └── commands/
│       ├── filter.rs    # 新增：过滤器管理命令（待实现）
│       └── monitor.rs   # 新增：监控命令（待实现）
└── Cargo.toml

examples/
├── message_filtering.rs
├── filter_config.rs
├── connection_monitor.rs
├── queue_overflow.rs
└── hot_reload.rs
```

**结构决策**: 遵循现有 workspace 结构，在 canlink-hal 中添加新模块（filter/, queue/, monitor/），保持与 002 规范的一致性。

## 阶段 1 设计后重新评估

**日期**: 2026-01-10
**状态**: ✅ 所有章程检查通过

### I. 库优先架构 ✅

- **验证通过**:
  - 所有新功能在 `canlink-hal` crate 中实现
  - 通过 feature flags 控制可选功能（tracing, hot-reload）
  - 清晰的模块边界（filter/, queue/, monitor/）
  - 公共 API 通过 trait 定义（MessageFilter）

### II. CLI 优先接口 ✅

- **验证通过**:
  - `canlink-cli` 已存在，将扩展过滤器管理命令
  - 支持 JSON 和人类可读输出格式
  - 遵循文本输入/输出协议

### III. 测试覆盖要求 ✅

- **验证通过**:
  - SC-005 要求 ≥ 90% 测试覆盖率
  - contracts/ 目录定义了测试要求
  - 包含单元测试、集成测试、性能测试

### IV. 硬件抽象与可移植性 ✅

- **验证通过**:
  - MessageFilter trait 抽象过滤器接口
  - 支持硬件过滤器和软件过滤器
  - FilterChain 自动管理硬件过滤器回退
  - 配置文件支持跨平台

### V. 协议正确性与安全性 ✅

- **验证通过**:
  - 使用 Rust 类型系统（QueueOverflowPolicy enum）
  - 编译时类型安全（trait 约束）
  - 完整的错误处理（FilterError, QueueError, MonitorError）
  - 线程安全要求明确（Send + Sync）

### 设计决策确认

**消息过滤设计**:
- ✅ 已在 research.md 中决策：trait 对象 + 组合模式
- ✅ 已在 contracts/filter-trait.md 中定义 API
- ✅ 硬件过滤器优先，自动回退到软件过滤

**队列溢出策略**:
- ✅ 已在 research.md 中决策：用户可配置策略
- ✅ 已在 contracts/queue-policy.md 中定义 API
- ✅ 三种策略：DropOldest（默认）、DropNewest、Block

**连接监控**:
- ✅ 已在 research.md 中决策：心跳检测 + 可选自动重连
- ✅ 已在 contracts/connection-monitor.md 中定义 API
- ✅ 默认不自动重连，避免掩盖硬件问题

**日志框架**:
- ✅ 已在 research.md 中决策：使用 tracing
- ✅ 通过 feature flag 控制（可选）
- ✅ 与 tokio 生态系统兼容

### 最终评估: ✅ 设计完成，可以进入实施阶段

所有设计文档已完成：
- ✅ research.md - 技术决策（7 个研究主题）
- ✅ data-model.md - 数据模型（10 个核心实体）
- ✅ contracts/filter-trait.md - 过滤器 API
- ✅ contracts/queue-policy.md - 队列策略 API
- ✅ contracts/connection-monitor.md - 连接监控 API
- ✅ quickstart.md - 快速入门指南

下一步：运行 `/speckit.tasks` 生成实施任务列表。

## 复杂度跟踪

*仅在章程检查有必须证明的违规时填写*

**无章程违规**: 所有章程原则均已满足，无需复杂度跟踪。
