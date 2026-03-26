# CANLink 文档双语与命名统一 Implementation Plan

**Goal:** 统一项目名为 CANLink，并在 GitHub / crates.io / docs.rs 上实现双语文档与清晰的四包关联展示。

**Architecture:** GitHub 根 README 中文优先、crate README 英文优先；docs.rs 文档采用英文优先的独立文档块（避免 crates.io / docs.rs 链接冲突）。

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

### Task 2: 根 README 双语（中文优先）+ Crate Map

**Files:**
- Modify: `D:\dev\canlink\README.md`

- [ ] **Step 1: 顶部添加语言切换锚点与可见切换行**
  - 使用 `<a id="zh"></a>` / `<a id="en"></a>`
  - 紧跟可见切换行 `[中文](#zh) | [English](#en)`
- [ ] **Step 2: 全文将 “CANLink-RS” 统一为 “CANLink”**
  - 仅在首段保留一次 “CANLink（Rust 实现）”
- [ ] **Step 3: 增加 Crate Map 表**
  - `canlink-hal` / `canlink-tscan-sys` / `canlink-tscan` / `canlink-cli`
  - 对照各 crate `Cargo.toml` 依赖核对顺序与关系
  - 显式写清依赖关系：
    - `canlink-tscan` 依赖 `canlink-hal` + `canlink-tscan-sys`
    - `canlink-cli` 依赖 `canlink-hal` + `canlink-tscan`
- [ ] **Step 4: 增加英文区块（内容与中文对应）**

### Task 3: 4 个 crate README 双语（英文优先）+ 关联包

**Files:**
- Modify: `D:\dev\canlink\canlink-hal\README.md`
- Modify: `D:\dev\canlink\canlink-tscan-sys\README.md`
- Modify: `D:\dev\canlink\canlink-tscan\README.md`
- Modify: `D:\dev\canlink\canlink-cli\README.md`

- [ ] **Step 1: 顶部添加语言切换锚点与可见切换行**（英文优先）
  - 使用 `<a id="en"></a>` / `<a id="zh"></a>`
  - 紧跟可见切换行 `[English](#en) | [中文](#zh)`
- [ ] **Step 2: 全文将 “CANLink-RS” 统一为 “CANLink”**
- [ ] **Step 3: 增加 Related Crates / 关联包**
  - README 使用 crates.io 链接
  - 顺序与根 README 的 Crate Map 一致
  - 每个 crate 明确自身位置与依赖关系（与 Crate Map 对齐）
- [ ] **Step 4: 补充中文区块（完整双语）**

### Task 4: docs.rs 文档（独立英文优先文档块）

**Files:**
- Modify: `D:\dev\canlink\canlink-hal\src\lib.rs`
- Modify: `D:\dev\canlink\canlink-tscan-sys\src\lib.rs`
- Modify: `D:\dev\canlink\canlink-tscan\src\lib.rs`
- Modify: `D:\dev\canlink\canlink-cli\src\main.rs`

- [ ] **Step 1: 将 crate 顶部文档改为英文优先 + 双语锚点**
  - 使用 docs.rs 链接（不是 crates.io 链接）
- [ ] **Step 2: `canlink-cli` 在 `src/main.rs` 顶部添加文档**
  - 与 README 同步内容但链接使用 docs.rs

### Task 5: 验证与收尾

**Files:**
- Modify: `D:\dev\canlink\README.md` 等（检查范围）

- [ ] **Step 1: 校验旧项目名消除**

```powershell
rg -n -i "canlink[- ]?rs" README.md canlink-*/README.md canlink-*/src/*.rs canlink-*/Cargo.toml -S
```

Expected: no matches.

- [ ] **Step 2: 校验“Rust 实现”仅根 README 出现一次**

```powershell
rg -n "Rust 实现" README.md canlink-*/README.md canlink-*/src/*.rs -S
```

Expected: only in `README.md`.

- [ ] **Step 3: 校验 Crate Map / Related Crates 段落存在**

```powershell
rg -n "Crate Map|Related Crates|关联包" README.md canlink-*/README.md -S
```

- [ ] **Step 4: 生成文档（轻量）**

```powershell
cargo doc --no-deps -p canlink-hal -p canlink-tscan-sys -p canlink-tscan -p canlink-cli
```

Expected: succeed.

- [ ] **Step 5: 提交并推送全部文档变更**

```powershell
git add README.md canlink-*/README.md canlink-*/src/*.rs canlink-*/Cargo.toml

git commit -m "docs: bilingual README and docs.rs alignment"

git push origin main
```

