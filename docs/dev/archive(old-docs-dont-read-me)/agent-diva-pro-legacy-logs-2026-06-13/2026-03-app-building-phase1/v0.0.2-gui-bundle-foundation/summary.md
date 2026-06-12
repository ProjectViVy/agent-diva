# Summary

## Iteration

- Name: `2026-03-app-building-phase1`
- Version: `v0.0.2-gui-bundle-foundation`
- Scope: `CA-GUI-BUNDLE` 第一阶段（Windows 平台为主）的打包与构建链路落地

## Delivered

- `agent-diva-gui` 侧：
  - 固定 Tauri 元信息：`productName = "Agent Diva"`、`identifier = "com.agentdiva.desktop"`，避免与 macOS `.app` 扩展冲突。
  - 将 `bundle.targets` 从 `"all"` 收敛为明确的 `["nsis","msi","app","dmg","deb","appimage"]`，与 GUI WBS / 分发 WBS 对齐。
  - 在 `bundle.icon` 中统一使用由 `src-tauri/icons/icon-source.svg` 通过 `pnpm tauri icon` 生成的多平台图标资产。
  - 启用 `bundle.resources = ["resources/"]` 与 Windows NSIS `installerHooks = "./windows/hooks.nsh"`，作为 CLI/Service 二进制入包与服务安装选项的承载点。
  - 在 `src-tauri/src/lib.rs` 中注册 GUI 所需的所有 Tauri commands（包括消息发送流、配置/工具配置更新、健康检查与服务管理）。
  - 在 `src-tauri/src/commands.rs` 中补齐：
    - SSE 流事件到前端的结构化 payload（`StreamTextPayload` / `StreamToolStartPayload` / `StreamToolFinishPayload`）；
    - `get_runtime_info`：暴露平台、是否打包、资源目录；
    - Windows-only 的服务管理桥接：通过定位 `agent-diva.exe` 并调用 `service status --json` / `service install --auto-start` / `service start|stop|uninstall`。

- 前端与锁文件：
  - `package.json`：新增 `bundle:prepare` 脚本，显式调用 `scripts/ci/prepare_gui_bundle.py` 为 Tauri 安装器准备 `resources/bin/<os>/agent-diva(.exe)`。
  - `pnpm-lock.yaml`：同步 `vue-i18n` 等依赖，使锁文件与现有代码使用保持一致。
  - 通过 `pnpm tauri icon src-tauri/icons/icon-source.svg --output src-tauri/icons` 生成完整图标集，消除手工维护多格式图标的不一致性风险。

- Windows 独立 App 文档：
  - 更新 `docs/windows-standalone-app-solution.md`，使其中对 `tauri.conf.json` 的描述与当前实现一致（产品名、identifier、bundle.targets、resources、图标来源）。

## Impact

- 类型：GUI 打包配置 + 服务管理桥接代码 + 前端依赖/资产修正 + Windows 打包方案文档更新。
- 影响范围：
  - 代码：`agent-diva-gui/src-tauri/tauri.conf.json`、`src-tauri/src/lib.rs`、`src-tauri/src/commands.rs`、`agent-diva-gui/package.json`、`agent-diva-gui/pnpm-lock.yaml`、`agent-diva-gui/src-tauri/icons/*`、`agent-diva-gui/src-tauri/resources/*`、`agent-diva-gui/src-tauri/windows/hooks.nsh`。
  - 文档：`docs/app-building/wbs-gui-cross-platform-app.md`、`docs/app-building/wbs-distribution-and-installers.md`、`docs/app-building/README.md`、`docs/windows-standalone-app-solution.md`。
- 不涉及：
  - 核心业务模块（`agent-diva-core` / `agent-diva-agent` / `agent-diva-providers` / `agent-diva-channels` / `agent-diva-tools`）的行为变更；
  - CI workflow 文件本身（仍沿用 `v0.0.1-ca-ci-matrix-foundation` 的配置）；
  - 跨平台 GUI smoke 自动化与 Release 发布流程（由后续 CI/QA 控制账户接手）。
