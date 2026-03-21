# 任务: CAN 硬件抽象层

**输入**: 来自 `/specs/002-can-hardware-abstraction/` 的设计文档
**前置条件**: plan.md、spec.md、research.md、data-model.md、contracts/

**测试**: 本规范要求 90% 测试覆盖率（SC-003），因此包含测试任务

**组织结构**: 任务按用户故事分组，以便每个故事能够独立实施和测试

## 格式: `[ID] [P] [Story] 描述`
- **[P]**: 可以并行运行（不同文件，无依赖关系）
- **[Story]**: 此任务属于哪个用户故事（例如: US1、US2、US3）
- 在描述中包含确切的文件路径

## 路径约定
- **Workspace 根目录**: `canlink-rs/`
- **核心抽象层**: `canlink-hal/src/`
- **Mock 后端**: `canlink-mock/src/`
- **示例代码**: `examples/`
- **测试**: `canlink-hal/tests/` 和 `canlink-mock/tests/`

---

## 阶段 1: 设置（共享基础设施）

**目的**: 项目初始化和基本结构

- [X] T001 创建 Rust workspace 结构（根目录 Cargo.toml）
- [X] T002 [P] 创建 canlink-hal crate 目录结构（src/, tests/, Cargo.toml）
- [X] T003 [P] 创建 canlink-mock crate 目录结构（src/, tests/, Cargo.toml）
- [X] T004 [P] 创建 canlink-cli crate 目录结构（src/, tests/, Cargo.toml）
- [X] T005 [P] 创建 examples 目录
- [X] T006 配置 workspace 依赖项（toml, thiserror, semver, bitflags, serde, clap）
- [X] T007 [P] 配置 rustfmt 和 clippy
- [X] T008 [P] 配置测试覆盖率工具（cargo-tarpaulin 或 cargo-llvm-cov）

---

## 阶段 2: 基础（阻塞前置条件）

**目的**: 在任何用户故事可以实施之前必须完成的核心基础设施

**⚠️ 关键**: 在此阶段完成之前，无法开始任何用户故事工作

### 核心数据类型（所有故事依赖）

- [X] T009 [P] 在 canlink-hal/src/message.rs 中实现 CanId enum（标准/扩展 ID）
- [X] T010 [P] 在 canlink-hal/src/message.rs 中实现 MessageFlags bitflags（RTR, FD, BRS, ESI）
- [X] T011 [P] 在 canlink-hal/src/message.rs 中实现 Timestamp 结构体
- [X] T012 在 canlink-hal/src/message.rs 中实现 CanMessage 结构体（依赖 T009-T011）
  - 实现基础字段：id (CanId), data (Vec<u8>), timestamp (Timestamp), flags (MessageFlags)
  - 实现 CAN-FD 特定字段：BRS (Bit Rate Switch) 和 ESI (Error State Indicator) 标志
  - 添加字段验证：数据长度检查（CAN 2.0: 0-8，CAN-FD: 0-64）
- [X] T013 在 canlink-hal/src/message.rs 中为 CanMessage 添加构造方法（new_standard, new_extended, new_fd, new_remote）

### 错误处理（所有故事依赖）

- [X] T014 [P] 在 canlink-hal/src/error.rs 中实现 BusErrorKind enum
- [X] T015 在 canlink-hal/src/error.rs 中实现 CanError enum（使用 thiserror）

### 硬件能力描述（US2 依赖，但作为基础）

- [X] T016 [P] 在 canlink-hal/src/capability.rs 中实现 TimestampPrecision enum
- [X] T017 在 canlink-hal/src/capability.rs 中实现 HardwareCapability 结构体

### 版本管理（所有后端依赖）

- [X] T018 [P] 在 canlink-hal/src/version.rs 中实现 BackendVersion 结构体和版本兼容性检查方法

### 配置管理（US1 依赖）

- [X] T019 [P] 在 canlink-hal/src/config.rs 中实现 BackendConfig 结构体（使用 serde）
- [X] T020 在 canlink-hal/src/config.rs 中实现 CanlinkConfig 结构体和 from_file 方法

### 生命周期状态（所有后端依赖）

- [X] T021 [P] 在 canlink-hal/src/state.rs 中实现 BackendState enum

### 库入口点

- [X] T022 在 canlink-hal/src/lib.rs 中导出所有公共 API（依赖 T008-T021）

**检查点**: 基础就绪 - 现在可以开始并行实施用户故事

---

## 阶段 3: 用户故事 1 - 统一的硬件接口（优先级: P1）🎯 MVP

**目标**: 定义统一的硬件接口，实现 Mock 后端，支持后端注册和发现

**独立测试**: 可以通过编写使用统一接口的测试代码，连接 Mock 硬件，验证消息收发功能正常工作

### 用户故事 1 的核心接口定义

- [X] T023 [US1] 在 canlink-hal/src/backend.rs 中定义 CanBackend trait（initialize, close, send_message, receive_message, open_channel, close_channel, get_capability, version, name）
- [X] T024 [US1] 在 canlink-hal/src/backend.rs 中定义 BackendFactory trait（create, name, version）
- [X] T025 [US1] 在 canlink-hal/src/backend.rs 中添加线程安全文档注释（外部同步要求）
- [X] T025a [US1] 在 canlink-hal/src/backend.rs 中实现初始化重试逻辑辅助函数（公共 API，供后端实现者使用，默认重试 3 次，间隔 1 秒，满足 FR-009）

### 用户故事 1 的后端注册表

- [X] T026 [US1] 在 canlink-hal/src/registry.rs 中实现 BackendRegistry 结构体（使用 RwLock<HashMap>）
- [X] T027 [US1] 在 canlink-hal/src/registry.rs 中实现 register 方法（检查重复、验证版本兼容性，满足 FR-008）
- [X] T028 [US1] 在 canlink-hal/src/registry.rs 中实现 unregister 方法
- [X] T029 [US1] 在 canlink-hal/src/registry.rs 中实现 create 方法（创建后端实例）
- [X] T030 [US1] 在 canlink-hal/src/registry.rs 中实现 list_backends 方法
- [X] T031 [US1] 在 canlink-hal/src/registry.rs 中实现 get_backend_info 方法
- [X] T032 [US1] 在 canlink-hal/src/registry.rs 中实现 is_registered 方法
- [X] T033 [US1] 在 canlink-hal/src/registry.rs 中实现 global 单例方法（使用 OnceLock）
- [X] T034 [US1] 在 canlink-hal/src/registry.rs 中定义 BackendInfo 结构体

**注**: FR-012 提到的动态库加载功能（可选高级功能）延迟到 v0.2.0 或更高版本实施。当前版本（v0.1.0）专注于编译时静态链接，这已满足核心需求并简化实现。

### 用户故事 1 的 Mock 后端实现

- [X] T035 [P] [US1] 在 canlink-mock/src/backend.rs 中实现 MockBackend 结构体
- [X] T036 [US1] 在 canlink-mock/src/backend.rs 中为 MockBackend 实现 CanBackend trait
- [X] T037 [US1] 在 canlink-mock/src/backend.rs 中实现 MockBackendFactory
- [X] T038 [P] [US1] 在 canlink-mock/src/recorder.rs 中实现消息记录器（记录发送的消息）
- [X] T039 [P] [US1] 在 canlink-mock/src/config.rs 中实现 Mock 后端配置结构
- [X] T040 [US1] 在 canlink-mock/src/lib.rs 中导出公共 API

### 用户故事 1 的测试

- [X] T041 [P] [US1] 在 canlink-hal/tests/backend_trait_test.rs 中编写 CanBackend trait 契约测试
- [X] T042 [P] [US1] 在 canlink-hal/tests/registry_test.rs 中编写 BackendRegistry 单元测试（注册、查询、创建）
- [X] T043 [P] [US1] 在 canlink-mock/tests/mock_backend_test.rs 中编写 Mock 后端测试（消息收发）
- [X] T044 [P] [US1] 在 canlink-hal/tests/integration/backend_switching_test.rs 中编写后端切换集成测试（包括状态迁移和清理验证）

### 用户故事 1 的示例代码

- [X] T045 [P] [US1] 在 examples/basic_usage.rs 中创建基础使用示例（注册后端、发送接收消息）
- [X] T046 [P] [US1] 在 examples/backend_switching.rs 中创建后端切换示例（从配置文件加载）

**检查点**: 此时，用户故事 1 应该完全功能化且可独立测试（统一接口、Mock 后端、后端注册）

---

## 阶段 4: 用户故事 2 - 硬件能力发现（优先级: P2）

**目标**: 实现硬件能力查询功能，支持运行时检测硬件特性

**独立测试**: 可以通过查询 Mock 后端的能力信息，验证返回的能力描述准确反映硬件特性

### 用户故事 2 的实施

- [X] T047 [US2] 在 canlink-mock/src/backend.rs 中为 MockBackend 实现 get_capability 方法（返回预定义能力）
- [X] T048 [US2] 在 canlink-mock/src/config.rs 中添加能力配置选项（可配置通道数、CAN-FD 支持等）
- [X] T049 [US2] 在 canlink-hal/src/backend.rs 中添加能力查询文档和使用示例

### 用户故事 2 的测试

- [X] T050 [P] [US2] 在 canlink-hal/tests/capability_test.rs 中编写能力查询测试（验证响应时间 < 1ms）
- [X] T051 [P] [US2] 在 canlink-mock/tests/capability_test.rs 中编写 Mock 后端能力配置测试
- [X] T052 [P] [US2] 在 canlink-hal/tests/integration/capability_adaptation_test.rs 中编写能力适配集成测试（根据能力调整行为）

### 用户故事 2 的示例代码

- [X] T053 [P] [US2] 在 examples/capability_query.rs 中创建能力查询示例（查询并显示硬件能力）
- [X] T054 [P] [US2] 在 examples/capability_adaptation.rs 中创建能力适配示例（根据能力选择消息类型）

**检查点**: 此时，用户故事 1 和 2 都应该独立运行（能力查询功能完整）

---

## 阶段 5: 用户故事 3 - 无硬件测试支持（优先级: P3）

**目标**: 增强 Mock 后端功能，支持预设消息、错误注入、行为验证

**独立测试**: 可以通过使用 Mock 后端运行完整的应用测试套件，验证所有功能在无硬件环境下正常工作

### 用户故事 3 的实施

- [X] T055 [P] [US3] 在 canlink-mock/src/injector.rs 中实现错误注入器（模拟总线错误、发送失败等）
- [X] T056 [US3] 在 canlink-mock/src/backend.rs 中集成错误注入功能
- [X] T057 [US3] 在 canlink-mock/src/config.rs 中添加预设消息配置（从配置文件加载预设消息）
- [X] T058 [US3] 在 canlink-mock/src/backend.rs 中实现预设消息队列（receive_message 返回预设消息）
- [X] T059 [US3] 在 canlink-mock/src/recorder.rs 中添加消息验证功能（验证发送的消息是否符合预期）

### 用户故事 3 的测试

- [X] T060 [P] [US3] 在 canlink-mock/tests/error_injection_test.rs 中编写错误注入测试
- [X] T061 [P] [US3] 在 canlink-mock/tests/preset_messages_test.rs 中编写预设消息测试
- [X] T062 [P] [US3] 在 canlink-mock/tests/message_verification_test.rs 中编写消息验证测试
- [X] T063 [P] [US3] 在 canlink-hal/tests/integration/mock_testing_test.rs 中编写完整的 Mock 测试场景

### 用户故事 3 的示例代码

- [X] T064 [P] [US3] 在 examples/mock_testing.rs 中创建 Mock 测试示例（预设消息、错误注入、验证）
- [X] T065 [P] [US3] 在 examples/automated_testing.rs 中创建自动化测试示例（使用 Mock 后端的测试套件）

**检查点**: 所有用户故事现在应该独立功能化（Mock 后端功能完整）

---

## 阶段 6: CLI 工具实现（FR-013）

**目的**: 实现命令行工具，满足章程原则 II

**优先级**: P1（章程要求）

### CLI 基础设施

- [X] T066a [P] 在 canlink-cli/src/main.rs 中创建 CLI 入口点（使用 clap）
- [X] T066b [P] 在 canlink-cli/src/output.rs 中实现输出格式化（JSON 和人类可读格式）
- [X] T066c [P] 在 canlink-cli/src/error.rs 中实现 CLI 错误处理（退出码、stderr 输出）

### CLI 命令实现

- [X] T066d [P] 在 canlink-cli/src/commands/list.rs 中实现 `canlink list` 命令（列出所有可用后端）
- [X] T066e [P] 在 canlink-cli/src/commands/info.rs 中实现 `canlink info <backend>` 命令（查询后端能力）
- [X] T066f [P] 在 canlink-cli/src/commands/send.rs 中实现 `canlink send` 命令（发送 CAN 消息）
- [X] T066g [P] 在 canlink-cli/src/commands/receive.rs 中实现 `canlink receive` 命令（接收 CAN 消息）
- [X] T066h [P] 在 canlink-cli/src/commands/validate.rs 中实现 `canlink validate` 命令（验证配置文件）

### CLI 测试

- [X] T066i [P] 在 canlink-cli/tests/integration_test.rs 中编写 CLI 集成测试
- [X] T066j [P] 在 examples/cli_usage.sh 中创建 CLI 使用示例脚本

### CLI 文档

- [X] T066k [P] 在 canlink-cli/ 中添加 README.md（CLI 使用说明）
- [X] T066l [P] 为 CLI 添加 --help 文档（通过 clap 自动生成）

---

## 阶段 7: 可选异步支持（研究决策实施）

**目的**: 实现可选的异步 API（通过 feature flag 控制）

**注意**: 此阶段是可选的，基于 research.md 中的决策

- [X] T067a [P] 在 canlink-hal/Cargo.toml 中配置 async feature flags（async, async-tokio, async-async-std）
- [X] T067b [P] 在 canlink-hal/src/backend.rs 中定义 CanBackendAsync trait（send_message_async, receive_message_async）
- [X] T067c [P] 在 canlink-mock/src/backend.rs 中为 MockBackend 实现 CanBackendAsync trait
- [X] T067d [P] 在 canlink-hal/tests/async_test.rs 中编写异步 API 测试（需要 async feature）
- [X] T067e [P] 在 examples/async_usage.rs 中创建异步使用示例

---

## 阶段 8: 完善与横切关注点

**目的**: 影响多个用户故事的改进

### 文档完善

- [X] T071 [P] 为 canlink-hal/src/lib.rs 添加 crate 级别文档（概述、快速开始）
- [X] T072 [P] 为所有公共 API 添加 rustdoc 文档注释（确保 100% 文档覆盖率）
- [X] T072a [P] 在 canlink-hal/src/backend.rs 中添加线程安全使用指南文档（多线程场景示例，满足风险 R-002 缓解措施）
- [X] T073 [P] 在 docs/ 中添加用户指南（项目介绍、安装、使用）
- [X] T074 [P] 在 canlink-mock/ 中添加 README.md（Mock 后端说明）

### 性能优化和验证

- [X] T075 [P] 在 canlink-hal/benches/performance_bench.rs 中创建性能基准测试（验证抽象层开销 < 5%）
- [X] T076 [P] 在 canlink-hal/benches/capability_bench.rs 中创建能力查询基准测试（验证响应时间 < 1ms）
- [X] T076a [P] 在 canlink-cli/benches/cli_bench.rs 中创建 CLI 命令响应时间基准测试（验证命令执行性能）
- [X] T077 优化消息收发路径（减少内存分配，使用栈分配）
- [ ] T077a [P] 使用 valgrind 或 miri 检测 FFI 边界的内存安全问题（满足风险 R-004 缓解措施）**[延迟到 v0.2.0: 当前无 FFI 代码，待 TSCan 后端集成后实施]**

### 测试覆盖率验证

- [X] T078 运行测试覆盖率工具，验证 Mock 后端覆盖率 >= 90%（SC-003）
- [X] T079 [P] 添加缺失的单元测试以达到覆盖率目标
- [X] T080 [P] 添加文档测试（确保所有示例代码可运行）

### 代码质量

- [X] T081 [P] 运行 clippy 并修复所有警告
- [X] T082 [P] 运行 rustfmt 格式化所有代码
- [X] T083 [P] 添加 CI 配置（GitHub Actions 或 GitLab CI）

### 验收标准验证

- [X] T084 验证 SC-001：开发者能够在 10 分钟内实现新后端（通过文档和示例）
- [X] T085 验证 SC-002：后端切换无需修改业务逻辑（通过集成测试）
- [X] T086 验证 SC-004：能力查询响应时间 < 1ms（通过基准测试）
- [X] T087 验证 SC-005：抽象层开销 < 5%（通过基准测试）
- [X] T088 验证 SC-006：错误处理代码 100% 可复用（通过代码审查和测试）
  - **验证方法**: 在 canlink-hal/tests/integration/error_handling_test.rs 中编写一个通用的错误处理函数，测试它能否处理 Mock 后端和 LibTSCAN 后端返回的所有错误类型
  - **成功标准**: 通用错误处理函数无需任何后端特定的分支逻辑，所有错误都通过统一的 CanError 类型处理
- [X] T089 验证 SC-007：文档完整性 100%（通过 rustdoc 检查）

### 最终验证

- [X] T090 运行 quickstart.md 中的所有示例，验证可用性
- [X] T091 创建发布检查清单（版本号、CHANGELOG、标签）

---

## 依赖关系与执行顺序

### 阶段依赖关系

- **设置（阶段 1）**: 无依赖关系 - 可立即开始
- **基础（阶段 2）**: 依赖于设置完成 - 阻塞所有用户故事
- **用户故事（阶段 3-5）**: 都依赖于基础阶段完成
  - 然后用户故事可以并行进行（如果有人员）
  - 或按优先级顺序进行（P1 → P2 → P3）
- **CLI 工具（阶段 6）**: 依赖于基础阶段和用户故事 1 完成（需要后端注册表）
- **异步支持（阶段 7）**: 可选，可在任何用户故事完成后进行
- **完善（阶段 8）**: 依赖于所有期望的用户故事和 CLI 完成

### 用户故事依赖关系

- **用户故事 1（P1）**: 可在基础（阶段 2）后开始 - 无其他故事依赖
  - 定义核心接口、实现 Mock 后端、后端注册表
- **用户故事 2（P2）**: 可在基础（阶段 2）后开始 - 独立于 US1，但通常在 US1 后实施
  - 增强能力查询功能
- **用户故事 3（P3）**: 依赖于 US1 完成（需要 Mock 后端基础）
  - 增强 Mock 后端功能（错误注入、预设消息）

### 每个用户故事内部

- 接口定义在实现之前
- 核心实现在测试之前（或 TDD 方式：测试先行）
- 示例代码在核心功能完成后
- 故事完成后才移至下一个优先级

### 并行机会

- 所有标记为 [P] 的设置任务可以并行运行（T002-T004, T006-T007）
- 所有标记为 [P] 的基础任务可以并行运行（T008-T010, T013, T015, T017, T019, T021）
- 基础阶段完成后，用户故事 1 和 2 可以并行开始（如果团队容量允许）
- 用户故事中所有标记为 [P] 的测试可以并行运行
- 用户故事中标记为 [P] 的实现任务可以并行运行（不同文件）
- 完善阶段的大部分任务可以并行运行

---

## 并行示例: 用户故事 1

```bash
# 一起启动用户故事 1 的核心数据类型（基础阶段）:
任务 T008: "在 canlink-hal/src/message.rs 中实现 CanId enum"
任务 T009: "在 canlink-hal/src/message.rs 中实现 MessageFlags bitflags"
任务 T010: "在 canlink-hal/src/message.rs 中实现 Timestamp 结构体"

# 一起启动用户故事 1 的 Mock 后端组件:
任务 T035: "在 canlink-mock/src/backend.rs 中实现 MockBackend 结构体"
任务 T038: "在 canlink-mock/src/recorder.rs 中实现消息记录器"
任务 T039: "在 canlink-mock/src/config.rs 中实现 Mock 后端配置结构"

# 一起启动用户故事 1 的所有测试:
任务 T041: "在 canlink-hal/tests/backend_trait_test.rs 中编写 CanBackend trait 契约测试"
任务 T042: "在 canlink-hal/tests/registry_test.rs 中编写 BackendRegistry 单元测试"
任务 T043: "在 canlink-mock/tests/mock_backend_test.rs 中编写 Mock 后端测试"
任务 T044: "在 canlink-hal/tests/integration/backend_switching_test.rs 中编写后端切换集成测试"
```

---

## 实施策略

### 仅 MVP（仅用户故事 1）

1. 完成阶段 1: 设置（T001-T007）
2. 完成阶段 2: 基础（T008-T022）- 关键，阻塞所有故事
3. 完成阶段 3: 用户故事 1（T023-T046）
4. **停止并验证**: 独立测试用户故事 1
   - 运行所有测试（cargo test）
   - 运行示例代码（cargo run --example basic_usage）
   - 验证后端切换功能
5. 如准备好则发布 v0.1.0（MVP）

### 增量交付

1. 完成设置 + 基础 → 基础就绪
2. 添加用户故事 1 → 独立测试 → 发布 v0.1.0（MVP - 统一接口 + Mock 后端）
3. 添加用户故事 2 → 独立测试 → 发布 v0.2.0（能力查询）
4. 添加用户故事 3 → 独立测试 → 发布 v0.3.0（完整 Mock 功能）
5. 添加异步支持 → 测试 → 发布 v0.4.0（可选异步 API）
6. 完善和优化 → 发布 v1.0.0（生产就绪）

### 并行团队策略

有多个开发人员时:

1. 团队一起完成设置 + 基础（T001-T022）
2. 基础完成后:
   - 开发人员 A: 用户故事 1 核心接口（T023-T034）
   - 开发人员 B: 用户故事 1 Mock 后端（T035-T040）
   - 开发人员 C: 用户故事 1 测试和示例（T041-T046）
3. US1 完成后:
   - 开发人员 A: 用户故事 2（T047-T054）
   - 开发人员 B: 用户故事 3（T055-T065）
   - 开发人员 C: 异步支持（T066-T070）
4. 最后一起完成完善阶段（T071-T091）

---

## 注意事项

- **[P] 任务** = 不同文件，无依赖关系，可并行执行
- **[Story] 标签** 将任务映射到特定用户故事以实现可追溯性
- **每个用户故事应该独立可完成和可测试**
- **测试优先**: 建议使用 TDD 方法，先编写测试再实现
- **在每个任务或逻辑组后提交**: 保持小而频繁的提交
- **在任何检查点停止以独立验证故事**: 确保每个故事完成后可独立运行
- **避免**: 模糊任务、相同文件冲突、破坏独立性的跨故事依赖
- **性能关键**: T008-T012（消息类型）和 T023（CanBackend trait）是性能关键路径，需要特别注意零成本抽象
- **文档关键**: 所有公共 API 必须有完整的 rustdoc 文档（SC-007 要求 100% 文档覆盖率）
- **测试覆盖率**: Mock 后端必须达到 90% 测试覆盖率（SC-003）

---

## 任务统计

- **总任务数**: 106
- **已完成任务**: 105
- **延迟任务**: 1（T077a - 延迟到 v0.2.0）
- **设置任务**: 7（T001-T007）✅
- **基础任务**: 15（T008-T022）✅
- **用户故事 1**: 24（T023-T046）✅
- **用户故事 2**: 8（T047-T054）✅
- **用户故事 3**: 11（T055-T065）✅
- **CLI 工具**: 12（T066a-T066l）✅
- **异步支持**: 5（T067a-T067e）✅
- **完善任务**: 24（T071-T091，包含新增的 T072a, T076a, T077a）

**v0.1.0 发布状态**: ✅ 已发布

**并行机会**: 约 40% 的任务标记为 [P]，可并行执行

**MVP 范围**: 阶段 1-3 + CLI（T001-T046 + T066a-T066l，共 58 个任务）

**实际工作量**: v0.1.0 已完成，包含所有用户故事和异步支持
