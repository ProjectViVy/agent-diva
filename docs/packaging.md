# Agent Diva 跨平台打包指南

本文档介绍如何为多个平台构建 Agent Diva 安装包。

## 目录

- [方案概览](#方案概览)
- [GitHub Actions 自动构建](#github-actions-自动构建)
- [macOS 打包](#macos-打包)
- [使用 cross 进行交叉编译](#使用-cross-进行交叉编译)
- [使用 cargo-deb 构建 DEB 包](#使用-cargo-deb-构建-deb-包)
- [本地构建指南](#本地构建指南)

---

## 方案概览

| 平台 | 包格式 | 推荐方案 |
|------|--------|----------|
| Linux x86_64 | AppImage / .deb（Tauri bundle） | GitHub Actions `CI`（`gui-build`） / 本地 `pnpm tauri build` |
| Windows x86_64 | .msi / .exe（Tauri bundle） | 同上 |
| macOS（Apple Silicon） | .dmg / .app（Tauri bundle） | 同上 |

无头 CLI 的 `.deb` / 交叉编译 tarball 等请用本地 `just` / `cargo`（见下文）；CI 不再构建此类产物。

---

## GitHub Actions 自动构建

### 触发方式

**方式 1：推送语义化标签（发布到 GitHub Releases）**

仅当 tag 匹配 `v*.*.*` 且变更命中 workflow 的 `paths` 过滤时，会跑完整 CI；成功后在同一运行中创建 **Desktop release**，附件为 Linux / Windows / macOS 的 Tauri 安装包。

```bash
git tag v0.4.0
git push origin v0.4.0
```

**方式 2：手动跑 CI（构建 artifact，不发 Release）**

```bash
gh workflow run CI
# 或 GitHub → Actions → CI → Run workflow
```

### 构建产物

在对应 workflow run 的 **Artifacts** 中下载（名称形如 `agent-diva-gui-<linux|windows|macos>-<arch>-<sha>`），内容为 `agent-diva-gui/src-tauri/target/release/bundle/**` 下的安装包（各平台格式由 Tauri 配置决定）。

推送 `v*.*.*` 标签且 `release` job 执行时，同一批安装包也会上传到 **GitHub Releases**。

---

## macOS 打包

### 包格式说明

| 格式 | 说明 |
|------|------|
| `.tar.gz` | 压缩的二进制文件，解压后手动安装 |
| `.dmg` | macOS 标准磁盘镜像，双击挂载后安装 |
| `.app` | 应用程序包（可选，适合 GUI 应用） |

### 本地构建 DMG

**前提条件:**
```bash
# 安装 create-dmg
brew install create-dmg
```

**构建通用二进制 + DMG:**
```bash
# 方式1: 使用脚本
./scripts/package-macos.sh

# 方式2: 使用 justfile
just build-macos-dmg
```

**手动构建:**
```bash
# 添加 Rust targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# 编译两个架构
cargo build --release --package agent-diva-cli --target x86_64-apple-darwin
cargo build --release --package agent-diva-cli --target aarch64-apple-darwin

# 创建通用二进制
lipo -create \
    target/x86_64-apple-darwin/release/agent-diva \
    target/aarch64-apple-darwin/release/agent-diva \
    -output target/universal/release/agent-diva

# 创建 DMG
create-dmg \
    --volname "Agent Diva 0.4.0" \
    --app-drop-link 400 200 \
    "dist/agent-diva-0.4.0-macos.dmg" \
    target/universal/release/
```

### 安装 DMG

1. 双击打开 DMG 文件
2. 打开终端，进入挂载目录
3. 运行安装脚本:
```bash
cd /Volumes/Agent\ Diva\ 0.4.0
./install.sh
```

或手动安装:
```bash
sudo cp agent-diva /usr/local/bin/
```

---

## 使用 cross 进行交叉编译

`cross` 是 Rust 官方推荐的交叉编译工具，需要 Docker 环境。

### 安装

```bash
cargo install cross
```

### 构建 Linux x86_64

```bash
cross build --release --target x86_64-unknown-linux-gnu -p agent-diva-cli
```

### 构建 Linux ARM64

```bash
cross build --release --target aarch64-unknown-linux-gnu -p agent-diva-cli
```

### 使用 justfile

```bash
# 安装 cross
just install-cross

# 构建 Linux x86_64
just cross-linux-x86_64

# 构建 Linux ARM64
just cross-linux-arm64
```

---

## 使用 cargo-deb 构建 DEB 包

`cargo-deb` 可以直接生成 `.deb` 安装包。

### 安装

```bash
cargo install cargo-deb
```

### 构建 DEB 包

```bash
# 在项目根目录执行
cargo deb -p agent-diva-cli

# 输出文件: target/debian/agent-diva-cli_0.4.0_amd64.deb
```

### 配置说明

`cargo-deb` 配置位于 `agent-diva-cli/Cargo.toml`:

```toml
[package.metadata.deb]
maintainer = "Agent Diva Team"
license-file = ["../LICENSE", "0"]
assets = [
    ["target/release/agent-diva", "usr/bin/agent-diva", "755"],
    ["../LICENSE", "usr/share/doc/agent-diva/LICENSE", "644"],
    ["../README.md", "usr/share/doc/agent-diva/README.md", "644"],
]
```

### 安装 DEB 包

```bash
# 安装
sudo dpkg -i agent-diva-cli_0.4.0_amd64.deb

# 或使用 apt (自动处理依赖)
sudo apt install ./agent-diva-cli_0.4.0_amd64.deb

# 卸载
sudo apt remove agent-diva
```

---

## 本地构建指南

### Linux 环境

```bash
# 安装依赖
sudo apt-get install build-essential pkg-config libssl-dev

# 构建
cargo build --release -p agent-diva-cli

# 生成 DEB 包
cargo deb -p agent-diva-cli
```

### Windows 环境

```powershell
# 仅 CLI
cargo build --release -p agent-diva-cli

# 输出: target\release\agent-diva.exe
```

**桌面 GUI 安装包（NSIS + MSI，一键）**

依赖：`cargo`、`pnpm`、`python`、系统自带的 `curl.exe`（用于预取 NSIS，降低 Tauri 在线下载超时概率）。

```powershell
# 仓库根目录
.\scripts\package-windows-gui.ps1

# 或通过 just
just package-windows-gui
```

可选开关：`-SkipCargo`、`-SkipPrepare`、`-SkipNsisPrecache`、`-SkipPnpmInstall`（说明见脚本注释或 `Get-Help .\scripts\package-windows-gui.ps1`）。产物通常在 `target\release\bundle\nsis\`、`target\release\bundle\msi\` 与 `target\release\agent-diva-gui.exe`。

### macOS 环境

```bash
# 构建
cargo build --release -p agent-diva-cli

# 通用二进制 (x86_64 + ARM64)
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo build --release -p agent-diva-cli --target x86_64-apple-darwin
cargo build --release -p agent-diva-cli --target aarch64-apple-darwin

# 合并通用二进制
lipo -create \
    target/x86_64-apple-darwin/release/agent-diva \
    target/aarch64-apple-darwin/release/agent-diva \
    -output target/release/agent-diva
```

---

## 使用 justfile 命令

```bash
# 查看所有可用命令
just --list

# 常用命令
just build-release      # 构建 release 版本
just package-linux      # 打包 Linux 版本
just package-windows-gui  # Windows GUI 安装包 (NSIS + MSI)
just build-deb          # 生成 DEB 包 (需要 cargo-deb)
just trigger-build      # 触发 GitHub Actions 构建
```

---

## 目录结构

```
agent-diva/
├── .github/
│   └── workflows/
│       └── ci.yml               # CI + 桌面三平台 Tauri 构建；tag 时发 Release
├── agent-diva-cli/
│   └── Cargo.toml               # 包含 cargo-deb 配置
├── contrib/
│   └── systemd/
│       └── agent-diva.service   # systemd 服务文件
├── dist/
│   └── linux/                   # Linux 安装脚本
└── scripts/
    ├── package-linux.sh         # Linux 打包脚本
    └── package-windows-gui.ps1  # Windows GUI 一键打包
```

---

## 常见问题

### Q: 为什么 Windows 上不能直接生成 DEB 包?

A: DEB 是 Debian Linux 的包格式，需要在 Linux 环境中构建。推荐使用:
- GitHub Actions (最简单)
- WSL2
- Docker + cross

### Q: 如何验证构建产物?

```bash
# Linux 验证
file target/release/agent-diva
ldd target/release/agent-diva

# DEB 包验证
dpkg-deb -I target/debian/*.deb
dpkg-deb -c target/debian/*.deb
```

### Q: 如何更新版本号?

1. 更新 `Cargo.toml` 中的 `version`
2. 更新 `justfile` 中的版本号
3. 更新 `dist/linux/install.sh` 中的版本号
4. 创建新标签: `git tag v0.3.0`
