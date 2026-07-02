# agent-diva Windows 独立 App 打包与网关服务化方案

## 1. 目标与约束

目标：将 `agent-diva` 交付为可独立安装的 Windows App，用户安装后即可获得可长期运行的网关能力，且满足以下两种体验之一：

1. App 启动后自动拉起内置网关（前台 App + 后台子进程）。
2. 安装阶段或首次启动阶段自动注册 Windows Service（系统服务常驻）。

约束：

- 保持现有 Rust workspace 结构，不引入破坏性重构。
- 优先复用现有 `agent-diva-gui`（Tauri）、`agent-diva-cli`、`agent-diva-manager`。
- 默认最小权限运行，只有“安装系统服务”动作需要管理员权限提升。

## 2. 推荐架构（双模式并存）

建议提供双模式，让普通用户零门槛，高级用户可切换服务化：

- 模式 A（默认）：`agent-diva-gui` 启动时拉起 `agent-diva gateway run` 子进程，并通过本地 IPC/HTTP 管理。
- 模式 B（可选）：GUI 调用 CLI 执行 `service install/start`，注册 `AgentDivaGateway` 系统服务并脱离 GUI 常驻。

建议新增一个轻量 crate：`agent-diva-service`（仅封装服务安装与生命周期），避免把平台细节散落到 GUI/CLI。

## 3. 组件改造建议

### 3.1 `agent-diva-cli`

新增子命令：

- `agent-diva gateway run`
- `agent-diva gateway status`
- `agent-diva service install --auto-start`
- `agent-diva service start|stop|restart|uninstall`

实现建议：

- 非管理员执行 `service install` 时返回明确提示并引导 UAC 提权。
- `service install` 默认 `Automatic (Delayed Start)`，避免开机抢占。

### 3.2 `agent-diva-service`（新增）

职责：

- Windows Service 主入口（`windows-service` crate）。
- SCM 状态上报（Starting/Running/StopPending/Stopped）。
- 统一启动 `agent-diva gateway run` 的 Tokio runtime。
- 处理 Stop/Shutdown 事件并优雅退出。

### 3.3 `agent-diva-gui`（Tauri）

新增能力：

- 首次启动向导：选择“仅当前用户后台运行”或“安装为系统服务”。
- 服务管理页面：安装/启动/停止/重启/卸载服务，展示运行状态和最近日志。
- 健康检查：每 10~30 秒探活本地网关健康端点，不通则提示修复操作。

### 3.4 `agent-diva-manager`

用于 GUI 与网关间的稳定控制面接口：

- `/health`：健康状态。
- `/runtime`：进程与版本信息。
- `/ops/reload`：热重载配置。
- `/ops/drain`：优雅停机。

## 4. 打包与安装设计（Windows）

推荐安装形态：

- 主包：`agent-diva-gui` 生成 NSIS/MSI（Tauri bundler）。
- 附带二进制：`agent-diva-cli.exe`、`agent-diva-service.exe`（或统一单二进制多子命令）。

当前仓库中的最小可执行落地方式如下：

- `agent-diva-gui/src-tauri/tauri.conf.json`
  - 已固定 `productName = "Agent Diva"`、`identifier = "com.agentdiva.desktop"`（避免与 macOS `.app` 扩展冲突）。
  - 已启用 `bundle.targets = ["nsis", "msi", "app", "dmg", "deb", "appimage"]`。
  - 已启用 `bundle.icon = [...]`，图标由 `src-tauri/icons/icon-source.svg` 通过 `tauri icon` 生成多平台资产。
  - 已启用 `bundle.resources = ["resources/"]`，供 CLI/Service 二进制入包。
- `scripts/ci/prepare_gui_bundle.py`
  - 在 `cargo build -p agent-diva-cli --release` 后，将 `target/release/agent-diva.exe` 复制到 `agent-diva-gui/src-tauri/resources/bin/windows/`。
  - 若 `target/release/agent-diva-service.exe` 已存在，也一并复制；若不存在，则记录到 manifest 并允许安装器降级运行。
- `agent-diva-gui/src-tauri/windows/hooks.nsh`
  - 为 NSIS 安装器增加“是否安装 Windows Service”的可选页。
  - 在用户勾选且资源二进制存在时，执行 `agent-diva.exe service install --auto-start` 与 `agent-diva.exe service start`。

安装流程建议：

1. 安装程序复制二进制与默认配置模板到 `Program Files\AgentDiva\`。
2. 写入用户数据目录 `%ProgramData%\AgentDiva\`（服务模式）或 `%USERPROFILE%\.agent-diva\`（用户模式）。
3. 用户选择“安装系统服务”时执行提权自定义动作：`agent-diva service install --auto-start`。
4. 安装完成后可选择“立即启动 GUI”与“立即启动网关”。

建议把上述流程映射到当前 CA/WP：

- `Phase 1` 对齐 `WP-DIST-GUI-01`
  - 完成 GUI 主安装包、图标、资源目录和 CLI 二进制入包。
- `Phase 2` 对齐 `WP-DIST-GUI-02` + `CA-HL-WIN-SERVICE`（WP-HL-WIN-00/01/02）
  - 通过 NSIS hook 增加服务安装复选框；`agent-diva-service` 与 `agent-diva service *` 已落地，安装器可直接调用完整服务化路径。
- `Phase 3` 对齐 `WP-QA-DESKTOP-01` 与 `WP-QA-HEADLESS-02`
  - 补齐升级、回滚、卸载残留与服务启停 smoke 验证。

### 与 CA-HL-WIN-SERVICE 的映射

| Phase | CA-HL-WIN-SERVICE WP | 说明 |
|-------|----------------------|------|
| Phase 1 | - | GUI 安装包与 CLI 入包，不涉及服务 |
| Phase 2 | WP-HL-WIN-01, WP-HL-WIN-02, WP-HL-WIN-00 | `agent-diva-service` crate、CLI `service` 子命令、GUI/Tauri commands 与 NSIS hook 集成 |
| Phase 3 | 验收 | 安装器完成服务注册后的 E2E 验证 |

升级策略：

- 升级前执行 `service stop`，替换二进制后 `service start`。
- 保留配置与会话数据目录，不覆盖用户密钥与历史记录。

回滚策略：

- 安装器保留上一个版本二进制（`backup/<version>/`）用于一键回退。

## 5. 运行与安全基线

- 服务账户：优先 `LocalService`，仅在确有文件/网络需求时调整权限。
- 日志：分离 GUI 日志与网关日志，支持大小轮转。
- 密钥：统一走环境变量或受控配置文件，GUI 不明文展示。
- 本地控制接口仅绑定 `127.0.0.1`，并启用随机 token（首次生成并持久化）。

## 6. 借鉴 `.workspace/openclaw` 的可复用实践

从 sibling 项目可直接迁移的思想：

- 网关作为常驻核心进程，Cron/Hook 等调度逻辑在网关内运行而非 UI 线程。
- 自动化与运维文档强调“可长期运行 + 可观测 + 可恢复”，适合 `agent-diva` 的网关定位。
- 通过 CLI + 网关 API 双控制面，降低 GUI 失效时的运维风险。

这些实践与当前 `agent-diva-core`（heartbeat/cron/event bus）架构方向一致。

## 7. 分阶段落地计划

### Phase 1（1~2 周）：最小可用独立 App

- GUI 可启动/停止内置网关子进程。
- 完成本地健康检查与日志查看。
- 输出安装包（不含系统服务自动安装）。

### Phase 2（1~2 周）：系统服务化

- 完成 `agent-diva-service` 与 `agent-diva service *` 子命令。
- 安装器接入提权动作，支持安装后自动注册服务。
- 增加故障自恢复与开机自启动验证。

### Phase 3（1 周）：可运维与发布质量

- 补齐升级/回滚流程与文档。
- 补齐 smoke test：安装、首次启动、服务重启、卸载残留检查。
- CI 增加 Windows 打包产物与基础安装校验。

## 8. 验收标准（面向你当前目标）

- 用户拿到一个 `Windows 安装包`，无需手工装依赖即可运行。
- GUI 可直接看到并控制网关状态。
- 可选一键安装系统服务，重启机器后网关仍可自动运行。
- 文档中明确了架构、命令、目录、升级回滚与安全边界。

## 9. 当前实现边界说明

- 当前仓库已具备：
  - Tauri 多平台 bundle 配置；
  - GUI 打包前自动整理 CLI companion binary；
  - Windows NSIS 安装器的服务安装 hook（调用 `agent-diva.exe service install --auto-start` 与 `service start`）；
  - `agent-diva-service` crate：以子进程方式托管 `agent-diva gateway run`，支持 Stop/Shutdown 优雅退出；
  - `agent-diva.exe service *` 完整子命令：Install、Start、Stop、Restart、Uninstall、Status（含 `--json` 输出）；
  - GUI Tauri commands：`get_runtime_info`、`get_service_status`、`install_service`、`uninstall_service`、`start_service`、`stop_service`，通过调用 CLI 实现；
  - 与 `docs/app-building/wbs-distribution-and-installers.md`、`docs/app-building/wbs-validation-and-qa.md` 的阶段映射。
- 待完善：
  - 安装器完成服务注册后的真实 end-to-end 验证（需在 Windows VM 中执行）；
  - CI 中为 Service 能力增加 dry-run 级别验证。

因此，Windows 安装器与 `CA-HL-WIN-SERVICE` 已具备完整实现，安装时勾选“安装 Windows 服务”即可完成服务注册与启动。
