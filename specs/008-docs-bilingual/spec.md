# 文档双语与命名统一（CANLink）规范

## 背景
- 当前仓库对外名称与 crate 名称均为 `canlink-*`，但部分文档仍写作 `CANLink-RS`，会造成对外认知不一致。
- 已发布 4 个 crate，需要在文档中清晰呈现其关联关系与完整发布范围。
- GitHub README 需中文优先，docs.rs / crates.io 需英文优先，并支持中英文切换。

## 目标
- 统一项目名称为 **CANLink**，仅在简介处保留一次“Rust 实现/生态”说明。
- 根 README 中文优先；crate README 与 docs.rs 顶部文档英文优先。
- 明确展示 4 个已发布 crate 与依赖关系。
- 为 GitHub / crates.io / docs.rs 提供单文件双语切换锚点。

## 非目标
- 不翻译 `docs/` 下的深度指南/报告/实验记录。
- 不调整 API 或功能，仅限文档与说明口径。
- 不新增 spec/plan/task 到 `docs/` 目录。

## 范围
- 根 `README.md`
- 4 个 crate README：
  - `canlink-hal/README.md`
  - `canlink-tscan-sys/README.md`
  - `canlink-tscan/README.md`
  - `canlink-cli/README.md`
- 4 个 crate 顶部文档注释（docs.rs 渲染）：
  - `canlink-hal/src/lib.rs`
  - `canlink-tscan-sys/src/lib.rs`
  - `canlink-tscan/src/lib.rs`
  - `canlink-cli/src/lib.rs`
- 4 个 crate `Cargo.toml` 的 `description` / `readme` 字段（crates.io 展示口径）

## 双语结构规则
- 单文件双语，顶部锚点切换：
  - 中文优先：`[中文](#zh) | [English](#en)`
  - 英文优先：`[English](#en) | [中文](#zh)`
- 统一使用 ASCII 锚点，避免 GitHub / crates.io / docs.rs 锚点规则不一致：
  - 在中文段落前插入 `<a id="zh"></a>`
  - 在英文段落前插入 `<a id="en"></a>`
- 根 README：中文段落在前、英文段落在后。
- crate README 与 `lib.rs` 文档：英文段落在前、中文段落在后。
- 全文统一使用 **CANLink** 作为项目名；“CANLink（Rust 实现）”仅在根 README 首段出现一次。

## 包关系呈现
- 根 README 增加 “Crate Map” 表，明确 4 个已发布 crate 与依赖关系：
  - `canlink-hal`：核心 HAL
  - `canlink-tscan-sys`：LibTSCAN FFI 绑定
  - `canlink-tscan`：LibTSCAN 后端（依赖 `canlink-hal` + `canlink-tscan-sys`）
  - `canlink-cli`：命令行工具（依赖 `canlink-hal` + `canlink-tscan`）
- 每个 crate README 增加 “Related Crates / 关联包”。
- 每个 crate `lib.rs` 文档顶部加入 “Related Crates”。
- 相关包的链接格式固定：
  - README 使用 crates.io 链接：`https://crates.io/crates/<name>`
  - `lib.rs` 使用 docs.rs 链接：`https://docs.rs/<name>`
- 顺序与根 README “Crate Map” 保持一致，并核对 `Cargo.toml` 实际依赖关系。

## 平台渲染映射
- GitHub：根 `README.md`
- crates.io：各 crate `Cargo.toml` 的 `readme` 指向的 README + `description`
- docs.rs：各 crate `src/lib.rs` 顶部文档

## 英文/中文内容密度
- README 与 `lib.rs` 提供完整双语（不做缩略版），避免信息不对称。
- docs.rs / crates.io 的默认展示页以英文区块为首。
- `Cargo.toml` 的 `description` 使用英文。
- README 与 `lib.rs` 的双语内容需保持一致（可通过 `#![doc = include_str!("../README.md")]` 复用 README 作为单一事实源，避免漂移）。

## 验收标准
- 根 README 顶部默认中文，并可点击切换英文。
- 4 个 crate README 顶部默认英文，并可点击切换中文。
- docs.rs 主页显示英文优先的 crate 文档，锚点跳转可用。
- crates.io 显示英文优先 README 与英文 `description`，锚点跳转可用。
- 文档中不再出现 “CANLink-RS” 作为项目名（仅限范围内文件）。
- 4 个 crate 的关联关系在根 README 与各 crate README 中可见。

## 变更清单（预期）
- 修改 `README.md`：
  - `CANLink-RS` -> `CANLink`
  - 添加双语锚点与英文区块
  - 增加 Crate Map 表
- 修改 4 个 crate README：
  - `CANLink-RS` -> `CANLink`
  - 英文优先 + 双语锚点
  - 添加 Related Crates
- 修改 4 个 crate `src/lib.rs` 顶部文档：
  - 英文优先 + 双语锚点
  - 添加 Related Crates
  - 可选：用 `include_str!("../README.md")` 复用 README
- 修改 4 个 crate `Cargo.toml`：
  - `description` 统一英文
  - 必要时补充 `readme = "README.md"`

