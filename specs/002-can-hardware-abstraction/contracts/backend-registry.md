# API 契约: BackendRegistry

**版本**: 1.0.0
**日期**: 2026-01-08
**目的**: 定义后端注册和发现机制（FR-002）

## 概述

`BackendRegistry` 管理所有已注册的硬件后端，提供后端注册、查询和创建功能。

## API 定义

```rust
use indexmap::IndexMap;
use std::sync::{Arc, RwLock};

/// 后端工厂 trait
///
/// 每个后端实现必须提供一个工厂，用于创建后端实例。
pub trait BackendFactory: Send + Sync {
    /// 创建后端实例
    fn create(&self) -> Box<dyn CanBackend>;

    /// 获取后端名称
    fn name(&self) -> &str;

    /// 获取后端版本
    fn version(&self) -> BackendVersion;
}

/// 后端注册表
///
/// # 线程安全
///
/// `BackendRegistry` 的所有方法都是线程安全的，可以从多个线程同时调用。
/// 内部使用 `RwLock` 保护共享状态。
///
/// # 注册顺序
///
/// 使用 `IndexMap` 保证后端按注册顺序存储和返回。
/// `list_backends()` 返回的列表按照注册时间排序。
///
/// # 示例
///
/// ```rust
/// // 注册后端
/// let registry = BackendRegistry::new();
/// registry.register(Box::new(MockBackendFactory::new()));
///
/// // 查询可用后端
/// let backends = registry.list_backends();
/// println!("Available backends: {:?}", backends);
///
/// // 创建后端实例
/// let backend = registry.create("mock")?;
/// ```
pub struct BackendRegistry {
    factories: RwLock<IndexMap<String, Box<dyn BackendFactory>>>,
}

impl BackendRegistry {
    /// 创建新的注册表
    ///
    /// # 返回
    /// 空的后端注册表
    ///
    /// # 示例
    /// ```rust
    /// let registry = BackendRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            factories: RwLock::new(IndexMap::new()),
        }
    }

    /// 获取全局注册表实例（单例）
    ///
    /// # 返回
    /// 全局共享的注册表实例
    ///
    /// # 示例
    /// ```rust
    /// let registry = BackendRegistry::global();
    /// ```
    pub fn global() -> Arc<Self> {
        static INSTANCE: OnceLock<Arc<BackendRegistry>> = OnceLock::new();
        INSTANCE
            .get_or_init(|| Arc::new(BackendRegistry::new()))
            .clone()
    }

    /// 注册后端工厂
    ///
    /// # 参数
    /// - `factory`: 后端工厂实例
    ///
    /// # 返回
    /// - `Ok(())`: 注册成功
    /// - `Err(CanError)`: 注册失败
    ///
    /// # 错误
    /// - `CanError::BackendAlreadyRegistered`: 后端名称已存在
    ///
    /// # 线程安全
    /// 此方法是线程安全的，可以从多个线程同时调用。
    ///
    /// # 示例
    /// ```rust
    /// registry.register(Box::new(MockBackendFactory::new()))?;
    /// ```
    pub fn register(&self, factory: Box<dyn BackendFactory>) -> Result<(), CanError> {
        let name = factory.name().to_string();
        let mut factories = self.factories.write().unwrap();

        if factories.contains_key(&name) {
            return Err(CanError::BackendAlreadyRegistered(name));
        }

        factories.insert(name, factory);
        Ok(())
    }

    /// 注销后端
    ///
    /// # 参数
    /// - `name`: 后端名称
    ///
    /// # 返回
    /// - `Ok(())`: 注销成功
    /// - `Err(CanError)`: 后端未找到
    ///
    /// # 示例
    /// ```rust
    /// registry.unregister("mock")?;
    /// ```
    pub fn unregister(&self, name: &str) -> Result<(), CanError> {
        let mut factories = self.factories.write().unwrap();

        if factories.remove(name).is_none() {
            return Err(CanError::BackendNotFound(name.to_string()));
        }

        Ok(())
    }

    /// 创建后端实例
    ///
    /// # 参数
    /// - `name`: 后端名称
    ///
    /// # 返回
    /// - `Ok(Box<dyn CanBackend>)`: 后端实例
    /// - `Err(CanError)`: 创建失败
    ///
    /// # 错误
    /// - `CanError::BackendNotFound`: 后端未注册
    ///
    /// # 线程安全
    /// 此方法是线程安全的，可以从多个线程同时调用。
    ///
    /// # 示例
    /// ```rust
    /// let backend = registry.create("mock")?;
    /// ```
    pub fn create(&self, name: &str) -> Result<Box<dyn CanBackend>, CanError> {
        let factories = self.factories.read().unwrap();

        let factory = factories
            .get(name)
            .ok_or_else(|| CanError::BackendNotFound(name.to_string()))?;

        Ok(factory.create())
    }

    /// 列出所有已注册的后端
    ///
    /// # 返回
    /// 后端名称列表，按注册顺序排序
    ///
    /// # 排序规则
    /// 返回的列表按照后端注册的时间顺序排列，
    /// 先注册的后端排在前面。
    ///
    /// # 实现说明
    /// 使用 `IndexMap` 而非 `HashMap` 以保证插入顺序。
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

    /// 获取后端信息
    ///
    /// # 参数
    /// - `name`: 后端名称
    ///
    /// # 返回
    /// - `Ok(BackendInfo)`: 后端信息
    /// - `Err(CanError)`: 后端未找到
    ///
    /// # 示例
    /// ```rust
    /// let info = registry.get_backend_info("mock")?;
    /// println!("Backend: {} v{}", info.name, info.version.version);
    /// ```
    pub fn get_backend_info(&self, name: &str) -> Result<BackendInfo, CanError> {
        let factories = self.factories.read().unwrap();

        let factory = factories
            .get(name)
            .ok_or_else(|| CanError::BackendNotFound(name.to_string()))?;

        Ok(BackendInfo {
            name: factory.name().to_string(),
            version: factory.version(),
        })
    }

    /// 检查后端是否已注册
    ///
    /// # 参数
    /// - `name`: 后端名称
    ///
    /// # 返回
    /// `true` 如果后端已注册，否则 `false`
    ///
    /// # 示例
    /// ```rust
    /// if registry.is_registered("mock") {
    ///     println!("Mock backend is available");
    /// }
    /// ```
    pub fn is_registered(&self, name: &str) -> bool {
        let factories = self.factories.read().unwrap();
        factories.contains_key(name)
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 后端信息
#[derive(Debug, Clone)]
pub struct BackendInfo {
    /// 后端名称
    pub name: String,

    /// 后端版本
    pub version: BackendVersion,
}
```

## 使用模式

### 模式 1: 全局注册表（推荐）

```rust
// 在应用启动时注册所有后端
fn register_backends() {
    let registry = BackendRegistry::global();
    registry.register(Box::new(MockBackendFactory::new())).unwrap();
    registry.register(Box::new(TsMasterBackendFactory::new())).unwrap();
}

// 在应用中使用
fn use_backend() -> Result<(), CanError> {
    let registry = BackendRegistry::global();
    let mut backend = registry.create("tsmaster")?;
    backend.initialize(&config)?;
    // ...
    Ok(())
}
```

### 模式 2: 本地注册表

```rust
// 创建独立的注册表实例
let registry = BackendRegistry::new();
registry.register(Box::new(MockBackendFactory::new()))?;

let mut backend = registry.create("mock")?;
```

### 模式 3: 从配置文件创建后端

```rust
// 从 TOML 配置加载并创建后端
fn create_backend_from_config(config_path: &str) -> Result<Box<dyn CanBackend>, CanError> {
    let config = CanlinkConfig::from_file(config_path)?;
    let registry = BackendRegistry::global();
    registry.create(&config.backend.backend_name)
}
```

## 线程安全保证

- ✅ 所有方法都是线程安全的
- ✅ 可以从多个线程同时注册和创建后端
- ✅ 使用 `RwLock` 优化读多写少的场景
- ✅ 工厂必须实现 `Send + Sync`

## 性能考虑

- 注册操作：低频（启动时），使用写锁
- 查询操作：中频，使用读锁
- 创建操作：中频，使用读锁

## 错误处理

| 错误类型 | 场景 | 处理建议 |
|---------|------|---------|
| `BackendNotFound` | 请求的后端未注册 | 检查后端名称拼写，确认后端已注册 |
| `BackendAlreadyRegistered` | 重复注册同名后端 | 先注销旧后端，或使用不同名称 |

## 测试要求

### 单元测试
- ✓ 注册和注销后端
- ✓ 创建后端实例
- ✓ 列出已注册后端
- ✓ 错误处理（未找到、重复注册）

### 并发测试
- ✓ 多线程同时注册
- ✓ 多线程同时创建
- ✓ 注册和创建并发

## 扩展点

### 自动注册宏（未来）

```rust
// 可能的未来扩展：自动注册宏
#[register_backend]
struct MyBackend;

// 自动生成工厂并注册到全局注册表
```

### 动态加载（未来）

```rust
// 可能的未来扩展：从动态库加载后端
registry.load_from_library("path/to/backend.so")?;
```

## 版本兼容性

- 主版本号变更：破坏性 API 变更
- 次版本号变更：向后兼容的功能添加
- 补丁版本号变更：向后兼容的错误修复
