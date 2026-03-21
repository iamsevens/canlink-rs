# TSMaster / LibTSCAN 厂商资料说明

本目录在公开仓库中只保留这一份说明文件。

## 公开分发边界

为降低公开发布时的授权与再分发风险，仓库**不再包含**以下厂商原始资料：

- 官方 PDF 文档
- 官方 ZIP 压缩包
- 官方示例代码及其解压内容
- 官方头文件原件
- 官方库文件、二进制与构建产物

这些资料请从厂商官方渠道自行获取，并仅在本地开发环境中使用。

## 官方来源

- 下载入口: <https://www.tosunai.com/en/downloads/>
- TSMaster API 下载页: <https://www.tosunai.com/downloads/tsmaster-api/>
- 网站使用条款: <https://www.tosunai.com/website-terms-of-use/>

## 本仓库如何使用这些资料

本项目的 `canlink-tscan` / `canlink-tscan-sys` 实现和文档，基于开发过程中查阅过的官方 `TSMaster` / `LibTSCAN` 资料整理而来。

当前仓库中保留的是：

- 我们自己编写的接口封装与实现代码
- 我们自己编写的能力说明、兼容性说明与测试文档
- 指向厂商官方下载页的入口信息

当前仓库中**不保留**厂商原始 SDK 内容本身。

## 本地开发建议

如需在本地继续研究或调试，可自行下载官方资料，并放在未跟踪目录中，例如：

- `docs/vendor/tsmaster/api/`
- `docs/vendor/tsmaster/examples/`
- `docs/vendor/tsmaster/headers/`
- `docs/vendor/tsmaster/libs/`

这些目录已加入 `.gitignore`，不会随公开仓库分发。
