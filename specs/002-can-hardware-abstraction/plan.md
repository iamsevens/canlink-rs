# 实施计划: CAN 硬件抽象层

**分支**: `002-can-hardware-abstraction` | **日期**: 2026-01-08 | **规范**: [spec.md](spec.md)
**输入**: 来自 `/specs/002-can-hardware-abstraction/spec.md` 的功能规范

**注意**: 此模板由 `/speckit.plan` 命令填充. 执行工作流程请参见 `.specify/templates/commands/plan.md`.

## 摘要

本功能实现 CAN 硬件抽象层，提供统一的接口定义，使应用代码能够在不同硬件后端（TSMaster、PEAK、Kvaser、Mock 等）之间无缝切换。抽象层通过 Rust trait 定义硬件接口契约，支持运行时后端注册与发现、硬件能力查询、统一的消息和错误类型、生命周期管理、版本兼容性检查，以及通过 TOML 配置文件进行后端选择。核心设计原则包括：零成本抽象（性能开销 < 5%）、外部同步模型（高频操作无锁）、编译时静态链接优先、Mock 后端支持无硬件测试。

## 技术背景

**语言/版本**: Rust 2021 edition, MSRV 1.75+
**主要依赖**:
- `toml` - TOML 配置文件解析
- `thiserror` - 错误类型定义
- `semver` - 语义版本控制
- 异步支持（可选）: `tokio` 或 `async-std` 通过 feature flags 控制（见 research.md 决策）
**存储**: N/A（无持久化存储需求，配置通过 TOML 文件）
**测试**: cargo test（单元测试）、cargo test --test integration（集成测试）、文档测试
**目标平台**:
- Windows 10+ (主要平台，TSMaster SDK 支持)
- Linux (次要平台，未来其他硬件后端)
- 支持 `no_std` 环境（嵌入式系统，章程要求）
**项目类型**: 库项目（单一 workspace，多个 crate）
**性能目标**:
- 抽象层开销 < 5%（相比直接调用硬件 API）
- 能力查询响应时间 < 1ms
- 支持 1000 消息/秒吞吐量（取决于硬件）
**约束条件**:
- 零成本抽象原则
- 最小化内存分配（优先栈分配）
- 外部同步模型（常规操作无锁）
- 编译时类型安全
**规模/范围**:
- 核心抽象层 crate: ~2000 行代码
- Mock 后端实现: ~1000 行代码
- 文档和示例: ~500 行代码
- 预计支持 3-5 个硬件后端

## 章程检查

*门控: 必须在阶段 0 研究前通过. 阶段 1 设计后重新检查. *

### I. 库优先架构 ✅

- **符合**: 抽象层设计为独立的 crate (`canlink-hal`)
- **验证**:
  - 核心抽象层是自包含的库
  - Mock 后端作为独立 crate (`canlink-mock`)
  - 每个硬件后端（TSMaster、PEAK 等）都是独立 crate
  - 清晰的公共 API（trait 定义）
  - 明确的功能边界（抽象层 vs 具体实现）

### II. CLI 优先接口 ✅

- **符合**: CLI 工具将在 v0.1.0 中实现
- **验证**:
  - 创建 `canlink-cli` crate 提供命令行工具
  - 支持后端列表、能力查询、消息发送/接收、配置验证等功能
  - 遵循文本输入/输出协议（stdin/args → stdout, 错误 → stderr）
  - 支持 JSON 和人类可读格式输出
  - 通过集成测试验证与库的一致性

### III. 测试覆盖要求 ✅

- **符合**:
  - 单元测试覆盖所有公共 trait 方法
  - 集成测试验证 Mock 后端与抽象层交互
  - 文档测试确保示例代码可运行
  - 目标：Mock 后端测试覆盖率 90%+ (SC-003)

### IV. 硬件抽象与可移植性 ✅

- **符合**: 这正是本功能的核心目标
- **验证**:
  - 定义与硬件无关的 trait 接口 (FR-001)
  - 独立的硬件后端实现 crate (FR-012)
  - Mock 后端支持无硬件测试 (FR-005)
  - 核心协议逻辑与硬件层分离

### V. 协议正确性与安全性 ✅

- **符合**:
  - 使用 Rust 类型系统防止无效状态（统一消息类型、错误类型）
  - 编译时类型安全（trait 约束）
  - 版本兼容性检查 (FR-008)
  - 错误处理覆盖所有操作 (FR-006)

### 门控评估: ✅ 通过

所有章程原则均符合或有合理延迟计划。可以进入阶段 0 研究。

---

## 阶段 1 设计后重新评估

**日期**: 2026-01-08
**状态**: ✅ 所有章程检查通过

### I. 库优先架构 ✅

- **验证通过**:
  - `canlink-hal` crate 定义清晰（见 data-model.md）
  - `canlink-mock` crate 独立实现
  - 公共 API 通过 trait 定义（见 contracts/backend-trait.md）
  - 功能边界明确（抽象层 vs 后端实现）

### II. CLI 优先接口 ✅

- **验证通过**:
  - `canlink-cli` crate 将在 v0.1.0 实现
  - 提供后端列表、能力查询、消息发送/接收、配置验证等功能
  - 遵循文本输入/输出协议（stdin/args → stdout, 错误 → stderr）
  - 支持 JSON 和人类可读格式输出
  - 通过集成测试验证与库的一致性

### III. 测试覆盖要求 ✅

- **验证通过**:
  - contracts/backend-trait.md 定义了测试要求
  - Mock 后端支持单元测试和集成测试
  - 文档测试通过 quickstart.md 中的示例保证

### IV. 硬件抽象与可移植性 ✅

- **验证通过**:
  - trait 接口完全与硬件无关
  - 后端注册表支持运行时切换
  - Mock 后端实现完整
  - 配置文件支持（canlink.toml）

### V. 协议正确性与安全性 ✅

- **验证通过**:
  - 使用 Rust 类型系统（CanId enum, MessageFlags bitflags）
  - 编译时类型安全（trait 约束）
  - 完整的错误处理（CanError enum）
  - 版本兼容性检查（BackendVersion）

### 设计决策确认

**线程安全模型**（解决 CHK004/CHK033/CHK054）:
- ✅ 已在 research.md 中明确定义
- ✅ 已在 contracts/backend-trait.md 中文档化
- ✅ 分层策略：高频操作外部同步，低频操作内部同步

**异步支持**:
- ✅ 已在 research.md 中决策
- ✅ 采用可选 feature flag 方式
- ✅ 同步和异步 API 都已定义

### 最终评估: ✅ 设计完成，可以进入实施阶段

所有设计文档已完成：
- ✅ research.md - 技术决策
- ✅ data-model.md - 数据模型
- ✅ contracts/backend-trait.md - 核心 API
- ✅ contracts/backend-registry.md - 注册表 API
- ✅ quickstart.md - 使用指南

下一步：运行 `/speckit.tasks` 生成实施任务列表。

## 项目结构

### 文档(此功能)

```
specs/[###-feature]/
├── plan.md              # 此文件 (/speckit.plan 命令输出)
├── research.md          # 阶段 0 输出 (/speckit.plan 命令)
├── data-model.md        # 阶段 1 输出 (/speckit.plan 命令)
├── quickstart.md        # 阶段 1 输出 (/speckit.plan 命令)
├── contracts/           # 阶段 1 输出 (/speckit.plan 命令)
└── tasks.md             # 阶段 2 输出 (/speckit.tasks 命令 - 非 /speckit.plan 创建)
```

### 源代码(仓库根目录)

```
canlink-rs/                    # Workspace 根目录
├── Cargo.toml                 # Workspace 配置
├── canlink-hal/               # 核心硬件抽象层 crate
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs             # 公共 API 入口
│   │   ├── backend.rs         # Backend trait 定义
│   │   ├── message.rs         # 统一消息类型
│   │   ├── error.rs           # 统一错误类型
│   │   ├── capability.rs      # 硬件能力描述
│   │   ├── config.rs          # TOML 配置解析
│   │   ├── registry.rs        # 后端注册表
│   │   └── version.rs         # 版本兼容性检查
│   └── tests/
│       ├── integration/       # 集成测试
│       └── contract/          # 契约测试
│
├── canlink-mock/              # Mock 后端实现
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── backend.rs         # Mock Backend 实现
│   │   ├── recorder.rs        # 消息记录器
│   │   └── injector.rs        # 错误注入器
│   └── tests/
│
├── canlink-cli/               # CLI 工具 (v0.1.0)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs            # CLI 入口
│   │   ├── commands/          # 命令实现
│   │   │   ├── list.rs        # 列出后端
│   │   │   ├── info.rs        # 查询能力
│   │   │   ├── send.rs        # 发送消息
│   │   │   ├── receive.rs     # 接收消息
│   │   │   └── validate.rs    # 验证配置
│   │   └── output.rs          # 输出格式化（JSON/人类可读）
│   └── tests/
│
├── canlink-tsmaster/          # TSMaster 后端（依赖 001 规范）
│   └── [将在 001 规范实现后集成]
│
└── examples/                  # 示例代码
    ├── basic_usage.rs         # 基础使用示例
    ├── backend_switching.rs   # 后端切换示例
    ├── mock_testing.rs        # Mock 测试示例
    └── cli_usage.sh           # CLI 使用示例
```

**结构决策**:
- 选择单一 workspace 结构（选项 1）
- 每个 crate 是独立的库，符合章程 I
- `canlink-hal` 是核心抽象层，定义所有 trait
- `canlink-mock` 是第一个后端实现，用于验证接口设计
- `canlink-cli` 提供命令行工具，符合章程 II
- `canlink-tsmaster` 将在 001 规范完成后作为真实硬件后端
- 未来其他硬件后端（PEAK、Kvaser）将作为独立 crate 添加

## 复杂度跟踪

*仅在章程检查有必须证明的违规时填写*

**无章程违规**: 所有章程原则均已满足，无需复杂度跟踪。
