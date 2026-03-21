# TSCan 断开卡死隔离规避设计（厂商 Bug Workaround）

**日期**：2026-03-14
**范围**：`canlink-tscan`（Windows + LibTSCAN）
**状态**：设计稿

## 背景与问题
LibTSCAN 在 Windows 下调用 `tscan_disconnect_by_handle()` / `tscan_disconnect_all_devices()` 会随机卡死（Rust 与 Python ctypes 皆可复现），导致主进程挂起。该问题在 `libTSCAN.dll` 旧版（2024.8.12.1163）与新版（2025.12.2.1746）均存在，确定为厂商 DLL 层问题。

**结论**：需要通过进程隔离进行规避，避免主进程被卡死拖垮。该方案属于厂商 Bug 规避措施，厂商修复后会发布新版并考虑移除/降级此机制。

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

## 高层方案概述
- 新增独立子进程可执行文件：`canlink-tscan-daemon`（随 `canlink-tscan` 源码构建）。
- `canlink-tscan` 默认使用隔离后端：主进程作为 IPC 客户端，子进程内部调用 LibTSCAN。
- 当断开或关键 API 调用超时/卡死时，主进程杀死子进程并重启，重连成功即视为断开成功。
- 配置文件用于开关与参数配置；无配置时仍默认启用。

## 架构与组件
### 1) 主进程（`canlink-tscan`）
- 提供新的“隔离后端”实现（内部封装）。
- 负责拉起、监控并在必要时重启 `canlink-tscan-daemon`。
- 通过 stdio 与子进程通信，转发现有 `CanBackend` API 调用。
  - 同进程内多个 `TSCanBackend` 实例：v1 **不支持共享 daemon**，默认每实例独立 daemon；若多实例竞争同一设备导致失败，返回 `already_connected`。

### 2) 子进程（`canlink-tscan-daemon`）
- 独立进程内直接调用 LibTSCAN。
- 实现 IPC 服务端：接收请求、执行、返回结果。
- 子进程自身不做复杂重连逻辑，故障由主进程负责重启。
  - daemon 退出策略：检测 `stdin` 关闭后自动退出，避免孤儿进程。

### 3) 设备范围（v1）
- v1 仅支持**单设备/单 handle**（与当前 `TSCanBackend` 行为一致）。
- 多设备支持不在本次范围内，后续如需扩展再设计协议与状态管理。
  - `CONNECT` 重复调用：返回 `status_code = 3`（already_connected）。
  - `OPEN_CHANNEL` 使用无效 handle：返回 `status_code = 4`（invalid_handle）。
  - `CONNECT` 为空 serial 时，选择 `SCAN` 列表中的第一个设备（index=0），并记录日志。
  - `CONNECT` 为空 serial 且 `SCAN=0`：返回 `status_code = 6`（no_device）。

## IPC 设计
### 传输方式
- **stdio**（跨平台最稳定，便于后续扩展到 Linux/macOS）。
  - `stdout` **仅**用于 IPC 帧输出；所有日志必须写入 `stderr` 或文件。
  - 如检测到非协议帧（非法长度/未知类型/解码失败），主进程视为协议污染并重启 daemon。
  - daemon 崩溃/退出由主进程检测并触发重启；重启次数计入日志。

### 握手与协议校验
- daemon 仅在完成 `HELLO` 握手后接受其它请求。
- 若在握手前收到其它请求，daemon 直接退出并由主进程重启。

### 编码格式
- 二进制长度前缀帧（避免 JSON 开销）。
- 每条消息结构（小端序）：`[u32 length][u64 request_id][u8 msg_type][payload...]`。
- `request_id` 用于请求/响应关联与乱序检测。
 - `bool` 编码为 `u8`（0/1），`f64` 为 IEEE-754 小端序。

### 并发模型与顺序保证
- v1 默认 **单飞模型**：主进程在收到上一条响应前不会发送下一条请求。
- 该模型避免并发错配问题，降低实现复杂度。
- 若未来需要并发，可基于 `request_id` 扩展为多路复用。
- `CanBackend` 现有接口要求外部同步（`&mut self`），因此请求序列化不会改变现有并发语义。

### request_id 异常处理
- 若响应 `request_id` 与当前请求不匹配，视为协议错误并重启 daemon。
- 若收到重复响应（相同 `request_id`），丢弃并记录警告。
- daemon 重启后产生的迟到响应一律丢弃。
 - RECV 超时不会产生迟到响应（daemon 仅在请求内返回一次）。

### 帧边界与安全限制
- `length` 为后续负载总长度（含 `request_id + msg_type + payload`）。
- 最小帧长度为 9 字节（`request_id + msg_type`），小于该值视为协议错误并重启 daemon。
- 设定最大帧大小 **1 MiB**（固定常量），超过则视为协议错误并重启 daemon。
- 读写采用 `read_exact` / `write_all`，确保处理 partial read/write。
- 若写入阻塞或读取超时，视为子进程不健康并触发重启。
- 若 payload 长度不足/多余或字符串长度不匹配：daemon 返回 `status_code=2`（invalid_length）；若解码失败则退出并由主进程重启。

### I/O 超时实现约束（Windows）
- 主进程与 daemon 的 stdio 读写应放在独立 I/O 线程，主线程通过 `mpsc`/channel 等方式等待并实现超时。
- 超时由主线程控制，若超时则杀死 daemon 并重启；避免在主线程直接阻塞 `read_exact`。
- 主进程需持续 drain daemon `stderr`，避免管道缓冲区填满导致阻塞。
- 重启时关闭旧管道句柄以触发 I/O 线程退出，确保线程可回收。
 - 对 `RECV_*`，I/O 线程以请求超时返回，不触发重启；重启仅用于非 RECV 的 I/O 超时。

### 超时/重启策略表（v1）
| 调用 | 超时来源 | 超时后是否重启 | 自动重试 |
| --- | --- | --- | --- |
| `RECV_*` | `timeout_ms`/`recv_timeout_ms` | 否 | 否 |
| `DISCONNECT_*` | `disconnect_timeout_ms` | 是 | 重启后按断开语义 |
| 其它请求 | `request_timeout_ms` | 是 | 仅幂等请求重试一次 |

### 性能策略（v1）
- v1 不强制批量帧，先以稳定为主；必要时可在协议中新增批量消息类型。

### RECV 行为（避免长阻塞）
- `RECV_*` 设计为**非阻塞**：默认 `timeout_ms=0`、`max_count=1`，若无数据立即返回 `count=0`。
- 如需等待，可在请求中传入 `timeout_ms`（默认来自配置 `recv_timeout_ms`），但 v1 建议保持短超时，避免阻塞其它请求。
- 该行为与当前 `TSCanBackend::receive_message` 的“尽快返回”语义一致，不改变外部行为。
- `timeout_ms` 到期：响应 `status_code=0` 且 `count=0`（不返回超时错误）。
 - RECV 超时仅由 daemon 内部处理，不触发主进程超时与重启。

### 超时优先级与裁剪
- 对 `RECV_*`：优先使用请求内 `timeout_ms`；若未提供则使用配置 `recv_timeout_ms`；不受 `request_timeout_ms` 限制。
- 对非 `RECV_*`：使用 `request_timeout_ms`；`disconnect` 可被 `disconnect_timeout_ms` 覆盖。
- 若传入超时大于 60s，裁剪到 60s 以避免长时间阻塞导致误判。

### 消息类型（初版）
- 初始化与握手：`HELLO` / `HELLO_ACK`（含协议版本）
- 设备生命周期：`SCAN` / `GET_DEVICE_INFO` / `CONNECT` / `DISCONNECT_BY_HANDLE` / `DISCONNECT_ALL`
- 通道管理：`OPEN_CHANNEL` / `CLOSE_CHANNEL`
- 消息收发：`SEND_CAN` / `SEND_CANFD` / `RECV_CAN` / `RECV_CANFD`
- 能力查询：`GET_CAPABILITY`
- 关闭：`FINALIZE`
- 通道配置（用于重建状态）：`CONFIG_CAN_BAUDRATE` / `CONFIG_CANFD_BAUDRATE`
- 初始化：`INIT_LIB`（对应 `initialize_lib_tscan`）

> 说明：仅覆盖当前 `TSCanBackend` 所需操作，未来按需扩展。

### 启动/状态机（简版）
1. `HELLO` 握手
2. `INIT_LIB`
3. `SCAN/CONNECT`
4. `CONFIG_*`（如需）
5. `OPEN_CHANNEL`

### 消息 schema 与返回（v1）
通用约定：
- 字符串使用 UTF-8，格式为 `[u32 len][bytes...]`。
- 响应帧使用 `msg_type | 0x80`。
- 响应 payload 前置：`[u32 status_code][u32 error_len][error_bytes...]`。
- `status_code=0` 表示成功，错误字段为空。

核心消息（请求 payload → 响应 payload）：
- `HELLO`：`protocol_version(u16) + client_version(str)` → `HELLO_ACK: protocol_version(u16) + daemon_version(str)`
- `INIT_LIB`：`enable_fifo(bool) + enable_error_frame(bool) + use_hw_time(bool)` → 无额外 payload
- `SCAN`：无 → `device_count(u32) + repeated { manufacturer(str), product(str), serial(str) }`
- `GET_DEVICE_INFO`：`index(u32)` → `manufacturer(str) + product(str) + serial(str) + device_type(i32)`
  - `index` 超出范围：返回 `status_code = 7`（invalid_index）。
- `CONNECT`：`serial(str, empty=default)` → `handle(u64) + channel_count(u8) + supports_canfd(bool)`
  - 若 `serial` 为空，daemon 会返回实际连接的 serial，并由主进程缓存用于重启校验。
- `DISCONNECT_BY_HANDLE`：`handle(u64)` → 无额外 payload
- `DISCONNECT_ALL`：无 → 无额外 payload
- `OPEN_CHANNEL`：`handle(u64) + channel(u8)` → 无额外 payload
  - v1 与现有实现一致：默认 500 kbps、终端电阻启用。
  - 若已执行 `CONFIG_CAN_*`/`CONFIG_CANFD_*`，则 `OPEN_CHANNEL` 不再覆盖配置。
- `CLOSE_CHANNEL`：`handle(u64) + channel(u8)` → 无额外 payload
- `CONFIG_CAN_BAUDRATE`：`handle(u64) + channel(u8) + rate_kbps(f64) + term(u8)` → 无额外 payload
- `CONFIG_CANFD_BAUDRATE`：`handle(u64) + channel(u8) + arb_kbps(f64) + data_kbps(f64) + ctrl_type(u8) + ctrl_mode(u8) + term(u8)` → 无额外 payload
- `SEND_CAN`：`handle(u64) + channel(u8) + id(u32) + is_ext(bool) + data_len(u8) + data[0..8]` → 无额外 payload
- `SEND_CANFD`：`handle(u64) + channel(u8) + id(u32) + is_ext(bool) + brs(bool) + esi(bool) + dlc(u8) + data[0..64]` → 无额外 payload
- `RECV_CAN`：`handle(u64) + channel(u8) + max_count(u8) + timeout_ms(u32)` → `count(u8) + repeated CAN message`
- `RECV_CANFD`：`handle(u64) + channel(u8) + max_count(u8) + timeout_ms(u32)` → `count(u8) + repeated CANFD message`
- `GET_CAPABILITY`：`handle(u64)` → `channel_count(u8) + supports_canfd(bool) + max_bitrate_kbps(u32) + supported_count(u32) + supported_bitrates_kbps[u32]`
  - `supported_count` 上限为 32，超过则裁剪为 32 并记录警告。
- `FINALIZE`：无 → 无额外 payload

- CAN message 编码（v1）：`id(u32) + is_ext(bool) + dlc(u8) + data[0..8]`（不含时间戳）
- CANFD message 编码（v1）：`id(u32) + is_ext(bool) + brs(bool) + esi(bool) + dlc(u8) + data[0..64]`
  - `dlc` 与数据长度映射遵循 CAN FD 标准（0-8 -> N bytes, 9-15 -> 12/16/20/24/32/48/64）。

数据长度校验：
- `data_len`/`dlc` 必须与实际 payload 长度一致，否则返回 `status_code = 2`，错误信息为 `invalid_length`。
- `max_count` 上限为 32，超过则裁剪为 32 并记录警告。

### 错误映射（v1）
- `status_code != 0`：默认映射为 `CanError::HardwareError`，并附带 `error_text`。
- 初始化/连接阶段失败：映射为 `CanError::InitializationFailed`（包含 `error_text`）。
- 参数合法性（如 channel 越界）由主进程先行校验，尽量避免无效 IPC 调用。
- 主进程超时/无响应：若未触发重启则映射为 `CanError::Timeout`；若触发重启则映射为 `CanError::InitializationFailed`。
- daemon 重启导致请求失败：映射为 `CanError::InitializationFailed`（原因包含 `daemon restarted`）。

### status_code → CanError 映射补充
- `invalid_length`：映射为 `CanError::InvalidState`。
- `invalid_handle`：映射为 `CanError::InvalidState`。
- `invalid_channel`：映射为 `CanError::ChannelNotFound`。
- `already_connected`：映射为 `CanError::InvalidState`。
- `no_device`：映射为 `CanError::DeviceNotFound`。
- `invalid_index`：映射为 `CanError::InvalidState`。
- `device_busy`：映射为 `CanError::InitializationFailed`。

### status_code 枚举（v1）
- `0`: OK
- `1`: libtscan_error（来自 DLL 返回值）
- `2`: invalid_length
- `3`: already_connected
- `4`: invalid_handle
- `5`: invalid_channel
- `6`: no_device
- `7`: invalid_index
- `8`: device_busy
 - `7`: invalid_index

### 版本兼容
- `HELLO_ACK` 返回协议版本与 `canlink-tscan-daemon` 版本。
- **协议版本必须完全一致**，否则主进程拒绝继续并给出明确错误提示。
- daemon 版本字符串仅用于日志与排查，不作为兼容判定依据。

### 协议常量（v1）
- `protocol_version = 1`
- `msg_type` 取值范围 `1..=127`（响应为 `msg_type | 0x80`）
  - `0x01 HELLO`
  - `0x02 SCAN`
  - `0x03 GET_DEVICE_INFO`
  - `0x04 CONNECT`
  - `0x05 DISCONNECT_BY_HANDLE`
  - `0x06 DISCONNECT_ALL`
  - `0x07 OPEN_CHANNEL`
  - `0x08 CLOSE_CHANNEL`
  - `0x09 SEND_CAN`
  - `0x0A SEND_CANFD`
  - `0x0B RECV_CAN`
  - `0x0C RECV_CANFD`
  - `0x0D GET_CAPABILITY`
  - `0x0E FINALIZE`
  - `0x0F CONFIG_CAN_BAUDRATE`
  - `0x10 CONFIG_CANFD_BAUDRATE`
  - `0x11 INIT_LIB`

### FINALIZE 语义
- 主进程在 `close()` 中发送 `FINALIZE` 并等待确认（受 `request_timeout_ms` 约束）。
- daemon 收到 `FINALIZE` 后清理资源并主动退出。
- 若 daemon 在 `FINALIZE` 前异常退出，主进程视为非正常退出并记录警告。
- 若 `FINALIZE` 超时，主进程强制结束 daemon，不再重启（关闭流程结束）。

## 生命周期与恢复策略
1. **启动**：主进程启动或首次使用时拉起 daemon。
2. **握手**：等待 `HELLO_ACK`，超时则重启子进程。
3. **运行**：正常转发 API 请求。
4. **卡死检测**：关键调用（尤其是 disconnect）设置超时阈值。
5. **恢复**：超时则 kill 子进程并重启，重新初始化并重连。
6. **成功判据**：重连成功视为断开成功。

### 状态重建（重启后的恢复）
主进程维护“期望状态”，daemon 重启后按以下顺序重建（**仅在需要继续运行时**）：
1. 重新 `initialize` + `scan/connect`。
2. 重新配置通道（波特率/FD 模式等）。
3. 重新打开通道。
4. 重新注册/恢复过滤器或其他可配置状态（若后端支持）。

> 若某状态无法恢复（例如硬件状态已变更），返回明确错误并提示用户重试。

### 状态快照（v1）
- 必须恢复：已连接设备、已打开通道、通道波特率/FD 配置。
- 可选恢复：过滤器、缓冲清理策略（如后端支持）。
- daemon 重启后主进程会更新内部 handle 为新连接返回值，并覆盖旧 handle。

## 失败语义与对外行为
- 若重启/重连成功：对上层尽量透明，记录警告日志。
- 若多次重启仍失败：返回 `CanError::InitializationFailed`，包含重试次数与原因。
- 对上层调用：保持现有 API 行为，不新增破坏性接口。

### In-flight 请求处理
- 子进程被杀死时，所有正在等待的请求立即返回错误（`CanError::InitializationFailed`，原因包含“daemon restarted”）。
- 对于**幂等请求**（如 `GET_CAPABILITY` / `SCAN` / `GET_DEVICE_INFO`），允许在重启成功后自动重试一次。
- 对于**非幂等请求**（如 `SEND_*`），不自动重试，直接返回错误。
- 若协议污染触发重启，当前请求返回 `CanError::InitializationFailed`，不自动重试。
- 幂等请求的“自动重试一次”不消耗额外的 `restart_max_retries` 计数，仅在重启成功后执行一次。

### 断开调用的返回语义
- `DISCONNECT_*` 的语义是“释放连接并保持断开”。
- 若 `DISCONNECT_*` 超时触发重启，主进程会杀死旧 daemon 并拉起新 daemon。
- 正常路径下，`DISCONNECT_*` 返回成功后主进程会执行一次 `SCAN` 验证；若验证失败则按“超时路径”处理。
- 新 daemon 启动后**不自动重连**；需要满足以下任一条件才视为断开完成：
  - `SCAN` 成功且随后一次 `CONNECT` 成功（立即再 `DISCONNECT_BY_HANDLE` 释放），或
  - `SCAN` 成功但设备数为 0（物理拔掉）。
- 若 `CONNECT` 返回 `already_connected`（被其它进程占用），视为断开未完成并返回错误。
  - 多进程场景下这是预期行为：设备被其它进程占用时断开校验失败。
- 若序列号不匹配（设备列表改变），返回错误并记录告警。
- 若 `SCAN` 返回错误或超时：重试至 `restart_max_retries`，仍失败则返回错误。
- daemon 无法启动或协议握手失败时，`DISCONNECT_*` 返回错误（`CanError::InitializationFailed`）。

### 重启重试范围
- `restart_max_retries` 为**总尝试次数上限**（包含第一次尝试）。
- 若达到上限，当前调用返回错误，但系统仍可在下一次调用再次尝试恢复。

### 启动重试策略
- 初次拉起 daemon 时也使用 `restart_max_retries` 与 `restart_interval_ms`。
- 若握手在 `startup_timeout_ms` 内仍失败，则返回 `InitializationFailed`。
 - 重启后等待 `restart_interval_ms` 再进行 `SCAN/CONNECT`，避免句柄未释放。

## 子进程交付与定位
- `canlink-tscan-daemon` 作为 crate 的 `[[bin]]` 构建产物，由用户 `cargo build`/`cargo install` 生成。
- 对库使用者：需将生成的 `canlink-tscan-daemon(.exe)` 与应用程序放在同目录，或在部署脚本中复制到指定位置。
- 主进程默认从**主程序同目录**查找 daemon；若未找到则按配置与 PATH 兜底。
- 找不到 daemon 时返回明确错误，提示用户构建/安装对应版本。
  - 默认策略为 **fail-closed**（不回退直连），用户可通过配置关闭隔离后再使用直连。

### 分发策略（面向 Rust 依赖用户）
- 库不自动生成 daemon；下游应用应在 CI/发布流程中同时构建 `canlink-tscan-daemon` 并与应用一同分发。
- 提供示例构建命令与部署说明（文档中补充）：`cargo build -p canlink-tscan --bin canlink-tscan-daemon --release`。
 - 文档落点：`canlink-tscan/README.md` 新增 “隔离模式部署” 小节，并在 `docs/getting-started.md` 增加构建/打包步骤。

### DLL 加载约束
- daemon 进程需确保 `libTSCAN.dll` 及其依赖与自身同目录，或可通过系统 PATH 找到。
- 若 DLL 不可加载，daemon 启动失败并返回明确错误。
- 启动失败的错误传递：主进程读取 daemon `stderr`，将最后 4 行拼接进 `InitializationFailed` 原因。
- 若 daemon 非 0 退出码，主进程视为启动失败并停止重试。
- 启动重试优先级：若出现非 0 退出码则立即失败；否则按 `restart_max_retries` 进行重试。

## 配置文件（独立）
**文件名建议**：`tscan_isolation.toml`

**默认行为**：
- 默认启用隔离（即使配置文件不存在）。
- 用户可显式关闭。

**建议字段**：
- `enabled = true|false`（默认 true）
- `schema_version = 1`
- `daemon_path = "..."`（可选；为空则按默认规则查找）
- `startup_timeout_ms = 3000`
- `request_timeout_ms = 2000`（除 RECV 外的通用请求超时）
- `disconnect_timeout_ms = 3000`（断开专用超时；未设置则沿用 request_timeout_ms）
- `recv_timeout_ms = 0`（RECV 默认非阻塞，单位毫秒）
- `restart_max_retries = 3`
- `restart_interval_ms = 500`

**重试节奏说明**：
- v1 使用固定 `restart_interval_ms`，避免复杂退避；若遇到连续失败，可在后续版本引入指数退避。

**daemon 查找顺序**：
1. `daemon_path`（若配置）
2. 主程序可执行文件同目录
3. `PATH` 中的 `canlink-tscan-daemon(.exe)`

**daemon_path 解析**：
- 若为相对路径，基于配置文件所在目录解析；若配置文件不存在，则基于主程序目录解析。

**配置文件发现顺序**：
1. 主程序可执行文件同目录
2. 当前工作目录
3. `%APPDATA%\\canlink\\tscan_isolation.toml`（Windows）

**发现策略**：
- 按顺序取第一个存在的配置文件，不做合并。

**配置容错策略**：
- 文件不存在：使用默认值（隔离启用）。
- 解析失败/字段非法：记录警告并使用默认值（隔离启用）。
- `schema_version` 不匹配：忽略该配置文件并使用默认值，同时记录警告。

## 文档声明（厂商 Bug 规避）
需要在以下位置明确说明：
- `canlink-tscan/README.md`
- 新增的隔离配置说明文档

声明内容要点：
- 该隔离机制为厂商 LibTSCAN 断开卡死问题的临时规避措施。
- 厂商修复后会发布新版并计划移除/降级此机制。
- 用户可通过配置关闭隔离。

## 测试计划
- 单元测试：协议编解码、消息路由、错误映射。
- 集成测试：启动/关闭 daemon、重启策略、超时路径。
- 手工硬件测试：复用现有 tscan 复现与压力测试用例。

## 风险与权衡
- 引入子进程带来额外复杂度，但能显著提升稳定性。
- stdio IPC 适配性强，性能足以覆盖 50–100ms 级别的常见负载。
- 极端高吞吐场景可能需要升级到本地 socket 或共享内存。

## 迁移与回退
- 默认启用隔离，配置可关闭。
- 厂商修复后，将发布新版，并在变更日志中说明取消该规避机制。
