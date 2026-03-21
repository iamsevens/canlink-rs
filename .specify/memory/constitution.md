<!--
SYNCHRONIZATION IMPACT REPORT
Generated: 2026-01-08

VERSION CHANGE: [TEMPLATE] → 1.0.0
Increment Rationale: Initial constitution ratification for new project

MODIFIED PRINCIPLES:
- All principles newly defined (initial version)

ADDED SECTIONS:
- Core Principles (5 principles defined)
- Technical Standards
- Development Workflow
- Governance

REMOVED SECTIONS:
- None (initial version)

TEMPLATE UPDATES REQUIRED:
✅ .specify/templates/plan-template.md - Constitution check section verified
✅ .specify/templates/spec-template.md - Requirements alignment verified
✅ .specify/templates/tasks-template.md - Task categorization verified
✅ .specify/templates/checklist-template.md - Exists and compatible
✅ .claude/commands/*.md - Command files verified

DEFERRED PLACEHOLDERS:
- None - all placeholders filled

NEXT STEPS:
- Review and approve constitution
- Begin first feature specification
- Establish CI/CD pipeline per governance rules
-->

# CANLink-RS 项目章程

## 核心原则

### I. 库优先架构

每个功能必须从独立的库(crate)开始. 库必须:
- 自包含且可独立测试
- 具有明确的文档和用途说明
- 提供清晰的公共 API
- 不允许仅用于代码组织的库 - 每个库必须有明确的功能边界

**理由**: 模块化设计确保代码可重用性、可测试性和清晰的依赖关系. 在 Rust 生态系统中, crate 是天然的模块化单元, 强制执行编译时的依赖检查.

### II. CLI 优先接口

每个库必须通过命令行接口暴露其核心功能. CLI 必须:
- 遵循文本输入/输出协议: stdin/args → stdout, 错误 → stderr
- 支持 JSON 格式用于机器可读输出
- 支持人类可读格式用于调试和开发
- 提供 `--help` 和 `--version` 标准选项

**理由**: CLI 优先确保功能可脚本化、可测试和可组合. 文本协议使调试变得简单, 并支持与其他工具的集成. 这对于 CAN 总线诊断和监控工具尤为重要.

### III. 测试覆盖要求

所有公共 API 必须有测试覆盖. 测试要求:
- 单元测试: 覆盖所有公共函数和方法
- 集成测试: 验证库之间的交互
- 文档测试: 确保示例代码可运行
- 测试可以与实现并行开发, 但必须在合并前完成

**理由**: 测试确保代码质量和回归预防. Rust 的测试框架使测试成为一等公民. CAN 协议实现需要高可靠性, 测试是实现这一目标的关键.

### IV. 硬件抽象与可移植性

CAN 硬件访问必须通过抽象层实现. 要求:
- 定义与硬件无关的 trait 接口
- 为不同硬件后端提供独立的实现 crate
- 支持模拟后端用于测试
- 核心协议逻辑与硬件层分离

**理由**: 硬件抽象使代码可在不同平台和设备上运行, 支持无硬件的开发和测试, 并允许用户选择适合其需求的硬件后端.

### V. 协议正确性与安全性

CAN 协议实现必须严格遵循标准. 要求:
- CAN 2.0A/2.0B 和 CAN FD 标准合规性
- 使用 Rust 类型系统防止无效状态
- 边界检查和错误处理覆盖所有协议操作
- 文档必须引用相关协议规范章节

**理由**: CAN 总线用于安全关键系统(汽车、工业控制). 协议错误可能导致系统故障或安全问题. Rust 的类型系统和所有权模型是实现协议正确性的理想工具.

## 技术标准

### 语言与工具链

- **语言**: Rust (edition 2021 或更高)
- **最低支持 Rust 版本(MSRV)**: 在 Cargo.toml 中明确声明
- **代码格式**: 使用 `rustfmt` 默认配置
- **代码检查**: 使用 `clippy` 并解决所有警告
- **文档**: 所有公共 API 必须有 rustdoc 注释

### 依赖管理

- 优先使用成熟的 crates.io 依赖
- 避免过度依赖 - 每个依赖必须有明确理由
- 定期更新依赖以获取安全补丁
- 在 Cargo.toml 中记录依赖选择理由(通过注释)

### 性能与资源约束

- 支持 `no_std` 环境(嵌入式系统)
- 最小化内存分配 - 优先使用栈分配
- 零拷贝解析和序列化(在可能的情况下)
- 性能关键路径必须有基准测试

## 开发工作流程

### 分支策略

- `master` 分支始终保持可发布状态
- 功能开发在 `feature/###-name` 分支进行
- 使用 Pull Request 进行代码审查
- 合并前必须通过所有测试和检查

### 提交规范

- 使用约定式提交(Conventional Commits)格式
- 类型: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`
- 每个提交应该是原子性的和可回滚的
- 提交消息必须清晰描述变更内容

### 代码审查

- 所有代码必须经过审查才能合并
- 审查者验证:
  - 章程合规性
  - 测试覆盖
  - 文档完整性
  - API 设计合理性
  - 性能影响

### 发布流程

- 遵循语义版本控制(SemVer)
- MAJOR: 破坏性 API 变更
- MINOR: 向后兼容的功能添加
- PATCH: 向后兼容的错误修复
- 每个发布必须有 CHANGELOG 条目

## 治理

### 章程优先级

本章程优先于所有其他开发实践和决策. 任何与章程冲突的代码或实践必须:
1. 记录冲突原因
2. 获得明确批准
3. 制定迁移计划(如适用)

### 章程修正流程

修正本章程需要:
1. 提出修正提案并说明理由
2. 评估对现有代码的影响
3. 更新相关模板和文档
4. 递增版本号(遵循语义版本控制)
5. 记录修正历史

### 合规性验证

- 所有 Pull Request 必须验证章程合规性
- 使用 `.specify/templates/checklist-template.md` 进行审查
- 复杂性增加必须有明确证明
- 定期审查(每季度)以确保持续合规

### 运行时指导

开发过程中参考以下文档:
- `.specify/templates/spec-template.md` - 功能规范
- `.specify/templates/plan-template.md` - 实施计划
- `.specify/templates/tasks-template.md` - 任务分解
- `.specify/templates/checklist-template.md` - 审查清单

**版本**: 1.0.0 | **批准日期**: 2026-01-08 | **最后修正**: 2026-01-08
