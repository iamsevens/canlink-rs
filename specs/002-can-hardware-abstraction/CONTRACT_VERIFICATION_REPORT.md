# 契约文档一致性验证报告

**日期**: 2026-01-10
**验证范围**: contracts/backend-trait.md 与 FR-014, FR-015 的一致性

---

## 验证 1: backend-trait.md 与 FR-014 的一致性

### FR-014 要求回顾

**FR-014**: 系统必须为 CanBackend trait 的核心方法定义明确的并发行为和资源管理需求

- **并发行为**: send_message 和 receive_message 方法要求外部同步，不保证内部线程安全
- **资源清理**: close 方法必须释放所有资源，包括：关闭所有打开的通道、清空消息队列、释放硬件连接
- **未发送消息处理**: close 时丢弃所有未发送的消息，不保证消息传递
- **幂等性**: close 方法必须是幂等的，多次调用不产生错误

### 一致性检查

#### ✅ 并发行为 (已覆盖)

**backend-trait.md 第 16-19 行**:
```rust
/// # 线程安全
///
/// 此 trait 的方法要求外部同步。如果需要从多个线程访问同一个后端实例，
/// 调用者必须使用 `Mutex` 或 `RwLock` 提供同步保护。
```

**backend-trait.md 第 265-268 行**:
```
### 线程安全
- 后端实例必须实现 `Send` trait
- 方法使用 `&mut self`，要求外部同步
- 不允许在方法内部使用锁（性能考虑）
```

**结论**: ✅ **完全一致** - 明确说明了外部同步要求

---

#### ⚠️ 资源清理 (部分覆盖，需补充)

**backend-trait.md 第 58-75 行**:
```rust
/// 关闭后端，释放资源
///
/// # 返回
/// - `Ok(())`: 关闭成功
/// - `Err(CanError)`: 关闭失败（但资源仍会尽力释放）
///
/// # 前置条件
/// - 后端处于 `Running` 状态
///
/// # 后置条件
/// - 后端处于 `Closed` 状态
/// - 所有资源已释放
```

**backend-trait.md 第 270-272 行**:
```
### 资源管理
- `close()` 必须释放所有资源，即使发生错误
- 实现 `Drop` trait 以确保资源清理
```

**缺失内容**:
1. ❌ 未明确列出需要释放的资源类型（通道、消息队列、硬件连接）
2. ❌ 未说明未发送消息的处理策略（丢弃）

**建议**: 需要补充详细的资源清理规范

---

#### ❌ 幂等性 (未覆盖)

**当前状态**: backend-trait.md 中没有提到 close 方法的幂等性要求

**缺失内容**:
- ❌ 未说明 close 方法可以多次调用
- ❌ 未说明重复调用不应产生错误

**建议**: 需要添加幂等性要求

---

### 总体评估

| 需求项 | 覆盖状态 | 说明 |
|--------|---------|------|
| 并发行为 | ✅ 完全覆盖 | 明确说明外部同步要求 |
| 资源清理 | ⚠️ 部分覆盖 | 缺少详细的资源类型列表 |
| 未发送消息处理 | ❌ 未覆盖 | 需要添加 |
| 幂等性 | ❌ 未覆盖 | 需要添加 |

**一致性评分**: 50% (2/4 项完全覆盖)

---

## 建议的改进

### 改进 1: 补充 close 方法的详细说明

在 backend-trait.md 第 58-75 行的 close 方法文档中添加：

```rust
/// 关闭后端，释放资源
///
/// # 资源清理
///
/// 此方法必须释放以下资源：
/// - 关闭所有打开的 CAN 通道
/// - 清空消息发送和接收队列
/// - 释放硬件连接和驱动资源
/// - 释放内存缓冲区
///
/// # 未发送消息处理
///
/// 调用 close 时，所有未发送的消息将被丢弃。
/// 不保证消息传递完成。如需确保消息发送，
/// 应在调用 close 前等待发送完成。
///
/// # 幂等性
///
/// 此方法是幂等的，可以安全地多次调用。
/// 重复调用不会产生错误，也不会有副作用。
///
/// # 返回
/// - `Ok(())`: 关闭成功
/// - `Err(CanError)`: 关闭失败（但资源仍会尽力释放）
///
/// # 前置条件
/// - 无（可以在任何状态下调用）
///
/// # 后置条件
/// - 后端处于 `Closed` 状态
/// - 所有资源已释放
/// - 后续调用 close 不会有任何效果
///
/// # 示例
/// ```rust
/// backend.close()?;
/// // 可以安全地再次调用
/// backend.close()?; // 不会产生错误
/// ```
fn close(&mut self) -> Result<(), CanError>;
```

### 改进 2: 更新资源管理章节

在 backend-trait.md 第 270-272 行更新为：

```markdown
### 资源管理

- `close()` 必须释放所有资源，即使发生错误
- 必须释放的资源包括：
  - 所有打开的 CAN 通道
  - 消息发送和接收队列
  - 硬件连接和驱动资源
  - 内存缓冲区
- 未发送的消息在 close 时被丢弃，不保证传递
- `close()` 方法必须是幂等的，可以安全地多次调用
- 实现 `Drop` trait 以确保资源清理（调用 close）
```

---

## 验证 2: backend-registry.md 与 FR-015 的一致性

### FR-015 要求回顾

**FR-015**: 系统必须为 BackendRegistry 定义线程安全和注册行为需求

- **线程安全**: BackendRegistry::register() 和 list_backends() 必须是线程安全的，支持并发调用
- **重复注册**: 重复注册同名后端时，返回 CanError::BackendAlreadyRegistered 错误，不覆盖已有注册
- **注册顺序**: list_backends() 返回的后端列表按注册顺序排序
- **注册失败**: 注册失败时不影响已注册的其他后端

### 一致性检查

#### ✅ 线程安全 (已覆盖)

**backend-registry.md 第 33-36 行**:
```rust
/// # 线程安全
///
/// `BackendRegistry` 的所有方法都是线程安全的，可以从多个线程同时调用。
/// 内部使用 `RwLock` 保护共享状态。
```

**backend-registry.md 第 100-101 行**:
```rust
/// # 线程安全
/// 此方法是线程安全的，可以从多个线程同时调用。
```

**backend-registry.md 第 298-303 行**:
```markdown
## 线程安全保证

- ✅ 所有方法都是线程安全的
- ✅ 可以从多个线程同时注册和创建后端
- ✅ 使用 `RwLock` 优化读多写少的场景
- ✅ 工厂必须实现 `Send + Sync`
```

**结论**: ✅ **完全一致** - 明确说明了线程安全保证

---

#### ⚠️ 重复注册 (部分一致，错误类型不匹配)

**backend-registry.md 第 97-98 行**:
```rust
/// # 错误
/// - `CanError::Other`: 后端名称已存在
```

**backend-registry.md 第 111-115 行**:
```rust
if factories.contains_key(&name) {
    return Err(CanError::Other(format!(
        "Backend '{}' is already registered",
        name
    )));
}
```

**问题**:
- ❌ 使用 `CanError::Other` 而非 `CanError::BackendAlreadyRegistered`
- ✅ 行为正确：不覆盖已有注册

**建议**: 更新错误类型为 `CanError::BackendAlreadyRegistered`

---

#### ❌ 注册顺序 (未覆盖)

**backend-registry.md 第 186-189 行**:
```rust
pub fn list_backends(&self) -> Vec<String> {
    let factories = self.factories.read().unwrap();
    factories.keys().cloned().collect()
}
```

**问题**:
- ❌ 使用 `HashMap`，不保证顺序
- ❌ 未说明返回列表的排序规则

**FR-015 要求**: 按注册顺序排序

**建议**:
1. 将 `HashMap` 改为 `IndexMap` 或添加注册顺序跟踪
2. 在文档中明确说明排序规则

---

#### ✅ 注册失败隔离 (已覆盖)

**backend-registry.md 第 111-119 行**:
```rust
if factories.contains_key(&name) {
    return Err(CanError::Other(format!(
        "Backend '{}' is already registered",
        name
    )));
}

factories.insert(name, factory);
Ok(())
```

**结论**: ✅ **符合要求** - 注册失败时提前返回，不影响其他后端

---

### 总体评估

| 需求项 | 覆盖状态 | 说明 |
|--------|---------|------|
| 线程安全 | ✅ 完全覆盖 | 明确说明并使用 RwLock 实现 |
| 重复注册 | ⚠️ 部分一致 | 行为正确，但错误类型不匹配 |
| 注册顺序 | ❌ 未覆盖 | HashMap 不保证顺序 |
| 注册失败隔离 | ✅ 完全覆盖 | 提前返回，不影响其他后端 |

**一致性评分**: 62.5% (2.5/4 项完全覆盖)

---

## 建议的改进

### 改进 3: 修复重复注册的错误类型

在 backend-registry.md 第 97-98 行和第 111-115 行更新为：

```rust
/// # 错误
/// - `CanError::BackendAlreadyRegistered`: 后端名称已存在

// ...

if factories.contains_key(&name) {
    return Err(CanError::BackendAlreadyRegistered(name));
}
```

### 改进 4: 实现注册顺序保证

**方案 1: 使用 IndexMap**

```rust
use indexmap::IndexMap;

pub struct BackendRegistry {
    factories: RwLock<IndexMap<String, Box<dyn BackendFactory>>>,
}
```

**方案 2: 添加注册顺序跟踪**

```rust
pub struct BackendRegistry {
    factories: RwLock<HashMap<String, Box<dyn BackendFactory>>>,
    registration_order: RwLock<Vec<String>>,
}

pub fn list_backends(&self) -> Vec<String> {
    let order = self.registration_order.read().unwrap();
    order.clone()
}
```

**推荐**: 方案 1（使用 IndexMap）更简洁

### 改进 5: 更新文档说明

在 backend-registry.md 第 174-189 行更新为：

```rust
/// 列出所有已注册的后端
///
/// # 返回
/// 后端名称列表，按注册顺序排序
///
/// # 排序规则
/// 返回的列表按照后端注册的时间顺序排列，
/// 先注册的后端排在前面。
///
/// # 示例
/// ```rust
/// let backends = registry.list_backends();
/// for name in backends {
///     println!("Available: {}", name);
/// }
/// ```
pub fn list_backends(&self) -> Vec<String> {
    let factories = self.factories.read().unwrap();
    // IndexMap 保证按插入顺序迭代
    factories.keys().cloned().collect()
}
```

---

## 总体验证结果

### 一致性评分汇总

| 契约文档 | 对应需求 | 一致性评分 | 状态 |
|---------|---------|-----------|------|
| backend-trait.md | FR-014 | 50% (2/4) | ⚠️ 需要改进 |
| backend-registry.md | FR-015 | 62.5% (2.5/4) | ⚠️ 需要改进 |
| **总体** | **FR-014, FR-015** | **56.25%** | **⚠️ 需要改进** |

### 关键发现

#### 高优先级问题 (P0)

1. **backend-trait.md 缺少幂等性要求** (FR-014)
   - 影响：实现可能不支持多次调用 close
   - 风险：资源泄漏或错误

2. **backend-trait.md 缺少详细资源清理规范** (FR-014)
   - 影响：实现可能遗漏某些资源
   - 风险：资源泄漏

3. **backend-registry.md 使用错误的错误类型** (FR-015)
   - 影响：错误处理代码不一致
   - 风险：错误处理逻辑混乱

4. **backend-registry.md 不保证注册顺序** (FR-015)
   - 影响：list_backends() 返回顺序不确定
   - 风险：用户体验不一致

#### 中优先级问题 (P1)

5. **backend-trait.md 缺少未发送消息处理说明** (FR-014)
   - 影响：行为不明确
   - 风险：用户期望不一致

---

## 行动计划

### 阶段 1: 更新契约文档 (立即)

#### 任务 1.1: 更新 backend-trait.md

**文件**: `specs/002-can-hardware-abstraction/contracts/backend-trait.md`

**更改**:
1. 补充 close 方法的详细资源清理说明（第 58-75 行）
2. 添加未发送消息处理策略
3. 添加幂等性要求
4. 更新资源管理章节（第 270-272 行）

**预计时间**: 15 分钟

---

#### 任务 1.2: 更新 backend-registry.md

**文件**: `specs/002-can-hardware-abstraction/contracts/backend-registry.md`

**更改**:
1. 修复重复注册的错误类型（第 97-98 行，第 111-115 行）
2. 添加注册顺序说明（第 174-189 行）
3. 更新错误处理表格（第 313-316 行）

**预计时间**: 10 分钟

---

### 阶段 2: 验证实现代码 (后续)

#### 任务 2.1: 检查 CanBackend 实现

**文件**:
- `canlink-hal/src/backend.rs`
- `canlink-mock/src/lib.rs`
- `canlink-tscan/src/lib.rs`

**验证项**:
- [ ] close 方法是否幂等
- [ ] close 方法是否释放所有资源
- [ ] close 方法是否丢弃未发送消息

**预计时间**: 30 分钟

---

#### 任务 2.2: 检查 BackendRegistry 实现

**文件**: `canlink-hal/src/registry.rs`

**验证项**:
- [ ] 是否使用 IndexMap 或其他顺序保证机制
- [ ] 重复注册是否返回 BackendAlreadyRegistered
- [ ] 是否线程安全

**预计时间**: 20 分钟

---

### 阶段 3: 添加测试 (后续)

#### 任务 3.1: 添加幂等性测试

```rust
#[test]
fn test_close_idempotent() {
    let mut backend = create_backend();
    backend.initialize(&config).unwrap();

    // 第一次关闭
    backend.close().unwrap();

    // 第二次关闭应该成功
    backend.close().unwrap();

    // 第三次关闭也应该成功
    backend.close().unwrap();
}
```

---

#### 任务 3.2: 添加注册顺序测试

```rust
#[test]
fn test_registration_order() {
    let registry = BackendRegistry::new();

    registry.register(Box::new(BackendA::factory())).unwrap();
    registry.register(Box::new(BackendB::factory())).unwrap();
    registry.register(Box::new(BackendC::factory())).unwrap();

    let backends = registry.list_backends();
    assert_eq!(backends, vec!["backend_a", "backend_b", "backend_c"]);
}
```

---

### 阶段 4: 更新文档 (后续)

#### 任务 4.1: 更新 README 和指南

**文件**:
- `canlink-hal/README.md`
- `specs/002-can-hardware-abstraction/quickstart.md`

**更新内容**:
- 添加 close 方法幂等性说明
- 添加资源清理最佳实践
- 添加注册顺序说明

---

## 优先级建议

### 立即执行 (今天)

1. ✅ 创建验证报告（已完成）
2. ⏳ 更新 backend-trait.md（任务 1.1）
3. ⏳ 更新 backend-registry.md（任务 1.2）
4. ⏳ 提交契约文档更新

### 发布前执行 (本周)

5. ⏳ 验证实现代码（任务 2.1, 2.2）
6. ⏳ 添加缺失的测试（任务 3.1, 3.2）
7. ⏳ 运行完整测试套件
8. ⏳ 更新相关文档（任务 4.1）

### 发布后执行 (下个版本)

9. 考虑添加更多契约测试
10. 考虑添加契约验证工具

---

## 风险评估

### 高风险

- **幂等性缺失**: 可能导致资源泄漏或崩溃
  - **缓解**: 立即更新文档并验证实现

- **注册顺序不确定**: 可能导致用户体验不一致
  - **缓解**: 使用 IndexMap 或添加顺序跟踪

### 中风险

- **错误类型不一致**: 可能导致错误处理混乱
  - **缓解**: 更新文档和实现

### 低风险

- **文档不完整**: 可能导致实现者困惑
  - **缓解**: 补充详细说明

---

## 结论

### 当前状态

⚠️ **契约文档与新需求存在不一致**

- backend-trait.md: 50% 一致性
- backend-registry.md: 62.5% 一致性
- 总体: 56.25% 一致性

### 建议

**不建议立即发布 v0.1.0**，原因：

1. 契约文档不完整，可能导致实现不一致
2. 关键需求（幂等性、注册顺序）未在契约中明确
3. 错误类型不匹配可能导致错误处理问题

**建议行动**:

1. **立即**: 更新契约文档（预计 25 分钟）
2. **今天**: 验证实现代码（预计 50 分钟）
3. **本周**: 添加测试并更新文档（预计 2 小时）
4. **然后**: 准备发布 v0.1.0

### 预计完成时间

- 契约文档更新: 25 分钟
- 实现验证: 50 分钟
- 测试和文档: 2 小时
- **总计**: ~3 小时

---

**报告生成时间**: 2026-01-10
**验证者**: Claude (AI Assistant)
**状态**: ⚠️ 需要改进后才能发布
