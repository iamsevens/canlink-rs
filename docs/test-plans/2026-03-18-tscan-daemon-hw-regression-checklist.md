# TSCan daemon 规避方案硬件回归清单

日期：2026-03-18  
适用分支：`codex/tscan-workaround-docs`

## 前置条件

- 已连接 TSCan 硬件（例如 TC1011）。
- `libTSCAN.dll` 可在系统 PATH 被加载。
- 当前代码已包含 `canlink-tscan-daemon` 与 `canlink-tscan-daemon-stub`。

## 执行命令

完整硬件回归：

```bat
scripts\tscan_hw_regression.bat
```

无硬件流程（现在可执行）：

```bat
scripts\tscan_hw_regression.bat --no-hw
```

自定义日志目录：

```bat
scripts\tscan_hw_regression.bat _logs\hw_regression\manual_run
```

## 用例与通过标准

1. 构建阶段  
通过标准：daemon 与 stub 二进制可成功构建。

2. 包内测试阶段  
通过标准：`cargo test -p canlink-tscan` 全通过。

3. 硬件基础连通（`backend_test`）  
通过标准：初始化、开通道、发收、关闭流程完整，无卡死。

4. CAN-FD 场景（`canfd_test`）  
通过标准：支持检测正确，CAN-FD 发送与接收流程正常结束。

5. 过滤场景（`hardware_filter_test`）  
通过标准：流程可运行结束，输出统计结果，不出现卡死。

6. 规避行为确认  
通过标准：`use_daemon=true` 路径稳定；`use_daemon=false` 可显式回退。

## 日志产物

默认目录：`_logs\hw_regression\<timestamp>\`

建议关注：

- `01_build.log`
- `02_test.log`
- `03_backend_test.log`
- `04_canfd_test.log`
- `05_hardware_filter_test.log`
- `summary.txt`

## 回归结论模板

```text
分支：
硬件：
DLL 版本：
执行时间：

结果：
- Build: PASS/FAIL
- Unit+Integration: PASS/FAIL
- backend_test: PASS/FAIL
- canfd_test: PASS/FAIL
- hardware_filter_test: PASS/FAIL

结论：
```
