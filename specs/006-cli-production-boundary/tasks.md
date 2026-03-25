# 006: CLI 生产能力收口任务清单

**范围**: 位于 `/specs/006-cli-production-boundary/`

**输入**: `spec.md`、`plan.md`、`checklists/requirements.md`

**目标**: 收口 CLI 正式命令集合、解耦 mock 运行时依赖、文档与发布口径一致。

## 任务分组

| 任务 | 优先级 | 说明 |
|---|---|---|
| US1: CLI 正式命令收口 | P1 | 仅保留 `list/info/send/receive/validate` |
| US2: mock 解耦 | P1 | mock 仅用于测试/示例，不作为 CLI 正式依赖 |
| US3: 文档与发布口径一致 | P1 | README/发布文档/CI 对齐 4-crate 发布面 |

---

## 阶段 1: CLI 依赖与命令收口

- [ ] T001 [US1] 调整 `canlink-cli/Cargo.toml`，将 `canlink-mock` 降为 `dev-dependencies`（或移除）
- [ ] T002 [US1] 移除 `canlink-cli/src/main.rs` 中默认注册的 mock 后端
- [ ] T003 [US1] CLI 命令集合收口为 `list/info/send/receive/validate`
- [ ] T004 [US1] 从 `commands/mod.rs` 移除 `filter/monitor/isotp`
- [ ] T005 [US1] 清理 `commands/filter.rs`（或移至开发测试用途）
- [ ] T006 [US1] 清理 `commands/monitor.rs`（或移至开发测试用途）
- [ ] T007 [US1] 清理 `commands/isotp.rs`（或移至开发测试用途）
- [ ] T008 [US1] 确认 `list/info/send/receive/validate` 不依赖 mock

---

## 阶段 2: 测试与验证

- [ ] T009 [US1] 更新 `canlink-cli/tests/integration_test.rs`，只验证 5 个正式命令
- [ ] T010 [US1] 更新 CLI 帮助与错误提示（非法命令应明确提示）
- [ ] T011 [US2] 移除 `filter`/`monitor` 相关测试文件或改为内部测试
- [ ] T012 [US2] 确保 `cargo test -p canlink-cli` 在无 mock 运行时依赖下通过
- [ ] T013 [US2] 确保 `cargo test --workspace` 保持通过

---

## 阶段 3: 文档与发布口径

- [ ] T014 [US3] 更新 README 与 CLI 文档，明确正式命令集合
- [ ] T015 [US3] 更新发布文档/清单与 CI 说明，保持 4-crate 发布面一致

---

## 说明

- 任务编号与规范一致，便于追踪变更与验收。
- 本清单仅描述范围，不替代 `spec.md` 中的验收场景与约束。
