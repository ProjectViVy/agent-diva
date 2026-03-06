## Iteration Summary

- **Iteration**: `2026-03-app-building-gui-installer`
- **Version**: `v0.0.1-ca-dist-gui-installer`
- **Scope**: CA-DIST-GUI-INSTALLER（桌面 GUI 安装器产物）首轮落地，打通 GUI 安装包 → CLI companion binary → Windows Service 安装选项的端到端链路。

### Delivered

- **GUI bundling 基线**
  - 固定 `agent-diva-gui/src-tauri/tauri.conf.json` 的产品名、标识符与多平台 `bundle.targets`，统一图标与资源路径。
  - 新增 `agent-diva-gui/public/app-icon.svg` 与 `pnpm tauri icon` 流程，自动生成 `src-tauri/icons/*` 图标集。
  - 引入 `scripts/ci/prepare_gui_bundle.py`，在 GUI 打包前自动将 `agent-diva` CLI（以及可选的 `agent-diva-service`）整理到 `src-tauri/resources/bin/<platform>/`。

- **Windows Service 能力闭环**
  - 新增 `agent-diva-service` crate，基于 `windows-service` 封装 `AgentDivaGateway` Windows 服务入口，内部拉起 `agent-diva gateway run`。
  - 在 `agent-diva-cli` 中新增 `service` 子命令（`install / start / stop / restart / uninstall / status --json`），作为 GUI / 安装器与 Windows Service 之间的桥接层。
  - 在 `agent-diva-gui/src-tauri/windows/hooks.nsh` 中接入 NSIS 安装 hook，实现“安装并启动 Agent Diva 网关系统服务”的可选勾选项（受控降级，缺少 service 二进制时会显式提示并跳过安装）。

- **GUI 控制面服务管理**
  - 在 `agent-diva-gui/src-tauri/src/commands.rs` 中新增：
    - `get_runtime_info`：返回 `platform` / `is_bundled` / `resource_dir`，用于前端判断运行模式。
    - `get_service_status` / `install_service` / `uninstall_service` / `start_service` / `stop_service`：通过定位随包 `agent-diva` 二进制并调用 `agent-diva service *` 子命令，完成服务管理。
  - 在 `agent-diva-gui/src/components/settings/GeneralSettings.vue` 中新增最小服务管理面板：
    - 在 General 设置页下方展示当前运行模式、服务安装状态与可见的安装/启动/停止/卸载按钮。
    - 在非 Tauri（纯浏览器/故事书）环境下自动降级为只读说明，避免前端报错。

- **文档与 CI 衔接**
  - 扩展 `docs/app-building/wbs-distribution-and-installers.md` 中 `CA-DIST-GUI-INSTALLER` 段落，补齐：
    - `WP-DIST-GUI-01/02/03/04` 的代码级命令片段与目录约定；
    - GUI bundling 前的 `bundle:prepare` 流程与图标/资源生成步骤；
    - Windows 安装器与 service 安装行为的受控降级说明。
  - 更新 `docs/app-building/wbs-ci-cd-and-automation.md`，在 `WP-CI-MATRIX-02` 中接入 `scripts/ci/prepare_gui_bundle.py` 和可选 `agent-diva-service` 构建。
  - 更新 `docs/app-building/wbs-validation-and-qa.md`，将 `CA-QA-SMOKE-DESKTOP` 与 GUI artifacts 命名规范、服务安装路径以及 GUI 服务管理面板回归（`WP-QA-REG-00`）对齐。
  - 更新 `docs/app-building/README.md` 中阶段建议，将 `CA-DIST-GUI-INSTALLER` 标记为当前进行中阶段，并显式指出本次迭代的主文档与核心动作。

### Impact

- **类型**：GUI 打包配置 + Windows Service 封装 + CLI/GUI 服务管理桥接 + CI/QA 文档对齐。
- **影响范围**：
  - 代码：`agent-diva-service`、`agent-diva-cli`（service 子命令）、`agent-diva-gui/src-tauri`（commands + hooks）与 `agent-diva-gui` 前端 General 设置页。
  - 文档：`docs/app-building/README.md`、`wbs-distribution-and-installers.md`、`wbs-ci-cd-and-automation.md`、`wbs-validation-and-qa.md`、`docs/windows-standalone-app-solution.md`。
  - CI：`.github/workflows/ci.yml` 的 `gui-build` job 增强。
- **不涉及**：
  - 核心业务逻辑（`agent-diva-core` / `agent-diva-agent` / `agent-diva-providers` / `agent-diva-channels` / `agent-diva-tools` 内部算法与对外接口）；
  - 生产级 Release workflow（`CA-CI-ARTIFACTS` 后续版本）与自动化 smoke job 的完整实现。

