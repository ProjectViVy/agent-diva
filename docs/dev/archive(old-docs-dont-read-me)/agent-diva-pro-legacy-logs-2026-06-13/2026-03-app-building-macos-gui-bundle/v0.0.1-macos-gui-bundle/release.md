# Release

## Release Type

- 内部研发基线发布（macOS GUI dmg + 内置 CLI），作为桌面端跨平台打包方案在 macOS 平台上的第一版可执行实现；
- 主要面向开发者与后续 CI/分发控制账户，不直接对终端用户公开发布。

## Deployment Method

- 本迭代不修改现有 CI workflow，也不自动上传 Release 产物，仅在本地提供一条稳定的打包路径：
  - 通过 `scripts/build-macos-gui-bundle.sh` 一键构建：
    - CLI：`cargo build --release -p agent-diva-cli`
    - GUI 资源准备：`python3 scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os macos`
    - GUI 打包：`cd agent-diva-gui && pnpm install && pnpm tauri build`
- 建议在后续 CI 迭代中：
  - 在 GitHub Actions 等 macOS runner 上复用上述命令；
  - 将 `agent-diva-gui/src-tauri/target/release/bundle/macos/*.app` 与 `bundle/dmg/*.dmg` 作为 Release artifacts 附加到 tag 发布。

## Artifacts (Local / CI-Ready)

> 以下路径为期望在真实 macOS 主机上执行脚本后得到的本地产物清单。

- GUI 安装包（macOS）：
  - `.app`：
    - `agent-diva-gui/src-tauri/target/release/bundle/macos/Agent Diva.app`
  - `.dmg`：
    - `agent-diva-gui/src-tauri/target/release/bundle/dmg/Agent Diva_<version>.dmg`

- 内置 CLI 资源（供 GUI / 服务脚本使用）：
  - `agent-diva-gui/src-tauri/resources/bin/macos/agent-diva`
  - `agent-diva-gui/src-tauri/resources/manifests/gui-bundle-manifest.json`

- macOS LaunchAgent 模板与脚本：
  - `contrib/launchd/com.agent-diva.gateway.plist`
  - `contrib/launchd/install.sh`
  - `contrib/launchd/uninstall.sh`
  - （当通过 `prepare_gui_bundle.py` 为 macOS 目标准备资源时）同步到：
    - `agent-diva-gui/src-tauri/resources/launchd/`

## Follow-up Release Suggestion

- **CI 集成**：
  - 在 macOS runner 的 release workflow 中增加一个 job：
    - 检出仓库；
    - 执行 `scripts/build-macos-gui-bundle.sh`；
    - 上传 `.app` 与 `.dmg` 为 Release artifacts；
  - 将当前文档中的命令顺序固化为 CI 步骤，避免脚本与流水线行为漂移。

- **分发与文档联动**：
  - 在面向用户的安装文档中（如未来的 GUI 使用指南）引用本迭代产出的 macOS dmg 路径与安装方式；
  - 与 Windows 安装包说明（`v0.0.2-gui-bundle-foundation`）一并形成跨平台桌面发行矩阵。

- **质量门槛提升**：
  - 后续可在 `CA-QA-SMOKE-DESKTOP` 中为 macOS 增加：
    - dmg 安装/卸载 smoke；
    - GUI 内一键启动网关、查看健康状态的 E2E 测试；
    - LaunchAgent 安装/重启/卸载 smoke（可复用 `contrib/launchd` 脚本和 GUI 的 service commands）。 

