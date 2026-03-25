# LibTSCAN 运行库获取与配置指南

## 适用范围

本指南用于 `canlink-tscan` / `canlink-tscan-sys` 的运行库准备。项目不分发厂商库文件，请按厂商许可自行获取。

## 已验证平台

- 当前已在 Windows 10/11 (x64) 验证。
- LibTSCAN 文档包含 Linux/macOS 相关库与示例，但本项目尚未在这些平台验证，也未在非 Windows 平台编译支持。

## 获取方式

- 推荐入口：优先从厂商 TSMaster API 下载页获取（`https://www.tosunai.com/downloads/tsmaster-api/`）。
- 方式 A：安装 TSMaster 软件包，从安装目录中获取 LibTSCAN 运行库（下载入口：`https://www.tosunai.com/en/downloads/`）。
- 方式 B：从厂商提供的 LibTSCAN 运行库包中获取（若厂商提供此类分发方式）。

说明：无论通过哪种方式获取，均需遵守厂商许可。本项目不提供 DLL/Lib 的打包分发。

## Windows 所需文件

在 Windows 上构建与运行时，需要以下文件同时存在：

- `libTSCAN.dll`
- `libTSCAN.lib`

> 仅有 DLL 或仅有 LIB 都会导致编译或运行失败。

## 配置方式

`canlink-tscan-sys` 支持以下常见方式定位运行库：

1. `TSMASTER_HOME`
   - 指向 TSMaster 安装目录，例如 `C:\Program Files\TSMaster`。
2. `CANLINK_TSCAN_BUNDLE_DIR`
   - 指向包含 `libTSCAN.dll` + `libTSCAN.lib` 的目录。
3. 系统 PATH
   - 运行时确保 `libTSCAN.dll` 在 `PATH` 或可执行文件同目录。

示例（PowerShell）：

```powershell
# 指定 LibTSCAN bundle 目录
$env:CANLINK_TSCAN_BUNDLE_DIR = "C:\Program Files\TSMaster\bin\x64"

# 或指定 TSMaster 安装目录
$env:TSMASTER_HOME = "C:\Program Files\TSMaster"
```

## 版本与位数匹配

- Rust 目标为 x64 时，必须使用 x64 的 `libTSCAN.dll` + `libTSCAN.lib`。
- DLL/Lib 版本需与硬件及厂商发行版本匹配。

## 验证步骤

无硬件时的基本验证：

```powershell
cargo build -p canlink-tscan
```

有硬件时的回归验证：

```powershell
scripts\tscan_hw_regression.bat
```

## 常见问题

1. 报错找不到 `libTSCAN.lib`
   - 确认 `CANLINK_TSCAN_BUNDLE_DIR` 指向的目录内同时包含 `libTSCAN.lib` 与 `libTSCAN.dll`。

2. 运行时报错找不到 `libTSCAN.dll`
   - 将 DLL 放入可执行文件同目录或加入系统 `PATH`。

3. 非 Windows 平台无法构建
   - 当前 crate 仅在 Windows 目标下启用，其他平台尚未支持。
