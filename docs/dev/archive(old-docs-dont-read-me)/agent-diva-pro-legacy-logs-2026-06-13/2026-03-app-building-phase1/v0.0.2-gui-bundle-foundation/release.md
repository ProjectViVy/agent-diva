# Release

## Release Type

- 内部研发基线发布（GUI 打包与服务管理桥接配置），用于后续 CI / QA / 分发控制账户的输入。

## Deployment Method

- 不直接发布到外部 Release 页面或制品库，仅在仓库中保留：
  - 已知可在 Windows 环境构建的 GUI 安装包（NSIS + MSI）；
  - 稳定的 Tauri 配置与资源目录布局；
  - 与 `agent-diva-cli service` 子命令的服务管理契约。
- 建议在后续 `CA-CI-ARTIFACTS` / `CA-DIST-GUI-INSTALLER` 阶段：
  - 复用当前 `bundle/` 目录结构与命名规则；
  - 在 tag/release workflow 中增加 GUI 安装包 artifact 下载与发布。

## Artifacts (Local / CI-Ready)

> 以下为在本迭代中于 Windows 开发环境实际生成的产物路径，尚未自动发布到远端：

- GUI 安装包（Windows）：
  - `target/x86_64-pc-windows-msvc/release/bundle/nsis/Agent Diva_0.1.0_x64-setup.exe`
  - `target/x86_64-pc-windows-msvc/release/bundle/msi/Agent Diva_0.1.0_x64_en-US.msi`
- 运行时资源（供安装器使用）：
  - `agent-diva-gui/src-tauri/resources/bin/windows/agent-diva.exe`
  - `agent-diva-gui/src-tauri/resources/manifests/gui-bundle-manifest.json`
- 图标资产：
  - `agent-diva-gui/src-tauri/icons/icon-source.svg`
  - `agent-diva-gui/src-tauri/icons/icon.png`
  - `agent-diva-gui/src-tauri/icons/icon.ico`
  - `agent-diva-gui/src-tauri/icons/icon.icns`
  - 以及 `32x32.png`、`64x64.png`、`128x128.png`、`128x128@2x.png` 等平台所需变体。

## Follow-up Release Suggestion

后续建议按以下路径演进：

- `CA-CI-ARTIFACTS`：
  - 在 CI tag/release workflow 中，基于 `v0.0.1-ca-ci-matrix-foundation` 的矩阵，下载 GUI 构建 job 的 artifacts；
  - 将本次迭代固化的 bundle 目录结构作为 Release 产物命名与布局的模板。

- `CA-DIST-GUI-INSTALLER`：
  - 在 `wbs-distribution-and-installers.md` 中，以本迭代的 Tauri 配置与资源目录为基础，细化：
    - Windows 安装器的升级/回滚/卸载行为；
    - 可选安装 Windows Service 的 UX 与错误提示；
    - 多语言安装文本（若有需要）。

- `CA-QA-SMOKE-DESKTOP`：
  - 在后续 QA 阶段，使用当前产物路径作为 smoke 与回归测试的输入：
    - 安装/启动/卸载验证；
    - 与 Windows Service smoke 的组合路径（当 `agent-diva-service` crate 完全落地后）。
