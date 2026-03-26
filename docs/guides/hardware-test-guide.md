# LibTSCAN 硬件测试指南

## 📋 前提条件

### 1. 硬件要求
- 可被 LibTSCAN 识别的 CAN 硬件设备（当前回归验证基于同星 / TOSUN 相关硬件）
- USB 连接线
- CAN 总线终端电阻（如果需要）

### 2. 软件要求
- Windows 操作系统（当前测试流程在 Windows 验证，其他平台未验证）
- TSMaster 软件已安装
- LibTSCAN 运行库目录（建议完整包；最低包含 `libTSCAN.lib` + `libTSCAN.dll`，通常还需要 `libTSH.dll` 等依赖 DLL）

## 🔧 安装步骤

### 步骤 1: 安装 TSMaster 软件

1. 从 TSMaster 官网下载安装包
2. 运行安装程序
3. 默认安装路径：`C:\Program Files\TSMaster\`

### 步骤 2: 定位 LibTSCAN 运行库目录

常见位置（以实际安装版本为准）：
- `C:\Program Files (x86)\TOSUN\TSMaster\bin64\`
- `C:\Program Files (x86)\TOSUN\TSMaster\bin\`
- API 包解压目录中的 `lib/lib/windows/x64` 或 `lib/lib/windows/x86`

### 步骤 3: 配置运行库路径

有三种方法可以让程序找到运行库：

#### 方法 1: 添加到系统 PATH（推荐）
```cmd
# 临时添加（当前会话有效）
set PATH=%PATH%;C:\Program Files (x86)\TOSUN\TSMaster\bin64

# 永久添加（需要管理员权限）
setx PATH "%PATH%;C:\Program Files (x86)\TOSUN\TSMaster\bin64"
```

#### 方法 2: 复制运行时 DLL 到项目目录
```cmd
# 复制到 target/debug 目录（示例：复制 bin64 下所有 DLL）
copy "C:\Program Files (x86)\TOSUN\TSMaster\bin64\*.dll" .\target\debug\

# 复制到 target/release 目录
copy "C:\Program Files (x86)\TOSUN\TSMaster\bin64\*.dll" .\target\release\
```

#### 方法 3: 修改 build.rs 指定路径
编辑 `canlink-tscan-sys/build.rs`，取消注释并修改路径：
```rust
println!("cargo:rustc-link-search=native=C:\\Program Files\\TSMaster\\bin");
```

### 步骤 4: 创建导入库（如果需要）

如果编译时提示找不到 `libTSCAN.lib`，需要创建导入库：

```cmd
# 使用 Visual Studio 的 lib.exe 工具
cd "C:\Program Files\TSMaster\bin"
lib /def:libTSCAN.def /out:libTSCAN.lib /machine:x64

# 或者使用 dumpbin 和 lib
dumpbin /exports libTSCAN.dll > exports.txt
# 根据 exports.txt 创建 .def 文件
lib /def:libTSCAN.def /out:libTSCAN.lib /machine:x64
```

**注意**: 如果 TSMaster 已经提供了 `.lib` 文件，直接使用即可。

## 🚀 运行测试

### 1. 连接硬件
1. 将 TSMaster 设备通过 USB 连接到电脑
2. 确认设备驱动已正确安装
3. 在设备管理器中检查设备状态

### 2. 编译测试程序

```cmd
cd <repo-root>\canlink-tscan-sys
cargo build --example basic_hardware_test
```

### 3. 运行测试程序

```cmd
cargo run --example basic_hardware_test
```

## 📊 预期输出

如果一切正常，你应该看到类似以下的输出：

```
🔍 LibTSCAN Hardware Connection Test

=====================================

1. Initializing LibTSCAN library...
   ✓ Library initialized

2. Scanning for devices...
   ✓ Found 1 device(s)

3. Getting device information...
   Device 0: TSMaster TSMaster Mini (S/N: 12345678)

4. Connecting to default device...
   ✓ Connected (handle: 0x1234ABCD)

5. Querying device capabilities...
   CAN Channels: 2
   CAN-FD Support: Yes
   Device Type: 1 (TSMaster Mini)

6. Configuring CAN channel 0 (500 kbps)...
   ✓ Channel configured

7. Sending test CAN message...
   ✓ Message sent: ID=0x123, DLC=8, Data=[01, 02, 03, 04, 05, 06, 07, 08]

8. Receiving messages (5 seconds)...
   RX Std ID=0x456, DLC=8, Data=[AA, BB, CC, DD, EE, FF, 00, 11]
   Total received: 1 messages

9. Cleaning up...
   ✓ Disconnected
   ✓ Library finalized

=====================================
✅ Test completed successfully!
```

## ⚠️ 常见问题

### 问题 1: 找不到 libTSCAN.dll

**错误信息**:
```
error while loading shared libraries: libTSCAN.dll: cannot open shared object file
```

**解决方法**:
1. 检查 `libTSCAN.dll` 是否存在
2. 同时检查依赖 DLL（如 `libTSH.dll`）是否存在
3. 将运行库目录加入 PATH，或复制完整 DLL 集到可执行文件目录

### 问题 2: 找不到设备

**错误信息**:
```
Found 0 device(s)
⚠️  No devices found. Please connect a TSMaster device.
```

**解决方法**:
1. 检查 USB 连接
2. 确认设备驱动已安装
3. 在设备管理器中查看设备状态
4. 尝试重新插拔设备

### 问题 3: 连接失败

**错误信息**:
```
❌ Failed to connect (error code: XXX)
```

**解决方法**:
1. 确认设备未被其他程序占用
2. 关闭 TSMaster 软件（如果正在运行）
3. 检查设备权限
4. 尝试以管理员身份运行

### 问题 4: 配置通道失败

**错误信息**:
```
❌ Failed to configure channel (error code: XXX)
```

**解决方法**:
1. 检查通道索引是否正确（0-based）
2. 确认设备支持该通道
3. 检查波特率是否有效
4. 确认设备已正确初始化

### 问题 5: 发送消息失败

**错误信息**:
```
❌ Failed to send message (error code: XXX)
```

**解决方法**:
1. 确认通道已配置
2. 检查 CAN ID 是否有效
3. 确认数据长度正确（0-8 字节）
4. 检查 CAN 总线连接

## 🔍 调试技巧

### 1. 启用详细日志

修改测试程序，添加错误描述：

```rust
if result != 0 {
    let mut desc: *const i8 = ptr::null();
    tscan_get_error_description(result, &mut desc);
    if !desc.is_null() {
        let error_msg = CStr::from_ptr(desc).to_string_lossy();
        println!("   Error: {}", error_msg);
    }
}
```

### 2. 检查设备状态

使用 TSMaster 软件检查：
1. 打开 TSMaster
2. 连接设备
3. 查看设备信息和状态
4. 测试基本通信功能

### 3. 使用 CAN 分析仪

如果有 CAN 分析仪：
1. 连接到 CAN 总线
2. 监控发送的消息
3. 验证消息格式和内容

## 📚 参考资料

### LibTSCAN 文档
- 厂商资料说明: `docs/vendor/tsmaster/README.md`
- 官方下载入口: <https://www.tosunai.com/en/downloads/>
- TSMaster API 下载页: <https://www.tosunai.com/downloads/tsmaster-api/>

### TSMaster 资源
- 官方网站: [TSMaster 官网]
- 用户手册: TSMaster 安装目录下的文档
- 技术支持: 联系 TSMaster 技术支持

## 🎯 下一步

测试成功后，可以：

1. **实现高层封装**: 创建安全的 Rust 包装器
2. **集成到 canlink-hal**: 实现 `CanBackend` trait
3. **编写单元测试**: 测试各个功能模块
4. **性能测试**: 测试消息吞吐量和延迟
5. **完善错误处理**: 添加详细的错误信息

## ✅ 检查清单

在运行测试前，确认：

- [ ] TSMaster 软件已安装
- [ ] `libTSCAN.dll` 与依赖 DLL（如 `libTSH.dll`）可访问（在 PATH 或项目目录）
- [ ] TSMaster 设备已连接
- [ ] 设备驱动已安装
- [ ] 没有其他程序占用设备
- [ ] 编译环境正确（Windows + MSVC）

---

**准备好了吗？运行测试吧！** 🚀

```cmd
cargo run --example basic_hardware_test
```


## 逐模块硬件测试 (Per-Module Hardware Tests)

### Environment and Logs
- Record device model/serial, DLL version, and OS version.
- Log directory: `_logs/hw/YYYY-MM-DD/`
  - `env.txt`: OS, device model/serial, DLL version
  - `steps.md`: executed commands and step order
  - `result.log`: full stdout/stderr capture
- Evidence zip: `_logs/hw/YYYY-MM-DD/hw-evidence.zip`
- `hw-evidence.zip` and `_logs/` are not committed; keep for sharing/archival only.

### 证据模板
以下模板用于整理测试证据，便于复现与问题上报：

`env.txt`:
```text
date: YYYY-MM-DD
os: Windows 10 10.0.19045
device: <model>
serial: <serial>
dll_version: <dll_version> (x64)
lib_version: <lib_version>
features: <enabled_features>
```

`steps.md`:
```markdown
1. Connect device and verify status in TSMaster (if applicable)
2. Run: cargo run -p canlink-tscan --example <example> -- <args>
3. Record key observations and timestamps
```

`result.log`:
```text
# Capture full stdout/stderr
# PowerShell example:
cargo run -p canlink-tscan --example <example> -- <args> *> result.log
```

证据包 `hw-evidence.zip` 建议包含：
- env.txt
- steps.md
- result.log
- 相关截图（如设备连接状态或错误提示）

### 样例输出（通过/失败）

注：如文档平台不支持 Markdown 自动锚点，可改用显式 HTML 锚点并验证可用性。

| 模块 | 通过样例 | 失败样例 | 关键判定点 |
| --- | --- | --- | --- |
| Module 1 | [Module 1 PASS](#module-1-pass) | [Module 1 FAIL](#module-1-fail) | 设备枚举>0，open/close 无错误 |
| Module 2 | [Module 2 PASS](#module-2-pass) | [Module 2 FAIL](#module-2-fail) | send_count=recv_count，ID 匹配 |
| Module 3 | [Module 3 PASS](#module-3-pass) | [Module 3 FAIL](#module-3-fail) | 过滤生效且可移除 |
| Module 4 | [Module 4 PASS](#module-4-pass) | [Module 4 FAIL](#module-4-fail) | period=100ms，expected=10±2 |
| Module 5 | [Module 5 PASS](#module-5-pass) | [Module 5 FAIL](#module-5-fail) | ISO-TP 完整接收，无超时 |
| Module 6 | [Module 6 PASS](#module-6-pass) | [Module 6 FAIL](#module-6-fail) | 可断开并重连恢复收发 |

#### Module 1 PASS
```text
time=2026-03-17T10:00:01+08:00 module=1 step=scan result=PASS detail="found=1"
time=2026-03-17T10:00:02+08:00 module=1 step=open result=PASS detail="handle=0x1234ABCD"
time=2026-03-17T10:00:03+08:00 module=1 step=get_info result=PASS detail="model=TSMaster Mini"
time=2026-03-17T10:00:04+08:00 module=1 step=get_info result=PASS detail="serial=<device-serial>"
time=2026-03-17T10:00:05+08:00 module=1 step=close result=PASS detail="error=0"
time=2026-03-17T10:00:06+08:00 module=1 step=summary result=PASS detail="open_close_ok"
```

#### Module 1 FAIL
```text
time=2026-03-17T10:01:01+08:00 module=1 step=scan result=PASS detail="found=1"
time=2026-03-17T10:01:02+08:00 module=1 step=open result=FAIL reason="open_error=E_ACCESS"
time=2026-03-17T10:01:03+08:00 module=1 step=get_info result=FAIL reason="handle_invalid"
time=2026-03-17T10:01:04+08:00 module=1 step=close result=FAIL reason="not_opened"
time=2026-03-17T10:01:05+08:00 module=1 step=finalize result=PASS detail="library_finalized"
time=2026-03-17T10:01:06+08:00 module=1 step=summary result=FAIL reason="open_failed"
```

#### Module 2 PASS
```text
time=2026-03-17T10:02:01+08:00 module=2 step=configure result=PASS detail="ch=0 bitrate=500k"
time=2026-03-17T10:02:02+08:00 module=2 step=send result=PASS detail="id=0x123 dlc=8"
time=2026-03-17T10:02:03+08:00 module=2 step=receive result=PASS detail="id=0x123 count=1"
time=2026-03-17T10:02:04+08:00 module=2 step=stats result=PASS detail="send_count=1 recv_count=1"
time=2026-03-17T10:02:05+08:00 module=2 step=loopback result=PASS detail="match=true"
time=2026-03-17T10:02:06+08:00 module=2 step=summary result=PASS detail="send_recv_ok"
```

#### Module 2 FAIL
```text
time=2026-03-17T10:03:01+08:00 module=2 step=configure result=PASS detail="ch=0 bitrate=500k"
time=2026-03-17T10:03:02+08:00 module=2 step=send result=PASS detail="id=0x123 dlc=8"
time=2026-03-17T10:03:03+08:00 module=2 step=receive result=FAIL reason="timeout_ms=1000"
time=2026-03-17T10:03:04+08:00 module=2 step=stats result=FAIL reason="recv_count=0"
time=2026-03-17T10:03:05+08:00 module=2 step=loopback result=FAIL reason="id_mismatch"
time=2026-03-17T10:03:06+08:00 module=2 step=summary result=FAIL reason="no_rx"
```

#### Module 3 PASS
```text
time=2026-03-17T10:04:01+08:00 module=3 step=add_filter result=PASS detail="id=0x456"
time=2026-03-17T10:04:02+08:00 module=3 step=send result=PASS detail="id=0x456"
time=2026-03-17T10:04:03+08:00 module=3 step=send result=PASS detail="id=0x111"
time=2026-03-17T10:04:04+08:00 module=3 step=receive result=PASS detail="id=0x456 count=1"
time=2026-03-17T10:04:05+08:00 module=3 step=remove_filter result=PASS detail="id=0x456"
time=2026-03-17T10:04:06+08:00 module=3 step=receive result=PASS detail="id=0x111 count=1"
time=2026-03-17T10:04:07+08:00 module=3 step=summary result=PASS detail="filter_ok"
```

#### Module 3 FAIL
```text
time=2026-03-17T10:05:01+08:00 module=3 step=add_filter result=PASS detail="id=0x456"
time=2026-03-17T10:05:02+08:00 module=3 step=send result=PASS detail="id=0x456"
time=2026-03-17T10:05:03+08:00 module=3 step=send result=PASS detail="id=0x111"
time=2026-03-17T10:05:04+08:00 module=3 step=receive result=FAIL reason="unexpected_id=0x111"
time=2026-03-17T10:05:05+08:00 module=3 step=remove_filter result=PASS detail="id=0x456"
time=2026-03-17T10:05:06+08:00 module=3 step=summary result=FAIL reason="filter_not_effective"
```

#### Module 4 PASS
```text
time=2026-03-17T10:06:01+08:00 module=4 step=configure result=PASS detail="period=100ms duration=1s"
time=2026-03-17T10:06:02+08:00 module=4 step=start_periodic result=PASS detail="id=0x321"
time=2026-03-17T10:06:03+08:00 module=4 step=stats result=PASS detail="expected=10 tolerance=2 actual=10"
time=2026-03-17T10:06:04+08:00 module=4 step=jitter result=PASS detail="max_jitter_ms=5"
time=2026-03-17T10:06:05+08:00 module=4 step=stop_periodic result=PASS detail="ok"
time=2026-03-17T10:06:06+08:00 module=4 step=summary result=PASS detail="within_tolerance"
```

#### Module 4 FAIL
```text
time=2026-03-17T10:07:01+08:00 module=4 step=configure result=PASS detail="period=100ms duration=1s"
time=2026-03-17T10:07:02+08:00 module=4 step=start_periodic result=PASS detail="id=0x321"
time=2026-03-17T10:07:03+08:00 module=4 step=stats result=FAIL reason="expected=10 tolerance=2 actual=2"
time=2026-03-17T10:07:04+08:00 module=4 step=jitter result=FAIL reason="max_jitter_ms=80"
time=2026-03-17T10:07:05+08:00 module=4 step=stop_periodic result=PASS detail="ok"
time=2026-03-17T10:07:06+08:00 module=4 step=summary result=FAIL reason="out_of_tolerance"
```

#### Module 5 PASS
```text
time=2026-03-17T10:08:01+08:00 module=5 step=isotp_send result=PASS detail="tx_len=64 frames=5"
time=2026-03-17T10:08:02+08:00 module=5 step=isotp_receive result=PASS detail="rx_len=64"
time=2026-03-17T10:08:03+08:00 module=5 step=verify result=PASS detail="crc_ok=true"
time=2026-03-17T10:08:04+08:00 module=5 step=timeout_config result=PASS detail="timeout_ms=1000"
time=2026-03-17T10:08:05+08:00 module=5 step=stats result=PASS detail="rx_frames=5"
time=2026-03-17T10:08:06+08:00 module=5 step=summary result=PASS detail="isotp_ok"
```

#### Module 5 FAIL
```text
time=2026-03-17T10:09:01+08:00 module=5 step=isotp_send result=PASS detail="tx_len=64 frames=5"
time=2026-03-17T10:09:02+08:00 module=5 step=isotp_receive result=FAIL reason="timeout_ms=1000"
time=2026-03-17T10:09:03+08:00 module=5 step=verify result=FAIL reason="rx_len=0"
time=2026-03-17T10:09:04+08:00 module=5 step=stats result=FAIL reason="rx_frames=0"
time=2026-03-17T10:09:05+08:00 module=5 step=cleanup result=PASS detail="ok"
time=2026-03-17T10:09:06+08:00 module=5 step=summary result=FAIL reason="isotp_timeout"
```

#### Module 6 PASS
```text
time=2026-03-17T10:10:01+08:00 module=6 step=disconnect result=PASS detail="handle=0x1234ABCD"
time=2026-03-17T10:10:02+08:00 module=6 step=wait result=PASS detail="delay_ms=300"
time=2026-03-17T10:10:03+08:00 module=6 step=reopen result=PASS detail="handle=0x2234ABCD"
time=2026-03-17T10:10:04+08:00 module=6 step=send result=PASS detail="id=0x123"
time=2026-03-17T10:10:05+08:00 module=6 step=receive result=PASS detail="count=1"
time=2026-03-17T10:10:06+08:00 module=6 step=summary result=PASS detail="reconnect_ok"
```

#### Module 6 FAIL
```text
time=2026-03-17T10:11:01+08:00 module=6 step=disconnect result=PASS detail="handle=0x1234ABCD"
time=2026-03-17T10:11:02+08:00 module=6 step=wait result=PASS detail="delay_ms=300"
time=2026-03-17T10:11:03+08:00 module=6 step=reopen result=FAIL reason="error=E_DEVICE_BUSY"
time=2026-03-17T10:11:04+08:00 module=6 step=send result=FAIL reason="not_opened"
time=2026-03-17T10:11:05+08:00 module=6 step=receive result=FAIL reason="not_opened"
time=2026-03-17T10:11:06+08:00 module=6 step=summary result=FAIL reason="reconnect_failed"
```

### Module 1: Device Discovery / Open / Close
- Goal: device can be discovered, opened, and closed.
- Pass: device enumerated, open succeeds, close returns no error.

### Module 2: Loopback Send / Receive
- Goal: basic send/receive path works.
- Pass: sent message is received (or receive stats match expectation).

### Module 3: Filter Add / Remove
- Goal: filters take effect and can be removed.
- Pass: only matching messages received after add; default behavior restored after remove.

### Module 4: Periodic Send
- Goal: periodic scheduler runs at interval; stats are usable.
- Pass: send_count aligns with interval (allow minor jitter).

### Module 5: ISO-TP Send / Receive
- Goal: multi-frame ISO-TP send/receive works.
- Pass: multi-frame send succeeds; receive data complete; no unexpected timeouts.

### Module 6: Disconnect / Reconnect
- Goal: reconnect works after disconnect.
- Pass: device reopens and messaging works again.
