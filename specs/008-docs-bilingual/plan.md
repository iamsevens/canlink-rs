# CANLink 文档双语与命名统一 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 统一项目名为 CANLink，并在 GitHub/ crates.io / docs.rs 上实现双语文档与清晰的四包关联展示。

**Architecture:** 以 README 作为单一事实源，crate 级文档通过 `include_str!` 复用 README；GitHub 根 README 中文优先、crate README 英文优先。

**Tech Stack:** Rust 文档注释（rustdoc）、Markdown、Cargo 元数据。

---

### Task 1: 统一 crate 元数据（description/readme）

**Files:**
- Modify: `D:\dev\canlink\canlink-hal\Cargo.toml`
- Modify: `D:\dev\canlink\canlink-tscan-sys\Cargo.toml`
- Modify: `D:\dev\canlink\canlink-tscan\Cargo.toml`
- Modify: `D:\dev\canlink\canlink-cli\Cargo.toml`

- [ ] **Step 1: 将 `description` 统一为英文描述**
- [ ] **Step 2: 确认每个 crate 设置 `readme = "README.md"`（如缺失则补）**
- [ ] **Step 3: 检查关键词与分类是否仍合理（不改动功能）**
- [ ] **Step 4: 提交变更**

### Task 2: 根 README 双语（中文优先）+ Crate Map

**Files:**
- Modify: `D:\dev\canlink\README.md`

- [ ] **Step 1: 顶部添加语言切换锚点**
  - 使用 `<a id="zh"></a>` / `<a id="en"></a>`
- [ ] **Step 2: 全文将 “CANLink-RS” 统一为 “CANLink”**
  - 仅在首段保留一次 “CANLink（Rust 实现）”
- [ ] **Step 3: 增加 Crate Map 表**
  - `canlink-hal` / `canlink-tscan-sys` / `canlink-tscan` / `canlink-cli`
- [ ] **Step 4: 增加英文区块（内容与中文对应）**
- [ ] **Step 5: 提交变更**

### Task 3: 4 个 crate README 双语（英文优先）+ 关联包

**Files:**
- Modify: `D:\dev\canlink\canlink-hal\README.md`
- Modify: `D:\dev\canlink\canlink-tscan-sys\README.md`
- Modify: `D:\dev\canlink\canlink-tscan\README.md`
- Modify: `D:\dev\canlink\canlink-cli\README.md`

- [ ] **Step 1: 顶部添加语言切换锚点**（英文优先）
- [ ] **Step 2: 全文将 “CANLink-RS” 统一为 “CANLink”**
- [ ] **Step 3: 增加 Related Crates / 关联包**
  - README 使用 crates.io 链接
- [ ] **Step 4: 补充中文区块（完整双语）**
- [ ] **Step 5: 提交变更**

### Task 4: docs.rs 文档（复用 README）

**Files:**
- Modify: `D:\dev\canlink\canlink-hal\src\lib.rs`
- Modify: `D:\dev\canlink\canlink-tscan-sys\src\lib.rs`
- Modify: `D:\dev\canlink\canlink-tscan\src\lib.rs`
- Modify: `D:\dev\canlink\canlink-cli\src\main.rs`

- [ ] **Step 1: 在文件顶部加入 `#![doc = include_str!("../README.md")]`**
- [ ] **Step 2: 移除原有顶层 doc 注释，避免重复**
- [ ] **Step 3: `canlink-cli` 使用 `src/main.rs` 作为文档入口（bin crate）**
- [ ] **Step 4: 提交变更**

### Task 5: 验证与收尾

**Files:**
- Modify: `D:\dev\canlink\README.md` 等（检查范围）

- [ ] **Step 1: 校验关键字消除**

```powershell
rg -n "CANLink-RS" README.md canlink-*/README.md canlink-*/src/*.rs -S
```

Expected: no matches.

- [ ] **Step 2: 生成文档（轻量）**

```powershell
cargo doc --no-deps -p canlink-hal -p canlink-tscan-sys -p canlink-tscan -p canlink-cli
```

Expected: succeed.

- [ ] **Step 3: 提交并推送全部文档变更**

```powershell
git add README.md canlink-*/README.md canlink-*/src/*.rs canlink-*/Cargo.toml

git commit -m "docs: bilingual README and docs.rs alignment"

git push origin main
```

