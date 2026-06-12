# Summary

## Iteration

- Name: `2026-03-app-building-macos-gui-bundle`
- Version: `v0.0.1-macos-gui-bundle`
- Scope: 在现有 `CA-GUI-BUNDLE` 基础上，完成 macOS GUI dmg 前后端一体打包链路与用户向导补全。

## Delivered

- **macOS GUI 一体化打包（GUI + CLI）**
  - 新增脚本 `scripts/build-macos-gui-bundle.sh`：
    - 在 workspace 根目录构建 `agent-diva-cli`（`cargo build --release -p agent-diva-cli`）；
    - 调用 `scripts/ci/prepare_gui_bundle.py --target-os macos` 将 `agent-diva` 二进制复制到 `agent-diva-gui/src-tauri/resources/bin/macos/agent-diva`，并生成 manifest；
    - 在 `agent-diva-gui` 下执行 `pnpm install && pnpm tauri build`，生成 macOS `.app` / `.dmg`。
  - 复用既有 Tauri 配置：
    - `bundle.targets` 已包含 `"app"` 与 `"dmg"`；
    - `bundle.resources = ["resources/"]` 负责将 `resources/bin/macos/agent-diva` 等资源一并打包进 `.app` / `.dmg`。
  - 通过 `agent-diva-gui/src-tauri/src/commands.rs` 现有逻辑：
    - 在打包环境下，优先从 `ResourceDir/bin/macos/agent-diva` 解析 CLI；
    - GUI 通过 `start_gateway` / `get_gateway_process_status` / `stop_gateway` 实现一键启动/停止本地网关子进程。

- **macOS LaunchAgent 服务脚本对齐**
  - 对现有 `contrib/launchd/install.sh` / `uninstall.sh` 与 `com.agent-diva.gateway.plist` 做轻量复核：
    - 默认以用户级 LaunchAgent 方式安装 `com.agent-diva.gateway`；
    - 允许从 GUI 调用 `install_service` / `start_service` / `stop_service` / `uninstall_service` 时在 macOS 上走 `launchd` 流程。
  - `prepare_gui_bundle.py` 对 macOS 的 `service_templates` 输出指向 `resources/launchd`，与上述脚本布局保持一致。

- **用户向导文档补全**
  - 在 `docs/user-guide/commands.md` 末尾新增“平台构建与打包指南（GUI + CLI）”章节：
    - 说明 macOS 一键脚本使用方式与产物路径；
    - 概述 Windows GUI 安装包（NSIS/MSI）构建步骤与可选服务安装行为；
    - 补充 Linux/其他平台下 CLI 构建与打包的基本入口；
    - 给出“先本地 smoke，再接 CI”的推荐实践。

## Impact

- 类型：打包脚本新增 + GUI 资源准备链路在 macOS 上的具体化 + 用户文档增强。
- 影响范围：
  - 代码/脚本：`scripts/build-macos-gui-bundle.sh`、`scripts/ci/prepare_gui_bundle.py`（作为 macOS 调用目标）、`agent-diva-gui/src-tauri/resources/*`（manifest 与 bin/macos）。
  - 文档：`docs/user-guide/commands.md` 新增平台构建与打包说明。
- 不涉及：
  - 核心业务逻辑（内核、Agent、Providers、Channels、Tools）的行为更改；
  - CI workflow 配置本身（仅提供可直接复用的本地命令组合）。 

