# TSCan 断开卡死隔离规避设计（daemon + JSON 协议）

**日期**：2026-03-18
**范围**：`canlink-tscan`（Windows + LibTSCAN）
**状态**：设计稿

## 背景与问题
LibTSCAN 在 Windows 下调用 `tscan_disconnect_by_handle()` / `tscan_disconnect_all_devices()` 会随机卡死（Rust 与 Python ctypes 皆可复现），导致主进程挂起。该问题在 `libTSCAN.dll` 旧版（2024.8.12.1163）与新版（2025.12.2.1746）均存在，确定为厂商 DLL 层问题。

**结论**：需要通过进程隔离规避，避免主进程被卡死拖垮。该方案属于厂商 Bug 规避措施，厂商修复后将考虑移除/降级此机制。

## 目标
- 默认启用隔离，用户无感使用（Rust 开发者为主要用户）。
- 断开必须“真正成功”，以“设备可被重新枚举并允许重新连接”为成功标准。
- 卡死仅影响子进程，不影响主进程稳定性。
- 提供配置开关，用户可显式关闭。
- 清晰文档说明“厂商 Bug 规避，后续修复会移除”。

## 非目标
- 不建立通用隔离框架，仅针对 TSCan/LibTSCAN。
- 不提供跨语言统一 IPC SDK。
- 不承诺完全零停顿；极端情况下可能出现短暂重连延迟。

## 总体方案
- 新增独立子进程可执行文件：`canlink-tscan-daemon`（随源码构建）。
- `canlink-tscan` 默认使用隔离后端：主进程作为 IPC 客户端，子进程内部调用 LibTSCAN。
- 当断开或关键 API 调用超时/卡死时，主进程杀死子进程并重启，重连成功即视为断开成功。

## 架构与组件
### 主进程（`canlink-tscan`）
- 负责拉起、监控并在必要时重启 `canlink-tscan-daemon`。
- 通过 stdio 与子进程通信，转发现有 `CanBackend` API 调用。
- v1 采用单飞模型：一次只发送一个请求，等待响应后再发送下一条。
- 默认每个 `TSCanBackend` 实例独立 daemon，不共享进程。

### 子进程（`canlink-tscan-daemon`）
- 独立进程内直接调用 LibTSCAN。
- 实现 IPC 服务端：接收请求、执行、返回结果。
- 仅做最小错误判断与参数校验，复杂恢复逻辑由主进程负责。
- `stdin` 关闭后自动退出，避免孤儿进程。

## 配置与启用方式
读取优先级（高 → 低）：
1. `BackendConfig` 参数（代码或 `canlink.toml` 方式传入）
2. 项目工作目录 `canlink-tscan.toml`
3. 内置默认值

推荐配置项（`canlink-tscan.toml` 顶层）：
- `use_daemon = true`
- `daemon_path = "..."`（可选）
- `request_timeout_ms = 2000`
- `disconnect_timeout_ms = 3000`
- `restart_max_retries = 3`
- `recv_timeout_ms = 0`（与当前 `receive_message()` 非阻塞语义一致）

不新增环境变量开关，避免配置分散。

## daemon 路径解析
- 若 `BackendConfig` 或 `canlink-tscan.toml` 显式配置了 `daemon_path`，**优先使用该路径**（显式覆盖默认发现）。
- 未配置时按默认顺序寻找：
  1. 与主程序同目录 `canlink-tscan-daemon.exe`
  2. `PATH` 中的可执行文件

## IPC 协议（v1）
### 传输方式
- `stdin/stdout`，**长度前缀 JSON**：`[u32 little-endian length][utf8 json]`。
- `stdout` **仅**用于协议帧输出；日志写 `stderr`。
- 单帧最大 1 MiB，超过视为协议错误。

### 请求/响应结构
请求（示例）：
```json
{"id":1,"op":"SCAN","params":{}}
```
响应（示例）：
```json
{"id":1,"status":"ok","code":0,"message":"","data":{...}}
```
字段说明：
- `id`：请求 ID（递增）
- `op`：操作名（字符串）
- `params`：参数对象
- `status`：`ok` / `error`
- `code`：错误码（0 表示成功）
- `message`：错误说明（UTF-8）
- `data`：返回数据对象（成功时）

### 协议版本与兼容
- `HELLO`/`HELLO_ACK` 必含 `protocol_version`。
- 主进程与 daemon 版本不一致时，主进程**立即失败并报错**，不继续执行。

### 操作清单与 schema（v1）
通用约定：
- `serial` 为空字符串表示“使用扫描到的第一个设备”。
- `data` 中数组元素字段均为显式命名字段。

| op | params | data |
| --- | --- | --- |
| `HELLO` | `{ "protocol_version":1, "client_version":"x" }` | `{ "protocol_version":1, "daemon_version":"y" }` |
| `INIT_LIB` | `{ "enable_fifo":true, "enable_error_frame":false, "use_hw_time":true }` | `{}` |
| `SCAN` | `{}` | `{ "devices":[{"manufacturer":"...","product":"...","serial":"...","device_type":0}] }` |
| `GET_DEVICE_INFO` | `{ "index":0 }` | `{ "manufacturer":"...","product":"...","serial":"...","device_type":0 }` |
| `CONNECT` | `{ "serial":"" }` | `{ "handle":123, "channel_count":1, "supports_canfd":true, "serial":"..." }` |
| `DISCONNECT_BY_HANDLE` | `{ "handle":123 }` | `{}` |
| `DISCONNECT_ALL` | `{}` | `{}` |
| `OPEN_CHANNEL` | `{ "handle":123, "channel":0 }` | `{}` |
| `CLOSE_CHANNEL` | `{ "handle":123, "channel":0 }` | `{}` |
| `CONFIG_CAN_BAUDRATE` | `{ "handle":123, "channel":0, "rate_kbps":500.0, "term":1 }` | `{}` |
| `CONFIG_CANFD_BAUDRATE` | `{ "handle":123, "channel":0, "arb_kbps":500.0, "data_kbps":2000.0, "ctrl_type":0, "ctrl_mode":0, "term":1 }` | `{}` |
| `SEND_CAN` | `{ "handle":123, "channel":0, "id":256, "is_ext":false, "data":[1,2,3,4] }` | `{}` |
| `SEND_CANFD` | `{ "handle":123, "channel":0, "id":256, "is_ext":false, "brs":true, "esi":false, "data":[1,2,3] }` | `{}` |
| `RECV_CAN` | `{ "handle":123, "channel":0, "max_count":1, "timeout_ms":0 }` | `{ "messages":[{"id":256,"is_ext":false,"data":[1,2]}] }` |
| `RECV_CANFD` | `{ "handle":123, "channel":0, "max_count":1, "timeout_ms":0 }` | `{ "messages":[{"id":256,"is_ext":false,"brs":true,"esi":false,"data":[1,2]}] }` |
| `GET_CAPABILITY` | `{ "handle":123 }` | `{ "channel_count":1, "supports_canfd":true, "max_bitrate_kbps":1000, "supported_bitrates_kbps":[125,250,500,1000] }` |
| `FINALIZE` | `{}` | `{}` |

### RECV 行为
- v1 设计为非阻塞，默认 `timeout_ms=0`、`max_count=1`。
- `timeout_ms` 到期返回空数组 `messages:[]`，不视为错误。
 - 主进程仍有 I/O 层硬超时保护（与 `request_timeout_ms` 同级），仅用于检测 daemon 卡死；若触发则按“超时重启”路径处理。

### 协议异常处理（主进程）
- EOF / daemon 退出：视为异常退出，触发重启。
- 非法 JSON / 长度不匹配 / 超过最大帧：视为协议错误，记录日志并触发重启。
- stdout 被污染（无法解析为协议帧）：视为协议错误并重启。
- 重启超过 `restart_max_retries`：返回 `CanError::InitializationFailed`。

## 超时与重启策略
- 非 `RECV` 请求使用 `request_timeout_ms`（默认 2000ms）。
- `DISCONNECT_*` 使用 `disconnect_timeout_ms`（默认 3000ms）。
- `RECV_*` 由 daemon 内部使用 `timeout_ms`（默认 `recv_timeout_ms`）。
- 超时即判定 daemon 不健康：主进程杀进程并重启（最多 `restart_max_retries` 次）。
- 若重启仍失败，返回 `CanError::InitializationFailed`。
- `FINALIZE` 超时：直接 kill，不再重启（保证关闭流程结束）。

### 重启后的状态恢复
- 主进程缓存以下状态：
  - `serial`（目标设备序列号）
  - `channel_count` / `supports_canfd`
  - 已配置的波特率参数（CAN/CANFD）
  - 已打开的通道集合
- daemon 重启后执行：
  1. `HELLO` / `INIT_LIB`
  2. `SCAN` 并尝试 `CONNECT` 同一 `serial`（若 `serial` 为空则连接列表首项并缓存实际 `serial`）
  3. 重新下发 `CONFIG_*` 与 `OPEN_CHANNEL`
- 若恢复失败，返回 `CanError::InitializationFailed`。
- 当前触发超时的请求默认返回错误；仅对幂等请求（`SCAN` / `GET_DEVICE_INFO` / `GET_CAPABILITY`）可在恢复成功后**重试一次**。

## 断开语义
- 成功标准：设备被真正释放，可重新枚举并允许重新连接。
- 正常路径：`DISCONNECT_*` 返回成功即可；不额外验证。
- 超时路径：
  1. 杀 daemon 并重启
  2. `SCAN` + `CONNECT`（同 `serial`）验证设备可重新连接
  3. **立即终止 daemon（kill）** 释放设备
  4. 若 `CONNECT` 返回 `already_connected` / `device_busy` 或失败，则视为断开未完成并返回错误
 - 断开完成后主进程不保留连接状态；后续如需使用需重新 `initialize()`。

## 错误映射（摘要）
### 统一错误码（响应 `code`）
- `0`: OK
- `1`: libtscan_error（DLL 返回非 0）
- `2`: invalid_params
- `3`: already_connected
- `4`: invalid_handle
- `5`: invalid_channel
- `6`: no_device
- `7`: invalid_index
- `8`: device_busy
- `9`: protocol_error

### 映射到 `CanError`
- `libtscan_error` → `CanError::HardwareError`（附带错误码/文本）
- `invalid_*` → `CanError::InvalidState` / `CanError::ChannelNotFound`
- `no_device` → `CanError::DeviceNotFound`
- `device_busy` / `already_connected` → `CanError::InitializationFailed`
- 协议/超时导致重启失败 → `CanError::InitializationFailed`
 - daemon 启动失败 / `HELLO` 版本不匹配 → `CanError::InitializationFailed`

## 测试与回归
- 单元测试：
  - 配置解析优先级
  - 协议帧编解码
  - daemon 路径解析
- 集成测试（无硬件）：
  - 使用 stub daemon 验证握手/超时/重启/错误映射
- 硬件回归：
  - 复用现有硬件测试流程，补充断开超时规避说明

## 交付与说明
- 文档明确“厂商 Bug 规避，后续修复会移除/降级”。
- 默认启用，可通过 `use_daemon=false` 关闭。
