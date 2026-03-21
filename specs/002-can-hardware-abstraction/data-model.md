# 数据模型: CAN 硬件抽象层

**功能**: CAN 硬件抽象层
**日期**: 2026-01-08
**目的**: 定义所有关键实体的数据结构、字段、关系和验证规则

## 核心实体

### 1. CanMessage - 统一消息类型

**目的**: 与硬件无关的 CAN 消息表示（FR-007）

**字段**:

| 字段名 | 类型 | 必需 | 描述 | 验证规则 |
|--------|------|------|------|---------|
| `id` | `CanId` | ✓ | CAN 标识符 | 标准帧: 0x000-0x7FF, 扩展帧: 0x00000000-0x1FFFFFFF |
| `data` | `Vec<u8>` | ✓ | 数据字节（拥有所有权） | CAN 2.0: 0-8 字节, CAN-FD: 0-64 字节 |
| `timestamp` | `Option<Timestamp>` | | 接收/发送时间戳 | 微秒精度 |
| `flags` | `MessageFlags` | ✓ | 消息标志位 | 见 MessageFlags 定义 |

**Rust 定义**:
```rust
/// 统一的 CAN 消息类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanMessage {
    /// CAN 标识符（标准或扩展）
    pub id: CanId,

    /// 消息数据（最多 64 字节）
    pub data: Vec<u8>,

    /// 时间戳（微秒精度）
    pub timestamp: Option<Timestamp>,

    /// 消息标志位
    pub flags: MessageFlags,
}

impl CanMessage {
    /// 创建标准 CAN 2.0 数据帧
    pub fn new_standard(id: u16, data: &[u8]) -> Result<Self, CanError> {
        if id > 0x7FF {
            return Err(CanError::InvalidId);
        }
        if data.len() > 8 {
            return Err(CanError::InvalidDataLength);
        }
        Ok(Self {
            id: CanId::Standard(id),
            data: data.to_vec(),
            timestamp: None,
            flags: MessageFlags::default(),
        })
    }

    /// 创建扩展 CAN 2.0B 数据帧
    pub fn new_extended(id: u32, data: &[u8]) -> Result<Self, CanError> {
        if id > 0x1FFFFFFF {
            return Err(CanError::InvalidId);
        }
        if data.len() > 8 {
            return Err(CanError::InvalidDataLength);
        }
        Ok(Self {
            id: CanId::Extended(id),
            data: data.to_vec(),
            timestamp: None,
            flags: MessageFlags::default(),
        })
    }

    /// 创建 CAN-FD 数据帧
    pub fn new_fd(id: CanId, data: &[u8]) -> Result<Self, CanError> {
        if data.len() > 64 {
            return Err(CanError::InvalidDataLength);
        }
        Ok(Self {
            id,
            data: data.to_vec(),
            timestamp: None,
            flags: MessageFlags::FD | MessageFlags::BRS,
        })
    }

    /// 创建远程帧
    pub fn new_remote(id: CanId, dlc: u8) -> Result<Self, CanError> {
        if dlc > 8 {
            return Err(CanError::InvalidDataLength);
        }
        Ok(Self {
            id,
            data: vec![],
            timestamp: None,
            flags: MessageFlags::RTR,
        })
    }
}
```

**状态转换**: N/A（消息是不可变的值对象）

**关系**:
- 被 `CanBackend` trait 的 `send_message` 和 `receive_message` 方法使用
- 被 `MockBackend` 记录和验证

---

### 2. CanId - CAN 标识符

**目的**: 类型安全的 CAN ID 表示

**Rust 定义**:
```rust
/// CAN 标识符（标准或扩展）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CanId {
    /// 标准 11 位 ID (0x000-0x7FF)
    Standard(u16),

    /// 扩展 29 位 ID (0x00000000-0x1FFFFFFF)
    Extended(u32),
}

impl CanId {
    /// 获取原始 ID 值
    pub fn raw(&self) -> u32 {
        match self {
            CanId::Standard(id) => *id as u32,
            CanId::Extended(id) => *id,
        }
    }

    /// 是否为标准帧
    pub fn is_standard(&self) -> bool {
        matches!(self, CanId::Standard(_))
    }

    /// 是否为扩展帧
    pub fn is_extended(&self) -> bool {
        matches!(self, CanId::Extended(_))
    }
}
```

---

### 3. MessageFlags - 消息标志位

**目的**: 表示消息的各种属性

**Rust 定义**:
```rust
use bitflags::bitflags;

bitflags! {
    /// CAN 消息标志位
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MessageFlags: u8 {
        /// 远程帧（RTR）
        const RTR = 0b0000_0001;

        /// CAN-FD 格式
        const FD = 0b0000_0010;

        /// 位速率切换（BRS）
        const BRS = 0b0000_0100;

        /// 错误状态指示（ESI）
        const ESI = 0b0000_1000;
    }
}

impl Default for MessageFlags {
    fn default() -> Self {
        MessageFlags::empty()
    }
}
```

---

### 4. Timestamp - 时间戳

**目的**: 微秒精度的时间戳

**Rust 定义**:
```rust
/// 微秒精度的时间戳
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    /// 微秒数（从某个参考点开始）
    micros: u64,
}

impl Timestamp {
    /// 创建时间戳
    pub fn from_micros(micros: u64) -> Self {
        Self { micros }
    }

    /// 获取微秒数
    pub fn as_micros(&self) -> u64 {
        self.micros
    }

    /// 获取毫秒数
    pub fn as_millis(&self) -> u64 {
        self.micros / 1000
    }
}
```

---

### 5. CanError - 统一错误类型

**目的**: 与硬件无关的错误表示（FR-006）

**字段**:

| 字段名 | 类型 | 描述 |
|--------|------|------|
| `kind` | `ErrorKind` | 错误类别 |
| `message` | `String` | 错误描述 |
| `context` | `Option<ErrorContext>` | 错误上下文信息 |

**Rust 定义**:
```rust
use thiserror::Error;

/// 统一的 CAN 错误类型
#[derive(Error, Debug)]
pub enum CanError {
    /// 后端未找到
    #[error("Backend not found: {0}")]
    BackendNotFound(String),

    /// 后端初始化失败
    #[error("Backend initialization failed: {0}")]
    InitializationFailed(String),

    /// 无效的 CAN ID
    #[error("Invalid CAN ID")]
    InvalidId,

    /// 无效的数据长度
    #[error("Invalid data length: expected {expected}, got {actual}")]
    InvalidDataLength { expected: usize, actual: usize },

    /// 硬件不支持的功能
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// 通道不存在
    #[error("Channel {0} does not exist")]
    ChannelNotFound(u8),

    /// 发送失败
    #[error("Send failed: {0}")]
    SendFailed(String),

    /// 接收失败
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    /// 总线错误
    #[error("Bus error: {0}")]
    BusError(BusErrorKind),

    /// 版本不兼容
    #[error("Version incompatible: backend {backend_version}, expected {expected_version}")]
    VersionIncompatible {
        backend_version: String,
        expected_version: String,
    },

    /// 配置错误
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// 超时
    #[error("Operation timed out")]
    Timeout,

    /// 其他错误
    #[error("Other error: {0}")]
    Other(String),
}

/// 总线错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusErrorKind {
    /// 位错误
    BitError,

    /// 填充错误
    StuffError,

    /// CRC 错误
    CrcError,

    /// ACK 错误
    AckError,

    /// 格式错误
    FormError,
}
```

---

### 6. HardwareCapability - 硬件能力描述

**目的**: 描述硬件的能力和限制（FR-003）

**字段**:

| 字段名 | 类型 | 必需 | 描述 |
|--------|------|------|------|
| `channel_count` | `u8` | ✓ | 支持的通道数 |
| `supports_canfd` | `bool` | ✓ | 是否支持 CAN-FD |
| `max_bitrate` | `u32` | ✓ | 最大波特率（bps） |
| `supported_bitrates` | `Vec<u32>` | ✓ | 支持的波特率列表 |
| `filter_count` | `u8` | ✓ | 支持的过滤器数量 |
| `timestamp_precision` | `TimestampPrecision` | ✓ | 时间戳精度 |

**Rust 定义**:
```rust
/// 硬件能力描述
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareCapability {
    /// 支持的通道数
    pub channel_count: u8,

    /// 是否支持 CAN-FD
    pub supports_canfd: bool,

    /// 最大波特率（bps）
    pub max_bitrate: u32,

    /// 支持的波特率列表
    pub supported_bitrates: Vec<u32>,

    /// 支持的过滤器数量
    pub filter_count: u8,

    /// 时间戳精度
    pub timestamp_precision: TimestampPrecision,
}

/// 时间戳精度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimestampPrecision {
    /// 微秒级
    Microsecond,

    /// 毫秒级
    Millisecond,

    /// 无时间戳
    None,
}
```

---

### 7. BackendConfig - 后端配置

**目的**: 从 TOML 配置文件加载的后端配置（FR-004）

**字段**:

| 字段名 | 类型 | 必需 | 描述 |
|--------|------|------|------|
| `backend_name` | `String` | ✓ | 后端名称 |
| `parameters` | `HashMap<String, Value>` | | 后端特定参数 |

**Rust 定义**:
```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// 后端配置
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackendConfig {
    /// 后端名称（如 "tsmaster", "mock"）
    pub backend_name: String,

    /// 后端特定参数
    #[serde(flatten)]
    pub parameters: HashMap<String, toml::Value>,
}

/// 完整的配置文件结构
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CanlinkConfig {
    /// 后端配置
    pub backend: BackendConfig,
}

impl CanlinkConfig {
    /// 从 TOML 文件加载配置
    pub fn from_file(path: &str) -> Result<Self, CanError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| CanError::ConfigError(format!("Failed to read config: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| CanError::ConfigError(format!("Failed to parse config: {}", e)))
    }
}
```

**TOML 配置示例**:
```toml
[backend]
backend_name = "tsmaster"
device_index = 0
channel = 0
bitrate = 500000

# 或者使用 Mock 后端
# [backend]
# backend_name = "mock"
# preset_messages = ["0x123:01020304", "0x456:05060708"]
```

---

### 8. BackendVersion - 后端版本

**目的**: 版本管理和兼容性检查（FR-008）

**Rust 定义**:
```rust
use semver::Version;

/// 后端版本信息
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendVersion {
    /// 语义版本号
    pub version: Version,
}

impl BackendVersion {
    /// 创建版本
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            version: Version::new(major, minor, patch),
        }
    }

    /// 检查是否与另一个版本兼容（主版本号相同）
    pub fn is_compatible_with(&self, other: &BackendVersion) -> bool {
        self.version.major == other.version.major
    }
}
```

---

### 9. BackendState - 后端状态

**目的**: 生命周期管理（FR-009）

**状态转换图**:
```
Uninitialized
    ↓ initialize()
Initializing (重试最多 3 次)
    ↓ 成功
Running
    ↓ close()
Closing
    ↓
Closed
```

**Rust 定义**:
```rust
/// 后端状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendState {
    /// 未初始化
    Uninitialized,

    /// 初始化中
    Initializing,

    /// 运行中
    Running,

    /// 关闭中
    Closing,

    /// 已关闭
    Closed,
}
```

---

## 实体关系图

```
CanlinkConfig
    ↓ contains
BackendConfig
    ↓ used by
BackendRegistry
    ↓ creates
CanBackend (trait)
    ↓ implements
    ├── MockBackend
    ├── TsMasterBackend
    └── [其他后端]

CanBackend
    ↓ sends/receives
CanMessage
    ↓ contains
    ├── CanId
    ├── MessageFlags
    └── Timestamp

CanBackend
    ↓ returns
    ├── CanError
    └── HardwareCapability
```

---

## 验证规则总结

### CanMessage
- ✓ 标准帧 ID: 0x000-0x7FF
- ✓ 扩展帧 ID: 0x00000000-0x1FFFFFFF
- ✓ CAN 2.0 数据长度: 0-8 字节
- ✓ CAN-FD 数据长度: 0-64 字节
- ✓ 远程帧 DLC: 0-8

### BackendVersion
- ✓ 主版本号相同即视为兼容
- ✓ 使用语义版本控制（SemVer）

### BackendConfig
- ✓ backend_name 必须非空
- ✓ TOML 格式必须有效

---

## 下一步

1. 基于这些数据模型生成 API 契约（contracts/）
2. 定义 `CanBackend` trait 的完整接口
3. 定义 `BackendRegistry` 的 API
4. 编写快速入门指南（quickstart.md）
