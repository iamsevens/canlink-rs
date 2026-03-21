# 一致性分析报告: 004 周期性消息发送与 ISO-TP 支持

**生成日期**: 2026-01-12
**分析工具**: /speckit.analyze
**规范版本**: spec.md v1.2.0

---

## 摘要

| 类别 | 发现数 | 严重 | 中等 | 轻微 |
|------|--------|------|------|------|
| 文档不一致 | 3 | 0 | 2 | 1 |
| 覆盖缺口 | 0 | 0 | 0 | 0 |
| 歧义/重复 | 0 | 0 | 0 | 0 |
| **总计** | **3** | **0** | **2** | **1** |

**整体状态**: ⚠️ 需要同步更新

---

## 发现详情

### ISSUE-001: plan.md FR 编号与 spec.md v1.2.0 不一致 [中等]

**位置**: [plan.md:159-164](plan.md#L159-L164)

**描述**: plan.md 中的实施阶段引用了旧的 FR 编号，与 spec.md v1.2.0 不匹配。

**当前状态**:
```markdown
1. **阶段 1**: 周期性消息发送 (FR-001 到 FR-005)
2. **阶段 2**: ISO-TP 帧编解码 (FR-006)
3. **阶段 3**: ISO-TP 接收和自动 FC (FR-007 到 FR-009, FR-011)
4. **阶段 4**: ISO-TP 发送 (FR-010)
5. **阶段 5**: ISO-TP 高级配置 (FR-012 到 FR-015)
6. **阶段 6**: CLI 扩展 (FR-016 到 FR-018)
```

**应更新为**:
```markdown
1. **阶段 1**: 周期性消息发送 (FR-001 到 FR-006)
2. **阶段 2**: ISO-TP 帧编解码 (FR-007)
3. **阶段 3**: ISO-TP 接收和自动 FC (FR-008 到 FR-010, FR-012)
4. **阶段 4**: ISO-TP 发送 (FR-011, FR-017)
5. **阶段 5**: ISO-TP 高级配置 (FR-013 到 FR-016, FR-018, FR-019)
6. **阶段 6**: CLI 扩展 (FR-020 到 FR-022)
```

**影响**: 开发人员可能引用错误的需求编号

**建议修复**: 更新 plan.md 中的 FR 引用以匹配 spec.md v1.2.0

---

### ISSUE-002: data-model.md 缺少 IsoTpConfig.max_wait_count 字段 [中等]

**位置**: [data-model.md:382-409](data-model.md#L382-L409)

**描述**: spec.md v1.2.0 在 IsoTpConfig 中新增了 `max_wait_count` 字段（FR-017），但 data-model.md 未同步更新。

**spec.md 定义**:
```markdown
5. **IsoTpConfig**: ISO-TP 配置
   - ...
   - max_wait_count: u8 - FC(Wait) 最大等待次数（默认 10）
```

**data-model.md 缺失**: `max_wait_count` 字段未在 IsoTpConfig 结构体中定义

**影响**: 实现时可能遗漏此配置项

**建议修复**: 在 data-model.md 的 IsoTpConfig 中添加:
```rust
/// FC(Wait) 最大等待次数（默认 10）
pub max_wait_count: u8,
```

---

### ISSUE-003: data-model.md IsoTpError 缺少新增错误类型 [中等]

**位置**: [data-model.md:643-705](data-model.md#L643-L705)

**描述**: spec.md v1.2.0 定义了新的错误场景，但 data-model.md 的 IsoTpError 枚举未包含对应错误类型。

**缺失的错误类型**:

| 错误类型 | 对应场景 | 描述 |
|---------|---------|------|
| `TooManyWaits` | 场景 3.4, FR-017 | 连续 FC(Wait) 超过最大次数 |
| `BackendDisconnected` | 场景 1.6, 2.4 | 后端断开连接 |
| `BufferAllocationFailed` | 边界情况 | 缓冲区分配失败 |

**影响**: 实现时可能使用不准确的错误类型

**建议修复**: 在 IsoTpError 枚举中添加:
```rust
/// 连续 FC(Wait) 超过最大次数
#[error("Too many FC(Wait) responses: {count} exceeds max {max}")]
TooManyWaits { count: u8, max: u8 },

/// 后端断开连接
#[error("Backend disconnected")]
BackendDisconnected,

/// 缓冲区分配失败
#[error("Buffer allocation failed: requested {size} bytes")]
BufferAllocationFailed { size: usize },
```

---

### ISSUE-004: plan.md 与 tasks.md 阶段编号差异 [轻微]

**位置**: [plan.md:156-164](plan.md#L156-L164), [tasks.md:15-228](tasks.md#L15-L228)

**描述**: plan.md 使用 6 个阶段，tasks.md 使用 8 个阶段，编号不对应。

**对比**:

| plan.md | tasks.md |
|---------|----------|
| 阶段 1: 周期发送 | 阶段 1: 设置 |
| 阶段 2: 帧编解码 | 阶段 2: 基础 |
| 阶段 3: ISO-TP 接收 | 阶段 3: US1 周期发送 |
| 阶段 4: ISO-TP 发送 | 阶段 4: US2 ISO-TP 接收 |
| 阶段 5: 高级配置 | 阶段 5: US3 ISO-TP 发送 |
| 阶段 6: CLI | 阶段 6: 高级配置 |
| - | 阶段 7: CLI |
| - | 阶段 8: 完善 |

**影响**: 轻微混淆，但 tasks.md 的划分更细粒度且合理

**建议**: 保持 tasks.md 的阶段划分（更符合实际开发流程），在 plan.md 中添加说明指向 tasks.md 的详细阶段

---

## 覆盖度分析

### FR → Task 映射 ✅ 完整

所有 22 个功能需求 (FR-001 ~ FR-022) 均有对应任务覆盖。

| FR 范围 | 任务范围 | 状态 |
|---------|---------|------|
| FR-001 ~ FR-006 (周期发送) | T010-T019 | ✅ |
| FR-007 ~ FR-012 (ISO-TP 基础) | T020-T029 | ✅ |
| FR-013 ~ FR-016 (ISO-TP 高级) | T038-T042 | ✅ |
| FR-017 ~ FR-019 (错误处理) | T033a-T036a | ✅ |
| FR-020 ~ FR-022 (CLI) | T043-T048 | ✅ |

### 场景 → Task 映射 ✅ 完整

所有 17 个验收场景均有对应测试任务。

| 场景 | 测试任务 |
|------|---------|
| 1.1-1.4 | T010-T013 |
| 1.5 | T013b |
| 1.6 | T013c |
| 2.1-2.3 | T020-T023 |
| 2.4 | T024a |
| 2.5 | T024b |
| 3.1-3.3 | T030-T032 |
| 3.4 | T033a |
| 3.5 | T033b |
| 3.6 | T033c |

### 实体 → data-model.md 映射 ⚠️ 部分缺失

| 实体 | data-model.md | 状态 |
|------|--------------|------|
| PeriodicMessage | ✅ 定义 | ✅ |
| PeriodicScheduler | ✅ 定义 | ✅ |
| PeriodicStats | ✅ 定义 | ✅ |
| IsoTpFrame | ✅ 定义 | ✅ |
| IsoTpConfig | ⚠️ 缺少 max_wait_count | ⚠️ |
| IsoTpChannel | ✅ 定义 | ✅ |
| IsoTpState | ✅ 定义 | ✅ |
| IsoTpError | ⚠️ 缺少 3 个错误类型 | ⚠️ |
| FlowStatus | ✅ 定义 | ✅ |
| AddressingMode | ✅ 定义 | ✅ |
| FrameSize | ✅ 定义 | ✅ |

---

## 建议操作

### 优先级 1 (实现前必须修复)

1. **更新 data-model.md** - 添加缺失的 `max_wait_count` 字段和 3 个错误类型
   - 预计工作量: 10 分钟

### 优先级 2 (建议修复)

2. **更新 plan.md** - 同步 FR 编号引用
   - 预计工作量: 5 分钟

### 优先级 3 (可选)

3. **添加阶段映射说明** - 在 plan.md 中说明与 tasks.md 阶段的对应关系
   - 预计工作量: 5 分钟

---

## 结论

004 规范整体质量良好，所有功能需求和验收场景均有任务覆盖。发现的 3 个问题均为文档同步问题，不影响需求完整性。

**建议**: 在开始实现前，先完成优先级 1 的修复（更新 data-model.md），确保数据模型与规范一致。

---

**分析完成时间**: 2026-01-12
**下一步**: 修复发现的问题，然后开始阶段 1 实现

