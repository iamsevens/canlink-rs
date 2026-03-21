# Release Guide - CANLink-RS v0.2.0

## GitHub Actions 发布流程（当前推荐）

- `CI`：`.github/workflows/ci.yml`
  - 触发：`push` / `pull_request` 到 `main`
  - 作用：执行 `fmt + clippy + build + test + doc test + doc`，作为主干质量门禁
- `Release Dry Run`：`.github/workflows/release-dryrun.yml`
  - 触发：`workflow_dispatch`
  - 作用：发布前对全部 crate 执行 `cargo publish --dry-run`（对尚未上架依赖使用 `patch.crates-io` 指向工作区本地路径）
- `Release Publish`：`.github/workflows/release-publish.yml`
  - 触发：`workflow_dispatch`
  - 输入：`version`、`confirm=publish`
  - 作用：按依赖顺序发布到 crates.io，并等待索引生效后再发布下一个包

### 发布前仓库设置

- 在 GitHub 仓库 Secret 中配置：`CARGO_REGISTRY_TOKEN`
- 建议配置 `main` 分支保护，必需状态检查至少包含 `CI`
- `Release Publish` 默认使用 `environment: crates-io`，可在 GitHub 中加人工审批

### LibTSCAN 路径

- 工作流默认设置：
  - `CANLINK_TSCAN_BUNDLE_DIR=${{ github.workspace }}\\docs\\vendor\\tsmaster\\examples\\LibTSCAN\\lib_extracted\\lib\\lib\\windows\\x64`
- 如路径变更，可在 workflow `env` 中调整



**Project**: CANLink-RS - CAN Hardware Abstraction Layer

**Target Version**: v0.2.0

**Date**: 2026-01-09



---



## 📋 Pre-Release Checklist



### 1. Code Quality Verification



- [ ] **Run all tests**

  ```bash

  cargo test --all-features --workspace

  ```



- [ ] **Run benchmarks** (ensure they compile)

  ```bash

  cargo bench --no-run --all-features

  ```



- [ ] **Run quality checks**

  ```bash

  # Linux/macOS

  ./scripts/check.sh



  # Windows

  scripts\check.bat

  ```



- [ ] **Check formatting**

  ```bash

  cargo fmt --all -- --check

  ```



- [ ] **Run Clippy**

  ```bash

  cargo clippy --all-targets --all-features -- -D warnings

  ```



- [ ] **Build documentation**

  ```bash

  cargo doc --no-deps --all-features --workspace

  ```



- [ ] **Security audit**

  ```bash

  cargo audit

  ```



### 2. Version Updates



- [ ] **Update version in Cargo.toml files**

  - `Cargo.toml` (workspace)

  - `canlink-hal/Cargo.toml`

  - `canlink-mock/Cargo.toml`

  - `canlink-cli/Cargo.toml`



  ```toml

  [package]

  version = "0.2.0"

  ```



- [ ] **Update version in documentation**

  - README.md files

  - CHANGELOG.md

  - Documentation examples



### 3. Documentation Review



- [ ] **Review README.md files**

  - Root README.md

  - canlink-hal/README.md (if exists)

  - canlink-mock/README.md

  - canlink-cli/README.md



- [ ] **Create/Update CHANGELOG.md**

  ```markdown

  # Changelog



  ## [0.2.0] - 2026-01-09



  ### Added

  - Initial release

  - Hardware abstraction layer (canlink-hal)

  - Mock backend for testing (canlink-mock)

  - Command-line interface (canlink-cli)

  - Comprehensive test suite (140 tests)

  - Performance benchmarks

  - Complete documentation



  ### Features

  - Unified CAN backend interface

  - Backend registry and discovery

  - CAN 2.0 and CAN-FD support

  - Message recording and verification

  - Error injection for testing

  - Multi-threaded support

  ```



- [ ] **Review all examples**

  - Ensure examples compile

  - Test examples run correctly

  - Update example documentation



### 4. Testing



- [ ] **Run full test suite**

  ```bash

  cargo test --all-features --workspace -- --nocapture

  ```



- [ ] **Test on multiple platforms** (if possible)

  - [ ] Linux

  - [ ] Windows

  - [ ] macOS



- [ ] **Test examples**

  ```bash

  cargo run --example basic_usage

  cargo run --example backend_switching

  cargo run --example capability_query

  cargo run --example capability_adaptation

  cargo run --example mock_testing

  cargo run --example automated_testing

  ```



- [ ] **Test CLI commands**

  ```bash

  cargo run -p canlink-cli -- list

  cargo run -p canlink-cli -- info mock

  cargo run -p canlink-cli -- send mock 0 0x123 01 02 03

  ```



---



## 🚀 Release Process



### Step 1: Prepare Release Branch



```bash

# Ensure you're on the main branch

git checkout main

git pull origin main



# Create release branch

git checkout -b release/v0.2.0

```



### Step 2: Update Version Numbers



Edit the following files:



**Cargo.toml (workspace root):**

```toml

[workspace.package]

version = "0.2.0"

```



**Verify all crates inherit the version:**

```bash

grep -r "version.workspace = true" */Cargo.toml

```



### Step 3: Create CHANGELOG.md



Create `CHANGELOG.md` in the root directory:



```markdown

# Changelog



All notable changes to this project will be documented in this file.



The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),

and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).



## [0.2.0] - 2026-01-09



### Added



#### Core Features

- Hardware abstraction layer (canlink-hal) with unified backend interface

- Mock backend (canlink-mock) for testing without hardware

- Command-line interface (canlink-cli) for CAN operations

- Backend registry and discovery system

- Configuration-based backend switching



#### Message Support

- CAN 2.0 standard and extended frames

- CAN-FD support with up to 64 bytes

- Remote frames (RTR)

- Message timestamps with microsecond precision



#### Testing Features

- Message recording and verification

- Preset message configuration

- Error injection for testing

- 140 comprehensive tests (93% coverage)



#### Documentation

- Complete API documentation (100% coverage)

- 6 working examples

- Thread safety usage guide

- Performance analysis

- Quick start guide



#### Performance

- Capability queries < 1ms (actual: ~46ns)

- Abstraction overhead ~6.6%

- Comprehensive benchmark suite



### Technical Details

- Rust edition: 2021

- MSRV: 1.70.0

- Platforms: Linux, Windows, macOS



<!-- Public release URL to be filled after the repository is finalized. -->

```



### Step 4: Commit Changes



```bash

# Add all changes

git add -A



# Commit with release message

git commit -m "chore: prepare release v0.2.0



- Update version to 0.2.0

- Add CHANGELOG.md

- Update documentation

"



# Push release branch

git push origin release/v0.2.0

```



### Step 5: Create Pull Request



1. Go to GitHub/GitLab

2. Create Pull Request from `release/v0.2.0` to `main`

3. Title: "Release v0.2.0"

4. Description: Copy content from CHANGELOG.md

5. Wait for CI to pass

6. Get review approval

7. Merge to main



### Step 6: Create Git Tag



```bash

# Switch to main branch

git checkout main

git pull origin main



# Create annotated tag

git tag -a v0.2.0 -m "Release v0.2.0



CANLink-RS v0.2.0 - Release



Features:

- Hardware abstraction layer

- Mock backend for testing

- Command-line interface

- 140 tests with 93% coverage

- Complete documentation



See CHANGELOG.md for details.

"



# Push tag

git push origin v0.2.0

```



### Step 7: Create GitHub Release



1. Go to GitHub Releases page

2. Click "Draft a new release"

3. Select tag: `v0.2.0`

4. Release title: `v0.2.0 - Release`

5. Description: Copy from CHANGELOG.md

6. Attach artifacts (optional):

   - Pre-built binaries

   - Documentation archive

7. Click "Publish release"



### Step 8: Publish to Crates.io



**Important**: Publish in dependency order!



```bash

# 1. Publish canlink-hal first (no dependencies)

cd canlink-hal

cargo publish --dry-run  # Test first

cargo publish



# Wait for crates.io to index (usually 1-2 minutes)

sleep 120



# 2. Publish canlink-mock (depends on canlink-hal)

cd ../canlink-mock

cargo publish --dry-run

cargo publish



# Wait for indexing

sleep 120



# 3. Publish canlink-cli (depends on canlink-hal and canlink-mock)

cd ../canlink-cli

cargo publish --dry-run

cargo publish

```



**Note**: You need a crates.io account and API token:

```bash

# Login to crates.io

cargo login <your-api-token>

```



### Step 9: Verify Publication



```bash

# Check crates.io

open https://crates.io/crates/canlink-hal

open https://crates.io/crates/canlink-mock

open https://crates.io/crates/canlink-cli



# Test installation

cargo install canlink-cli

canlink --version

```



### Step 10: Announce Release



- [ ] Update project README with installation instructions

- [ ] Post announcement on:

  - Project discussion forum

  - Rust community forums

  - Social media (if applicable)

- [ ] Update documentation website (if exists)



---



## 📦 Post-Release Tasks



### 1. Update Development Branch



```bash

# Create next development version

git checkout main

git checkout -b develop



# Update version to 0.2.1-dev

# Edit Cargo.toml:

# version = "0.2.1-dev"



git commit -am "chore: bump version to 0.2.1-dev"

git push origin develop

```



### 2. Monitor Issues



- Watch for bug reports

- Respond to user questions

- Track feature requests



### 3. Plan Next Release



- Review feedback

- Prioritize features for v0.3.0

- Update roadmap



---



## 🔧 Troubleshooting



### Publishing Fails



**Error**: "crate not found"

- **Solution**: Wait for crates.io to index previous crate



**Error**: "version already exists"

- **Solution**: Increment version number



**Error**: "missing documentation"

- **Solution**: Ensure all public APIs are documented



### CI Fails



**Error**: Tests fail

- **Solution**: Fix tests before releasing



**Error**: Clippy warnings

- **Solution**: Fix all warnings



### Tag Already Exists



```bash

# Delete local tag

git tag -d v0.2.0



# Delete remote tag

git push origin :refs/tags/v0.2.0



# Recreate tag

git tag -a v0.2.0 -m "Release v0.2.0"

git push origin v0.2.0

```



---



## 📝 Release Checklist Summary



### Pre-Release

- [ ] All tests pass

- [ ] Quality checks pass

- [ ] Documentation complete

- [ ] Examples work

- [ ] Version numbers updated

- [ ] CHANGELOG.md created



### Release

- [ ] Release branch created

- [ ] Changes committed

- [ ] Pull request merged

- [ ] Git tag created

- [ ] GitHub release created

- [ ] Published to crates.io (in order)

- [ ] Installation verified



### Post-Release

- [ ] Development version updated

- [ ] Announcement posted

- [ ] Issues monitored

- [ ] Next release planned



---



## 🎯 Quick Release Commands



```bash

# Full release workflow

./scripts/release.sh v0.2.0



# Or manually:

git checkout -b release/v0.2.0

# Update versions and CHANGELOG

git commit -am "chore: prepare release v0.2.0"

git push origin release/v0.2.0

# Create PR, merge to main

git checkout main

git pull

git tag -a v0.2.0 -m "Release v0.2.0"

git push origin v0.2.0

# Publish to crates.io

cd canlink-hal && cargo publish

cd ../canlink-mock && cargo publish

cd ../canlink-cli && cargo publish

```



---



## 📚 Additional Resources



- [Cargo Book - Publishing](https://doc.rust-lang.org/cargo/reference/publishing.html)

- [Semantic Versioning](https://semver.org/)

- [Keep a Changelog](https://keepachangelog.com/)

- [Crates.io Publishing Guide](https://doc.rust-lang.org/cargo/reference/publishing.html)



---



**Last Updated**: 2026-01-09

**Status**: Ready for v0.2.0 Release
