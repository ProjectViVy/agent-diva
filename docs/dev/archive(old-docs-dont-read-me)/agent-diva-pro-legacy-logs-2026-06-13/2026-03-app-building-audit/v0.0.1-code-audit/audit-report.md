# Agent Diva app-building 文档与实现代码审计报告

> **审计范围**：`docs/app-building` 目录下的 WBS 文档、相关 `docs/logs` 迭代日志、以及对应代码实现  
> **审计日期**：2026-03-07  
> **审计方式**：先分析，不改代码

---

## 1. 执行摘要

### 1.1 总体结论

- **完成度**：大部分 WBS 工作包已落地，核心链路（CI 矩阵、GUI 打包、Headless 打包、Windows 服务、服务管理面板）均有实现。
- **主要问题**：存在若干**实现与 WBS 契约不一致**的缺口，以及 **Release 流程重复/触发条件潜在缺陷**。
- **建议**：优先修复 Headless 包契约缺口与 Release 触发逻辑，再补齐缺失的迭代日志。

### 1.2 工作项完成情况概览

| CA / 阶段 | 计划状态 | 实际状态 | 备注 |
|-----------|----------|----------|------|
| CA-HL-CLI-GATEWAY | 已完成 | ✅ 已完成 | `agent-diva gateway run` 已落地 |
| CA-CI-MATRIX | 已完成 | ✅ 已完成 | 三平台 rust-check / gui-build / headless-build |
| CA-GUI-ARCH / CA-GUI-CMDS | 已完成 | ✅ 已完成 | ServiceManagementPanel、Tauri commands |
| CA-HL-WIN-SERVICE | 已完成 | ✅ 已完成 | agent-diva-service + CLI service 子命令 |
| CA-HL-LNX-SYSTEMD | 已完成 | ✅ 已完成 | contrib/systemd + package_headless 入包 |
| CA-HL-MAC-LAUNCHD | 已完成 | ✅ 已完成 | contrib/launchd + package_headless 入包 + commands 对接 |
| CA-DIST-GUI-INSTALLER | 进行中 | ✅ 基本完成 | tauri.conf、hooks.nsh、prepare_gui_bundle |
| CA-DIST-CLI-PACKAGE | 并行推进 | ⚠️ 部分缺口 | 见下文 BUG-01、BUG-02 |
| CA-CI-ARTIFACTS | 已落地 | ⚠️ 存在风险 | 见下文 BUG-03、BUG-04 |

---

## 2. 已确认完成的步骤

### 2.1 CI 与构建

- **WP-CI-MATRIX-01**：三平台 `rust-check`（`just fmt-check`、`just check`、`just test`）✅
- **WP-CI-MATRIX-02**：GUI 构建矩阵，含 `prepare_gui_bundle.py`、可选 `agent-diva-service` 构建 ✅
- **WP-CI-MATRIX-03**：Headless 构建矩阵，使用 `package_headless.py` ✅
- **CA-HL-WIN-SERVICE 增量验证**：Windows 下 `service install/status/uninstall --dry-run` ✅

### 2.2 GUI 与服务管理

- **WP-GUI-CMDS-00/01/02/03/04**：`commands.rs` 中实现 `get_runtime_info`、`get_service_status`、`install_service`、`uninstall_service`、`start_service`、`stop_service` ✅
- **WP-GUI-ARCH-SMP-01/02**：`GeneralSettings.vue` 中 ServiceManagementPanel、`desktop.ts` API 封装 ✅
- **WP-DIST-GUI-01/02**：`tauri.conf.json` 多平台 targets、`hooks.nsh` 可选服务安装 ✅

### 2.3 Headless 与服务模板

- **contrib/systemd**：`agent-diva.service`、`install.sh`、`uninstall.sh` 存在 ✅
- **contrib/launchd**：`com.agent-diva.gateway.plist`、`install.sh`、`uninstall.sh` 存在 ✅
- **package_headless.py**：`bin/` 结构、`README.md`、`bundle-manifest.txt`、Linux systemd / macOS launchd 入包 ✅

### 2.4 迭代日志

- `2026-03-app-building-phase1`（v0.0.1-ca-ci-matrix-foundation、v0.0.2-gui-bundle-foundation）✅
- `2026-03-app-building-gui-installer`（v0.0.1-ca-dist-gui-installer）✅
- `2026-03-headless-cli-package`（v0.0.1-tech-enhanced-wbs）✅
- `2026-03-headless-service`（v0.0.1-ca-hl-lnx-systemd-baseline）✅
- `2026-03-ca-gui-arch`（v0.0.1-service-management-panel）✅
- `2026-03-ca-gui-cmds`（v0.0.1-ca-gui-cmds-complete）✅
- `2026-03-ci-artifacts-release`（v0.0.1-ca-ci-artifacts）✅

---

## 3. 发现的 BUG 与缺口

### BUG-01：Headless 包缺少 `config/config.example.json` 与 `services/README.md`

**WBS 契约**（`wbs-headless-cli-package.md` Phase 1 强制文件）：

- `config/config.example.json`
- `services/README.md`

**实际实现**（`scripts/ci/package_headless.py`）：

- 未创建 `config/` 目录
- 未复制或生成 `config.example.json`
- 未创建 `services/README.md`

**影响**：

- `headless-bundle-quickstart.md` 与随包 `README.md` 中声明的 Bundle Contents 与实际包内容不一致
- 用户按文档操作时，无法找到 `config/config.example.json`

**建议**：

1. 在仓库中新增 `config/config.example.json` 模板（或从现有配置生成）
2. 在 `package_headless.py` 中增加对 `config/` 与 `services/README.md` 的打包逻辑
3. 或明确将 Phase 1 中 `config_example`、`services/README.md` 调整为可选，并同步更新 WBS 与 quickstart

---

### BUG-02：`bundle-manifest.txt` 字段与 WBS 契约不一致

**WBS 契约**（`wbs-headless-cli-package.md` 3.3 节）：

```text
name=agent-diva
version=0.0.0
os=windows
arch=x86_64
entrypoint=bin/agent-diva.exe gateway run
service_mode=optional
config_example=config/config.example.json
readme=README.md
```

**实际实现**（`package_headless.py` 中 `write_manifest`）：

- 有：`version`、`os`、`arch`、`binary`、`entrypoint`、`systemd_files`、`launchd_files`
- 缺：`name`、`service_mode`、`config_example`、`readme`

**影响**：

- 依赖 `bundle-manifest.txt` 的 smoke/校验脚本可能无法按契约解析
- 与 `wbs-validation-and-qa.md`、`release-artifacts.yml` 的校验逻辑不一致

**建议**：

- 在 `write_manifest` 中补充 `name=agent-diva`、`service_mode=optional`、`config_example`、`readme` 等字段
- 若 `config_example` 暂不提供，可写为 `config_example=` 或按实际存在性条件写入

---

### BUG-03：`release-artifacts.yml` 在 tag 推送时可能不触发

**实现**（`.github/workflows/release-artifacts.yml`）：

- 触发：`workflow_run`（CI 完成后）
- 条件：`startsWith(github.event.workflow_run.head_branch, 'v')`

**问题**：

- 对 **tag 推送**（如 `v0.2.0`），`workflow_run.head_branch` 可能为空或非 tag 名
- GitHub 文档：`head_branch` 为「触发 workflow 的分支名」，tag 推送无分支概念
- 若 `head_branch` 为空，`startsWith('', 'v')` 为 false，Release Artifacts 不会执行

**影响**：

- 仅通过 tag 推送发布时，`release-artifacts.yml` 可能不运行
- 与「tag 推送即发布」的预期不符

**建议**：

- 增加对 `workflow_run.head_ref` 或 ref 的检查，或使用 `github.event.workflow_run.conclusion` 配合 ref 判断
- 或改为：tag 推送时由 `ci.yml` 的 release job 统一负责发布，并明确 `release-artifacts.yml` 仅用于 `workflow_dispatch` 补发

---

### BUG-04：`ci.yml` 与 `release-artifacts.yml` 的 Release 逻辑重复

**现状**：

- `ci.yml`：存在 `release` job，在 `startsWith(github.ref, 'refs/tags/v')` 时下载 artifacts 并调用 `softprops/action-gh-release`
- `release-artifacts.yml`：从 CI 的 artifacts 整理为 `dist/gui`、`dist/headless` 后发布

**问题**：

- 两个 workflow 均可能对同一 tag 创建/更新 Release
- 产物结构不同：`ci.yml` 直接上传原始 artifacts，`release-artifacts.yml` 使用规范化 `dist/` 结构并做校验
- 可能导致冲突或行为不清晰

**建议**：

- 明确单一发布入口：要么只用 `ci.yml` release job，要么只用 `release-artifacts.yml`
- 若保留 `release-artifacts.yml`，建议从 `ci.yml` 中移除 release job，避免重复发布

---

### 缺口-01：`windows-standalone-app-solution.md` 引用但可能过时

**README**（`docs/app-building/README.md`）阶段 2 中引用：

- `windows-standalone-app-solution.md`

**说明**：

- 文件存在于 `docs/windows-standalone-app-solution.md`
- 需确认其内容与当前 `tauri.conf.json`、`hooks.nsh`、`prepare_gui_bundle` 行为一致
- 若已过时，应更新或移除引用

---

### 缺口-02：macOS 服务管理文档与实现不一致

**文档**（`wbs-gui-cross-platform-app.md` 实现状态表）：

- macOS：`install_service` / `uninstall_service` 等「当前返回'待接入'」

**实现**（`commands.rs`）：

- macOS 已实现 `macos_service_status`、`install_service`、`uninstall_service`、`start_service`、`stop_service`
- `contrib/launchd` 与 `package_headless.py` 已支持 macOS

**说明**：

- 实现已超出文档描述，文档处于滞后状态
- `ui-ca-gui-arch-service-management-panel.md` 中 macOS 仍为「受控降级」提示，若产品策略为暂不开放，可保留；否则应同步更新 WBS 与 UI 设计文档

---

### 缺口-03：部分迭代缺少完整四件套

**规范**（`AGENTS.md` iteration-log-required）：

- 每个版本目录应包含：`summary.md`、`verification.md`、`release.md`、`acceptance.md`

**检查结果**：

- `2026-03-headless-gateway-phase1`、`2026-03-windows-standalone-app` 等已有对应文件
- 建议对 `docs/logs` 下所有 app-building 相关迭代做一次统一检查，确保四件套齐全

---

## 4. 验证建议（不改代码，仅执行验证）

### 4.1 本地验证

```bash
# 1. 基础质量门
just fmt-check && just check && just test

# 2. GUI 开发模式
cd agent-diva-gui && pnpm install --frozen-lockfile && pnpm tauri dev

# 3. Headless 打包（检查产物结构）
cargo build -p agent-diva-cli --release
python scripts/ci/package_headless.py --binary target/release/agent-diva.exe --version 0.2.0 --os windows --arch x86_64 --output-dir dist --readme docs/app-building/headless-bundle-quickstart.md
# 检查 dist/ 下压缩包内是否有 config/、services/README.md、bundle-manifest.txt 字段

# 4. GUI bundle 准备
cd agent-diva-gui && pnpm run bundle:prepare
# 检查 src-tauri/resources/bin/<platform>/ 与 manifests/gui-bundle-manifest.json
```

### 4.2 CI 与 Release 验证

- 在测试分支推送 tag（如 `v0.0.0-audit-test`），观察：
  - `ci.yml` 的 `release` job 是否执行
  - `release-artifacts.yml` 是否被触发
  - 两个 workflow 是否对同一 tag 重复创建 Release

---

## 5. 总结与后续动作建议

| 优先级 | 问题 | 建议动作 |
|--------|------|----------|
| P0 | BUG-01：Headless 包缺 config/services | 补齐 `config/config.example.json`、`services/README.md` 的打包逻辑，或调整 WBS 为可选 |
| P0 | BUG-02：bundle-manifest 字段不全 | 在 `package_headless.py` 中补全 `name`、`service_mode`、`config_example`、`readme` |
| P1 | BUG-03：release-artifacts 触发条件 | 修正 `workflow_run` 条件下对 tag 推送的兼容 |
| P1 | BUG-04：Release 逻辑重复 | 明确单一发布入口，移除或禁用冗余 release job |
| P2 | 缺口-01：windows-standalone-app-solution | 核对并更新文档 |
| P2 | 缺口-02：macOS 文档与实现 | 同步 WBS 与 UI 设计文档 |
| P2 | 缺口-03：迭代日志四件套 | 补全缺失的 summary/verification/release/acceptance |

---

*本报告仅做分析，未修改任何代码。*
