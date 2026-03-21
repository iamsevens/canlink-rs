# 🚀 Quick Release Guide



## 发布 v0.1.0 的简单步骤



### 方法 1: 使用自动化脚本（推荐）



**Linux/macOS:**

```bash

./scripts/release.sh 0.1.0

```



**Windows:**

```cmd

scripts\release.bat 0.1.0

```



脚本会自动：

- ✅ 运行所有测试

- ✅ 运行质量检查

- ✅ 更新版本号

- ✅ 创建 git tag

- ✅ 推送到远程仓库

- ✅ 发布到 crates.io



---



### 方法 2: 手动发布



#### 1. 运行检查

```bash

# 运行所有测试

cargo test --all-features --workspace



# 运行质量检查

./scripts/check.sh  # Linux/macOS

scripts\check.bat   # Windows

```



#### 2. 更新版本号

编辑 `Cargo.toml`（workspace root）:

```toml

[workspace.package]

version = "0.1.0"

```



#### 3. 确认 CHANGELOG.md

确保 `CHANGELOG.md` 已创建并包含 v0.1.0 的更新内容。



#### 4. 提交并打标签

```bash

git add -A

git commit -m "chore: prepare release v0.1.0"

git tag -a v0.1.0 -m "Release v0.1.0"

git push origin main

git push origin v0.1.0

```



#### 5. 发布到 crates.io

**重要**: 按依赖顺序发布！



```bash

# 1. 发布 canlink-hal（无依赖）

cd canlink-hal

cargo publish



# 2. 等待索引（2分钟）

sleep 120



# 3. 发布 canlink-mock（依赖 canlink-hal）

cd ../canlink-mock

cargo publish



# 4. 等待索引

sleep 120



# 5. 发布 canlink-cli（依赖前两者）

cd ../canlink-cli

cargo publish

```



#### 6. 创建 GitHub Release

1. 打开 GitHub 仓库的 Releases 页面
2. 选择 tag: `v0.1.0`

3. 标题: `v0.1.0 - Initial Release`

4. 描述: 复制 CHANGELOG.md 中的内容

5. 点击 "Publish release"



#### 7. 验证发布

```bash

# 检查 crates.io

open https://crates.io/crates/canlink-hal

open https://crates.io/crates/canlink-mock

open https://crates.io/crates/canlink-cli



# 测试安装

cargo install canlink-cli

canlink --version

```



---



## 📋 发布前检查清单



- [ ] 所有测试通过

- [ ] 质量检查通过

- [ ] 文档构建成功

- [ ] 版本号已更新

- [ ] CHANGELOG.md 已创建

- [ ] 示例可以运行

- [ ] README.md 已更新



---



## 🔧 常见问题



### Q: 发布失败怎么办？

**A**: 检查错误信息：

- "crate not found" → 等待 crates.io 索引

- "version already exists" → 增加版本号

- "missing documentation" → 确保所有公共 API 有文档



### Q: 如何删除错误的 tag？

```bash

# 删除本地 tag

git tag -d v0.1.0



# 删除远程 tag

git push origin :refs/tags/v0.1.0

```



### Q: 需要 crates.io 账号吗？

**A**: 是的，需要：

1. 注册账号: https://crates.io/

2. 获取 API token

3. 登录: `cargo login <your-token>`



---



## 📚 详细文档



完整的发布流程请参考: [release-guide.md](release-guide.md)



---



## 🎯 发布后



- [ ] 在 GitHub 创建 release

- [ ] 验证 crates.io 上的包

- [ ] 测试安装

- [ ] 发布公告

- [ ] 更新开发版本为 0.2.0-dev



---



**准备好了吗？运行发布脚本开始吧！** 🚀



```bash

./scripts/release.sh 0.1.0

```
