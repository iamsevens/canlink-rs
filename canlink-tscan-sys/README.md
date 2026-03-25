# CANLink TSCan Sys



[![Crates.io](https://img.shields.io/crates/v/canlink-tscan-sys.svg)](https://crates.io/crates/canlink-tscan-sys)

[![Documentation](https://docs.rs/canlink-tscan-sys/badge.svg)](https://docs.rs/canlink-tscan-sys)

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](../LICENSE-MIT)



**CANLink TSCan Sys** 是 LibTSCAN 的底层 Rust FFI 绑定，提供对 LibTSCAN 可识别 CAN 硬件的直接访问。

当前仓库的实机接入与回归验证，仍然以同星 / TOSUN 相关硬件为准；其他文档可见设备类型不在本 crate 中单独承诺兼容性。


## ⚠️ 警告



这是一个**底层 FFI 绑定包**，直接暴露 C API。除非您需要直接访问 LibTSCAN 的原始功能，否则应该使用更高级的 [canlink-tscan](../canlink-tscan/) 包。



## 特性



- 🔗 **完整 FFI 绑定** - 覆盖所有 LibTSCAN API

- 🛡️ **类型安全** - Rust 类型定义

- 📝 **文档完善** - 每个函数都有详细文档

- 🔧 **零依赖** - 仅依赖标准库

- 🎯 **直接访问** - 无额外抽象层



## 系统要求



- **操作系统**: 当前实现仅支持 Windows 10/11 (x64)，其他平台尚未适配/验证

- **依赖**: `libTSCAN.dll` + `libTSCAN.lib`（x64）



## 安装



```toml

[dependencies]

canlink-tscan-sys = "0.3.0"

```

> 本项目不分发 LibTSCAN 文件，请按厂商许可自行获取，并参考 `docs/guides/libtscan-setup-guide.md` 进行配置。



## 使用



### 基础示例



```rust

use canlink_tscan_sys::*;

use std::ptr;



unsafe {

    // 初始化库

    initialize_lib_tscan(true, false, true);



    // 扫描设备

    let mut device_count = 0;

    let result = tscan_scan_devices(&mut device_count);

    if result == 0 {

        println!("找到 {} 个设备", device_count);

    }



    // 连接设备

    let mut handle = 0;

    let result = tscan_connect(ptr::null(), &mut handle);

    if result == 0 {

        println!("已连接，句柄: {}", handle);



        // 发送 CAN 消息

        let mut msg = TLIBCAN {

            FIdxChn: 0,

            FProperties: 0,

            FDLC: 8,

            FReserved: 0,

            FIdentifier: 0x123,

            FTimeUs: 0,

            FData: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08],

        };



        let result = tscan_transmit_can_async(handle, &msg);

        if result == 0 {

            println!("消息已发送");

        }



        // 断开连接

        tscan_disconnect_by_handle(handle);

    }



    // 清理

    finalize_lib_tscan();

}

```



## API 概览



### 库初始化



```rust

// 初始化 LibTSCAN

pub fn initialize_lib_tscan(

    AEnableFIFO: bool,

    AEnableErrorFrame: bool,

    AUseHWTime: bool

);



// 清理 LibTSCAN

pub fn finalize_lib_tscan();

```



### 设备管理



```rust

// 扫描设备

pub fn tscan_scan_devices(ADeviceCount: *mut u32) -> u32;



// 获取设备信息

pub fn tscan_get_device_info(

    ADeviceIndex: u32,

    ADeviceInfo: *mut TTSDeviceInfo

) -> u32;



// 连接设备

pub fn tscan_connect(

    ADeviceSerial: *const c_char,

    AHandle: *mut usize

) -> u32;



// 断开设备

pub fn tscan_disconnect_by_handle(ADeviceHandle: usize) -> u32;

pub fn tscan_disconnect_all_devices() -> u32;

```



### 硬件能力查询



```rust

// 获取通道数

pub fn tscan_get_can_channel_count(

    ADeviceHandle: usize,

    AChnCount: *mut s32,

    AIsFDCAN: *mut bool

) -> u32;



// 获取设备类型

pub fn tscan_get_device_type(

    ADeviceHandle: usize,

    ADeviceType: *mut u32,

    ADeviceName: *mut *const c_char

) -> u32;

```



### CAN 消息传输



```rust

// 发送 CAN 消息 (异步)

pub fn tscan_transmit_can_async(

    ADeviceHandle: usize,

    ACAN: *const TLIBCAN

) -> u32;



// 发送 CAN 消息 (同步)

pub fn tscan_transmit_can_sync(

    ADeviceHandle: usize,

    ACAN: *const TLIBCAN,

    ATimeoutMS: u32

) -> u32;



// 发送 CAN-FD 消息

pub fn tscan_transmit_canfd_async(

    ADeviceHandle: usize,

    ACANFD: *const TLIBCANFD

) -> u32;

```



### CAN 消息接收



```rust

// 接收 CAN 消息

pub fn tscan_receive_can_msgs(

    ADeviceHandle: usize,

    ACAN: *mut TLIBCAN,

    ACANBufferSize: s32,

    AActualCANCount: *mut s32,

    ATimeoutMS: s32

) -> u32;



// 接收 CAN-FD 消息

pub fn tscan_receive_canfd_msgs(

    ADeviceHandle: usize,

    ACANFD: *mut TLIBCANFD,

    ACANFDBufferSize: s32,

    AActualCANFDCount: *mut s32,

    ATimeoutMS: s32

) -> u32;

```



## 数据结构



### TLIBCAN



标准 CAN 消息结构：



```rust

#[repr(C)]

pub struct TLIBCAN {

    pub FIdxChn: u8,        // 通道索引 (0-based)

    pub FProperties: u8,     // 属性标志

    pub FDLC: u8,           // 数据长度 (0-8)

    pub FReserved: u8,      // 保留字段

    pub FIdentifier: s32,   // CAN ID

    pub FTimeUs: s64,       // 时间戳 (微秒)

    pub FData: [u8; 8],     // 数据字节

}

```



### TLIBCANFD



CAN-FD 消息结构：



```rust

#[repr(C)]

pub struct TLIBCANFD {

    pub FIdxChn: u8,        // 通道索引

    pub FProperties: u8,     // 属性标志

    pub FDLC: u8,           // 数据长度码 (0-15)

    pub FFDProperties: u8,   // FD 属性 (EDL, BRS, ESI)

    pub FIdentifier: s32,   // CAN ID

    pub FTimeUs: s64,       // 时间戳

    pub FData: [u8; 64],    // 数据字节 (最多 64)

}

```



### TTSDeviceInfo



设备信息结构：



```rust

#[repr(C)]

pub struct TTSDeviceInfo {

    pub FDeviceIndex: u32,

    pub FDeviceType: u32,

    pub FDeviceName: [c_char; 256],

    pub FSerialString: [c_char; 256],

    // ... 更多字段

}

```



## 属性标志



### FProperties (消息属性)



```rust

const PROPERTY_TX: u8 = 0x01;       // 发送消息

const PROPERTY_REMOTE: u8 = 0x02;   // 远程帧

const PROPERTY_EXTENDED: u8 = 0x04; // 扩展帧

const PROPERTY_ERROR: u8 = 0x80;    // 错误帧

```



### FFDProperties (CAN-FD 属性)



```rust

const FD_PROPERTY_EDL: u8 = 0x01;   // 扩展数据长度

const FD_PROPERTY_BRS: u8 = 0x02;   // 波特率切换

const FD_PROPERTY_ESI: u8 = 0x04;   // 错误状态指示

```



## 错误码



```rust

const ERROR_OK: u32 = 0;                    // 成功

const ERROR_DEVICE_NOT_FOUND: u32 = 1;      // 设备未找到

const ERROR_DEVICE_NOT_CONNECTED: u32 = 2;  // 设备未连接

const ERROR_INVALID_PARAMETER: u32 = 3;     // 无效参数

const ERROR_TIMEOUT: u32 = 4;               // 超时

// ... 更多错误码

```



## 安全性



⚠️ **所有函数都是 `unsafe` 的**，因为它们直接调用 C FFI。使用时需要注意：



1. **空指针检查**: 确保传递有效的指针

2. **内存管理**: 正确管理分配的内存

3. **线程安全**: LibTSCAN 不保证线程安全

4. **错误处理**: 检查所有返回值



### 安全使用示例



```rust

use canlink_tscan_sys::*;

use std::ptr;



unsafe {

    // 初始化

    initialize_lib_tscan(true, false, true);



    // 扫描设备

    let mut device_count = 0;

    let result = tscan_scan_devices(&mut device_count);



    if result != 0 {

        eprintln!("扫描失败: {}", result);

        finalize_lib_tscan();

        return;

    }



    if device_count == 0 {

        eprintln!("未找到设备");

        finalize_lib_tscan();

        return;

    }



    // 连接设备

    let mut handle = 0;

    let result = tscan_connect(ptr::null(), &mut handle);



    if result != 0 {

        eprintln!("连接失败: {}", result);

        finalize_lib_tscan();

        return;

    }



    // 使用设备...



    // 清理

    tscan_disconnect_by_handle(handle);

    finalize_lib_tscan();

}

```



## 构建要求



### Windows



构建阶段需要同时找到 `libTSCAN.dll` 与 `libTSCAN.lib`（x64），推荐通过环境变量指定：

1. `CANLINK_TSCAN_BUNDLE_DIR=<包含 libTSCAN.dll 和 libTSCAN.lib 的目录>`
2. 或设置 `TSMASTER_HOME`（构建脚本会尝试 `<TSMASTER_HOME>/bin/x64` 与 `<TSMASTER_HOME>/bin`）
3. 若未设置环境变量，构建脚本会回退尝试仓库内 `libs/` 与 vendor 示例目录

运行时 `libTSCAN.dll` 还需要位于以下位置之一：



1. 应用程序目录

2. 系统 PATH 中的目录

3. `C:\Windows\System32`



### 链接



构建脚本会自动处理链接搜索路径与运行时 DLL 复制，无需手动修改 `build.rs`。核心链接指令如下：



```rust

fn main() {

    println!("cargo:rustc-link-lib=libTSCAN");

}

```



## 测试



```bash

# 运行测试 (需要硬件)

cargo test -p canlink-tscan-sys



# 构建文档

cargo doc -p canlink-tscan-sys --open

```



## 与高级包的关系



```

应用程序

    ↓

canlink-hal (抽象层)

    ↓

canlink-tscan (安全封装)

    ↓

canlink-tscan-sys (FFI 绑定) ← 您在这里

    ↓

libTSCAN.dll (C 库)

    ↓

LibTSCAN 可识别硬件
```



## 何时使用此包



**使用 canlink-tscan-sys 如果**:

- 需要直接访问 LibTSCAN 的所有功能

- 需要最大性能（避免额外抽象）

- 实现自定义的高级封装



**使用 canlink-tscan 如果**:

- 需要类型安全的 Rust API

- 想要自动的资源管理

- 需要与 CANLink HAL 集成



**使用 canlink-hal 如果**:

- 需要硬件无关的代码

- 想要在不同后端间切换

- 需要 Mock 测试支持



## 示例



查看 `examples/` 目录：



- `raw_ffi.rs` - 原始 FFI 调用示例

- `device_scan.rs` - 设备扫描示例

- `send_receive.rs` - 发送接收示例



## 文档



- [API 文档](https://docs.rs/canlink-tscan-sys)

- [LibTSCAN 官方文档](https://www.tosun.com/)



## 相关包



- [canlink-tscan](../canlink-tscan/) - 高级安全封装

- [canlink-hal](../canlink-hal/) - 硬件抽象层



## 贡献



欢迎贡献！请查看 [贡献指南](../CONTRIBUTING.md)。



## 许可证



MIT OR Apache-2.0



**注意**: LibTSCAN 本身可能有不同的许可证，请查看 LibTSCAN / TSMaster 官方文档。




