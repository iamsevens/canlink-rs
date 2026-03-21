# 研究文档: CAN 硬件抽象层

**功能**: CAN 硬件抽象层
**日期**: 2026-01-08
**目的**: 解决技术背景中的 NEEDS CLARIFICATION 项目，为设计阶段提供技术决策依据

## 研究项目

### 1. 异步运行时选择（tokio vs async-std vs 可选）

#### 决策: 可选异步支持（feature flag 控制）

#### 理由:

**问题分析**:
- FR-010 要求"支持同步和异步操作模式"
- 不同用户场景对异步的需求不同：
  - 嵌入式系统：通常使用同步 API，避免运行时开销
  - 服务器应用：可能需要异步 API 以提高并发性能
  - 测试环境：同步 API 更简单直接

**技术方案**:
```rust
// 核心 trait 提供同步接口（默认）
pub trait CanBackend {
    fn send_message(&mut self, msg: &CanMessage) -> Result<(), CanError>;
    fn receive_message(&mut self) -> Result<Option<CanMessage>, CanError>;
}

// 可选的异步扩展（通过 feature flag "async"）
#[cfg(feature = "async")]
pub trait CanBackendAsync {
    async fn send_message_async(&mut self, msg: &CanMessage) -> Result<(), CanError>;
    async fn receive_message_async(&mut self) -> Result<Option<CanMessage>, CanError>;
}
```

**运行时选择**:
- 如果启用 `async` feature，用户可以选择：
  - `tokio`: 最流行，生态系统丰富（默认推荐）
  - `async-std`: 更接近标准库 API
  - 通过 feature flags 控制：`async-tokio` 或 `async-std`
- 不启用 `async` feature 时，零运行时开销

**依赖配置**:
```toml
[dependencies]
tokio = { version = "1.35", optional = true, features = ["sync", "time"] }
async-std = { version = "1.12", optional = true }

[features]
default = []
async = []
async-tokio = ["async", "tokio"]
async-std = ["async", "async-std"]
```

#### 替代方案考虑:

1. **强制使用 tokio**:
   - ❌ 拒绝理由：增加嵌入式系统的开销，违反章程性能约束

2. **仅提供同步 API**:
   - ❌ 拒绝理由：不满足 FR-010 要求，限制高并发场景的性能

3. **运行时无关的异步（futures-only）**:
   - ⚠️ 可行但复杂：需要用户自己集成运行时，增加使用难度

#### 实施影响:

- **代码复杂度**: 中等（需要维护同步和异步两套 API）
- **测试负担**: 需要测试两种模式
- **文档要求**: 需要清晰说明何时使用同步/异步
- **性能**: 同步模式零开销，异步模式按需启用

---

### 2. 线程安全模型澄清（解决 CHK004/CHK033/CHK054）

#### 决策: 分层线程安全策略

#### 理由:

**问题分析**:
- 规范假设中提到"初始化和关闭操作是线程安全的"
- FR-010 要求"外部同步（不提供内部锁）"
- 这两者看似矛盾，需要明确区分

**技术方案**:

```rust
// 1. 后端实例：要求外部同步（高频操作）
pub trait CanBackend: Send {
    // 这些方法要求调用者提供同步保护
    fn send_message(&mut self, msg: &CanMessage) -> Result<(), CanError>;
    fn receive_message(&mut self) -> Result<Option<CanMessage>, CanError>;
    // 注意：使用 &mut self，强制外部同步
}

// 2. 后端注册表：内部同步（低频操作）
pub struct BackendRegistry {
    backends: RwLock<HashMap<String, Box<dyn BackendFactory>>>,
}

impl BackendRegistry {
    // 注册操作有内部锁保护
    pub fn register(&self, name: String, factory: Box<dyn BackendFactory>) {
        self.backends.write().unwrap().insert(name, factory);
    }

    // 查询操作有内部锁保护
    pub fn get(&self, name: &str) -> Option<Box<dyn CanBackend>> {
        self.backends.read().unwrap().get(name).map(|f| f.create())
    }
}

// 3. 生命周期管理：内部同步（低频操作）
pub struct BackendHandle {
    backend: Arc<Mutex<Box<dyn CanBackend>>>,
    state: Arc<AtomicU8>, // 状态机：Uninitialized, Initializing, Running, Closing
}

impl BackendHandle {
    // 初始化有内部锁保护（低频操作）
    pub fn initialize(&self, config: &Config) -> Result<(), CanError> {
        let mut backend = self.backend.lock().unwrap();
        // 状态转换受保护
        self.state.store(State::Initializing as u8, Ordering::SeqCst);
        backend.initialize(config)?;
        self.state.store(State::Running as u8, Ordering::SeqCst);
        Ok(())
    }

    // 关闭有内部锁保护（低频操作）
    pub fn close(&self) -> Result<(), CanError> {
        let mut backend = self.backend.lock().unwrap();
        self.state.store(State::Closing as u8, Ordering::SeqCst);
        backend.close()
    }
}
```

**设计原则**:

| 操作类型 | 频率 | 线程安全策略 | 理由 |
|---------|------|------------|------|
| 消息收发 | 高频 (1000+ msg/s) | 外部同步 | 避免锁开销，性能关键路径 |
| 能力查询 | 中频 | 外部同步 | 通常在初始化时调用一次 |
| 后端注册 | 低频 (启动时) | 内部同步 | 简化用户代码，性能影响小 |
| 初始化/关闭 | 低频 (生命周期) | 内部同步 | 保证状态一致性，避免竞态条件 |

**文档说明**:
```rust
/// # 线程安全
///
/// ## 后端实例操作（外部同步）
/// `CanBackend` trait 的方法要求调用者提供同步保护。
/// 如果需要从多个线程访问同一个后端实例，请使用 `Mutex` 或 `RwLock`：
///
/// ```rust
/// let backend = Arc::new(Mutex::new(create_backend()));
///
/// // 线程 1
/// backend.lock().unwrap().send_message(&msg1)?;
///
/// // 线程 2
/// backend.lock().unwrap().send_message(&msg2)?;
/// ```
///
/// ## 后端注册和生命周期（内部同步）
/// `BackendRegistry` 和 `BackendHandle` 的方法是线程安全的，
/// 可以从多个线程同时调用而无需额外同步。
```

#### 替代方案考虑:

1. **所有操作都外部同步**:
   - ❌ 拒绝理由：初始化/关闭的状态管理容易出错，用户负担重

2. **所有操作都内部同步**:
   - ❌ 拒绝理由：高频消息收发路径有锁开销，违反性能目标（< 5% 开销）

3. **提供两种后端实现（Sync 和 Unsync）**:
   - ❌ 拒绝理由：增加维护负担，API 复杂度翻倍

#### 实施影响:

- **代码复杂度**: 中等（需要清晰的文档说明）
- **性能**: 高频路径零锁开销，满足性能目标
- **用户体验**: 平衡了易用性和性能
- **规范更新**: 需要在规范的"假设"部分澄清这一设计

---

## 研究总结

### 已解决的 NEEDS CLARIFICATION 项目

1. ✅ **异步运行时选择**: 采用可选异步支持（feature flag 控制），默认同步 API，可选 tokio 或 async-std
2. ✅ **线程安全模型**: 采用分层策略，高频操作外部同步，低频操作内部同步

### 技术栈确认

**最终依赖列表**:
```toml
[dependencies]
toml = "0.8"                    # TOML 配置解析
thiserror = "1.0"               # 错误类型定义
semver = "1.0"                  # 语义版本控制
tokio = { version = "1.35", optional = true, features = ["sync", "time"] }
async-std = { version = "1.12", optional = true }

[features]
default = []
async = []
async-tokio = ["async", "tokio"]
async-async-std = ["async", "async-std"]
```

### 下一步行动

1. 进入 Phase 1：设计数据模型（data-model.md）
2. 定义 API 契约（contracts/）
3. 编写快速入门指南（quickstart.md）
4. 更新 agent context

### 需要在设计阶段关注的问题

- 确保异步 trait 的设计与同步 trait 保持一致
- 为分层线程安全策略编写清晰的文档和示例
- 在 Mock 后端中验证两种模式（同步和异步）都能正常工作
