# canlink-tscan v0.3.0 预发布说明（草稿）

日期：2026-03-18  
范围：`canlink-tscan`（Windows + LibTSCAN）

## 背景

`libTSCAN.dll` 在 `DISCONNECT_*` 调用上存在厂商侧卡死风险。  
本版本引入进程隔离规避方案，避免主进程被阻塞。

## 变更摘要

- 新增 daemon 进程：`canlink-tscan-daemon`。
- 新增 IPC 协议与长度前缀 JSON 编解码（1 MiB 帧上限）。
- `TSCanBackend` 默认启用 daemon 路径。
- 新增配置解析：`BackendConfig.parameters` > `canlink-tscan.toml` > 默认值。
- 新增无硬件测试：配置、协议、server/client、超时重启路径。

## 配置说明

```toml
# canlink-tscan.toml
use_daemon = true
request_timeout_ms = 2000
disconnect_timeout_ms = 3000
restart_max_retries = 3
recv_timeout_ms = 0
# daemon_path = "C:/path/to/canlink-tscan-daemon.exe"
```

- 默认值 `use_daemon = true`。
- 设置 `use_daemon = false` 时回退到直接 DLL 调用。

## 兼容性与风险

- 本变更属于厂商 Bug 规避措施，不改变上层 `CanBackend` 接口。
- 若厂商后续修复 DLL，可在后续版本降级或移除该规避逻辑。
- 旧项目需确保能找到 `canlink-tscan-daemon(.exe)`，或显式配置 `daemon_path`。

## 当前验证状态

已完成（无硬件）：

- `cargo test -p canlink-tscan` 全通过。
- daemon 协议、配置优先级、超时重启关键路径已覆盖。

已完成（硬件实机，2026-03-19）：

- `scripts\tscan_hw_regression.bat` 回归通过。
- 2 小时 soak 通过：`151/151 PASS`。
- 真实库路径断连压测通过：
- `disconnect_client_stress by_handle 20000`：`PASS`
- `disconnect_client_stress disconnect_all 20000`：`PASS`

当前结论：

- 对当前机器、当前硬件、当前 DLL/Lib 配对和当前代码版本，规避补丁已经生效。
- 该结论表示“我们的工程规避已验证有效”，不表示厂商 DLL 底层问题已经消失。
- 厂商问题仍建议持续跟踪，但不再阻塞当前版本推进。

## 发布说明建议文案

> 本版本为厂商 DLL 断开卡死问题的工程规避版。默认启用 daemon 隔离以保证主进程稳定性，并保留显式关闭开关。待厂商提供稳定修复后，我们将评估并在后续版本调整该机制。
