---
title: Agent Diva 跨平台独立应用构建文档集
---

## 概览

- **目标一（GUI 独立应用）**：在 Windows / macOS / Linux 上，通过 `agent-diva-gui`（Tauri 2 + Vue + Vite + Tailwind）构建“一次构建，多端运行”的桌面控制面应用，用于管理本地/远程 Agent Diva 网关。
- **目标二（Headless 纯后端）**：基于 `agent-diva-cli` + 各平台服务机制（Windows Service / systemd / launchd），提供无图形界面、可长期运行的网关部署形态。
- **方法论**：所有构建与打包工作都以“技术增强版 WBS”组织：每个控制账户（CA）与工作包（WP）都明确技术路线、代码/配置级实践方案以及测试与验收方式。

> 注意：本目录下的所有 WBS 文档**默认面向 Agent 本身**（包括主 Agent 与子 Agent），描述“当你作为构建执行器时，应该如何一步步完成构建/打包/测试工作”。人类工程师阅读时，可以把文中的“你”理解为“负责执行这些步骤的 Agent”。

## 文档索引

- **`docs/app-building/README.md`（本文件）**
  - 使用者：**Agent Diva / 子 Agent（负责全局规划）**
  - 内容：
    - 跨平台独立应用构建的整体目标与范围
    - GUI 模式与 Headless 模式的架构关系
    - 其余 WBS 文档的入口索引与适用场景

- **`docs/app-building/wbs-gui-cross-platform-app.md`**
  - 使用者：**负责 GUI 控制面与打包的 Agent / 子 Agent**
  - 覆盖范围：
    - GUI 控制面架构（与 gateway / manager 的通信方式）
    - Tauri commands 设计与 Rust 后端 crate 集成方式
    - 跨平台打包配置（Windows MSI/NSIS、macOS app/dmg、Linux deb/appimage 等）
    - GUI bundling 前置校验、图标生成、`bundle:prepare` 资源 staging
    - 与 CI artifact / 分发安装器 / 桌面 smoke 的输入输出映射
    - 针对 GUI 构建与安装的 smoke test、E2E 验证方案

- **`docs/app-building/wbs-headless-service-mode.md`**
  - 使用者：**负责 Headless 运行模式与服务化的 Agent / 子 Agent**
  - 覆盖范围：
    - `agent-diva gateway run` 与相关 CLI 子命令的运行模式
    - Windows Service 封装（推荐通过新增 `agent-diva-service` crate + `windows-service` crate 实现）
    - Linux systemd unit 文件设计与安装脚本
    - macOS launchd LaunchAgent / LaunchDaemon Plist 模板与管理命令
    - 各平台服务形态的启动 / 停止 / 重启 / 日志查看 / 开机自启验证

- **`docs/app-building/wbs-distribution-and-installers.md`**
  - 使用者：**负责分发与安装器流程的 Agent / 子 Agent**
  - 覆盖范围：
    - 基于 Tauri bundler 的 GUI 安装器产物（Windows NSIS/MSI、macOS dmg、Linux deb/appimage）
    - Headless CLI / 服务二进制的跨平台打包（zip / tar.gz）与命名规范
    - 安装过程中的自定义动作：复制二进制、写入默认配置、可选安装服务、创建快捷方式等
    - 升级 / 回滚 / 卸载策略及对应的文件与数据目录约定

- **`docs/app-building/wbs-headless-cli-package.md`**
  - 使用者：**负责 `CA-DIST-CLI-PACKAGE` 的 Agent / 子 Agent**
  - 覆盖范围：
    - Headless 压缩包的固定命名、目录结构与 `bundle-manifest.txt` 契约
    - Phase 1 到 Phase 3 的交付边界（最小包、服务模板入包、Release 固化）
    - PowerShell/Bash 打包脚本片段、CI 工件上传与 Release 门禁
    - Headless 包的 smoke/QA 映射与阶段验收标准

- **`docs/app-building/wbs-ci-cd-and-automation.md`**
  - 使用者：**在 CI/CD 环境中扮演构建/发布角色的 Agent / 子 Agent**
  - 覆盖范围：
    - 多平台构建矩阵（Windows / macOS / Linux）与缓存策略
    - GUI 与 Headless 构建任务的分层（基础测试 -> 构建 -> 打包 -> 发布工件）
    - 自动化安装 / 启动验证脚本的集成方式
    - 构建工件上传与发布（如 GitHub Releases 或内部制品库）

- **`docs/app-building/ui-ca-gui-arch-service-management-panel.md`**
  - 使用者：**负责 CA-GUI-ARCH 服务管理 UI 的 Agent / UI 设计师**
  - 覆盖范围：
    - 设置页 ServiceManagementPanel 区域的服务生命周期管理 UI 设计
    - 布局结构、交互流程、状态机、i18n 键、主题适配与验收标准
    - 与 WP-GUI-CMDS-00 / WP-GUI-CMDS-04 的接口映射

- **`docs/app-building/wbs-validation-and-qa.md`**
  - 使用者：**负责验证与 QA 流程的 Agent / 子 Agent**
  - 覆盖范围：
    - 基于平台与运行模式的测试矩阵（GUI / Headless × Windows / macOS / Linux）
    - 安装 / 首次启动 / 长期运行 / 升级 / 回滚 / 卸载的场景化测试用例
    - 与仓库规则对齐的基础验证：`just fmt-check`、`just check`、`just test`
    - GUI smoke test（如：`cargo tauri build` 后在目标平台启动 GUI 并执行关键路径）
    - 服务 smoke test（如：安装服务后验证自动启动、日志写入、健康检查端点）

- **`docs/app-building/headless-bundle-quickstart.md`**
  - 使用者：**负责 Headless artifact 打包的 Agent / 子 Agent**
  - 覆盖范围：
    - 第一阶段 `CA-CI-MATRIX` 生成的最小 Headless 压缩包随包说明
    - `bin/agent-diva(.exe) gateway run` 的最短启动路径
    - 作为后续 `CA-DIST-CLI-PACKAGE` 正式 README 的 Phase 1 占位模板

## 运行模式与文档映射

- **桌面 GUI 模式（适用于普通桌面用户）：**
  - 主要参考：
    - `wbs-gui-cross-platform-app.md`
    - `wbs-distribution-and-installers.md`
    - `wbs-validation-and-qa.md`（与 GUI 相关部分）
  - 目标：用户拿到 GUI 安装包即可完成安装、首次启动、查看与管理本地 Agent Diva 网关。

- **Headless 纯后端模式（适用于服务器 / 无头环境）：**
  - 主要参考：
    - `wbs-headless-service-mode.md`
    - `wbs-distribution-and-installers.md`
    - `wbs-headless-cli-package.md`
    - `wbs-ci-cd-and-automation.md`
    - `wbs-validation-and-qa.md`（与服务相关部分）
  - 目标：在不依赖 GUI 的前提下，将 Agent Diva 作为长期运行的守护进程 / 系统服务部署，并通过 CLI 与 Manager API 进行管理。

## 阶段建议

- **阶段 0（已完成）`CA-HL-CLI-GATEWAY`：**
  - `agent-diva gateway run` 已收敛为统一的 Headless 标准入口，可作为后续服务化、分发与 CI 自动化的共同上游。

- **阶段 1（已完成）`CA-CI-MATRIX`：**
  - 三平台 Rust 校验、GUI bundles 与 Headless bundles 已具备 CI 基线，可作为后续安装器与 QA 的稳定输入。

- **阶段 2（当前进行中）`CA-DIST-GUI-INSTALLER`：**
  - 当前由本轮实施计划驱动，主文档为：
    - `wbs-distribution-and-installers.md`
    - `windows-standalone-app-solution.md`
    - `wbs-validation-and-qa.md`
  - 当前阶段的核心动作：
    - 固化 `agent-diva-gui/src-tauri/tauri.conf.json` 的多平台 bundle 目标与品牌配置；
    - 用 `scripts/ci/prepare_gui_bundle.py` 在打包前把 `agent-diva` CLI 二进制整理到 `src-tauri/resources/`；
    - 为 Windows NSIS 安装器接入可选服务安装 hook，并为 macOS / Linux 补齐安装、卸载与 smoke 映射。

- **阶段 3（并行推进）`CA-DIST-CLI-PACKAGE`：**
  - 继续把 Headless artifact 固化为独立分发包、随包 README 与服务模板。
  - 其中专项实施以 `wbs-headless-cli-package.md` 为主文档，统一包结构、随包 README、CI 工件命名与 Release 门禁。

- **阶段 4（依赖安装器/分发包稳定后）**：
  - 当前已落地 `CA-CI-ARTIFACTS` 的第一版实现：
    - 在 `wbs-ci-cd-and-automation.md` 中补齐 CA/WP 定义与版本/tag 策略；
    - 新增 `.github/workflows/release-artifacts.yml`，从 `CI` workflow 的 artifacts 生成 Release 资产；
    - 在 `wbs-distribution-and-installers.md` 与 `wbs-validation-and-qa.md` 中补充 Release 获取方式与 Release 验收 checklist。
  - 后续可在该基础上继续推进 `CA-QA-SMOKE-DESKTOP`、`CA-QA-SMOKE-HEADLESS`，将 Release 资产纳入自动化 smoke 与人工验收闭环。

- **GUI 衔接阶段（可与阶段 2 并行推进）`CA-GUI-BUNDLE`：**
  - 以 `wbs-gui-cross-platform-app.md` 中的 `WP-GUI-BUNDLE-00/01/02/03/04` 为执行主线：
    - 先校对 workspace/Node 依赖与锁文件状态；
    - 再固化 `tauri.conf.json`、图标资源与 `bundle:prepare`；
    - 最后把 GUI bundle 目录结构对齐到 CI / 分发 / QA 文档。
  - 这样可以在不侵入核心 Rust 业务模块的情况下，把桌面端安装包构建能力补齐为可验证、可移交、可复用的工程资产。

## 最小侵入性与能力可达性说明

- **最小侵入性**：
  - 所有 WBS 文档默认约束：尽量不修改 `agent-diva-core` / `agent-diva-agent` / `agent-diva-providers` / `agent-diva-channels` / `agent-diva-tools` 的对外接口与核心行为。
  - 平台相关逻辑推荐集中在：
    - `agent-diva-gui`（Tauri commands + 前端页面）
    - `agent-diva-cli`（新增 `gateway` / `service` 等子命令）
    - 新增 `agent-diva-service` crate 及系统级脚本与配置模板（systemd / launchd）。

- **能力可达性**：
  - 技术路线严格基于当前项目已使用或主流的组件：Rust + Tokio、Tauri 2、`windows-service`、systemd、launchd 等。
  - 每个 CA/WP 都要求给出**可直接复制使用**的代码/配置片段与命令行示例，确保工程团队可以“照文档实现”，而不是停留在概念设计层面。

