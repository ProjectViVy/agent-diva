## Verification

### Scope

本次验证聚焦于 **文档与实现是否一致**，不包含完整的多平台打包与安装 smoke，仅检查：

- WBS 中命令片段与真实仓库文件/脚本是否对齐；
- 新增 crate / 命令 / Tauri commands 与文档的引用是否一一对应；
- CI 配置（`ci.yml`）是否引用了正确的脚本与路径。

### Manual Checks

1. **配置与脚本位置**
   - `agent-diva-gui/src-tauri/tauri.conf.json`：
     - `productName = "Agent Diva"`；
     - `bundle.targets` 包含 `nsis` / `msi` / `app` / `dmg` / `deb` / `appimage`；
     - `bundle.resources = ["resources/"]`；
     - `bundle.windows.nsis.installerHooks = "./windows/hooks.nsh"`.
   - `scripts/ci/prepare_gui_bundle.py`：
     - 支持 `--gui-root`、`--workspace-root`、`--target-os` 参数；
     - 缺省情况下使用 `target/release/agent-diva(.exe)` 作为 CLI 源；
     - 将二进制放置到 `agent-diva-gui/src-tauri/resources/bin/<platform>/`；
     - 生成 `resources/manifests/gui-bundle-manifest.json`。
   - `agent-diva-gui/src-tauri/windows/hooks.nsh`：
     - 定义 NSIS hooks，显示 “Install and start Agent Diva Gateway as a Windows Service” 复选框；
     - 在 `NSIS_HOOK_POSTINSTALL` 中检测 `$INSTDIR\resources\bin\windows\agent-diva.exe` 与 `agent-diva-service.exe` 是否存在，并在缺失时给出提示。

2. **服务封装与 CLI 子命令**
   - `agent-diva-service`：
     - 已加入 workspace `Cargo.toml` 的 `members`；
     - `src/main.rs` 使用 `windows-service`，实现 `SERVICE_NAME = "AgentDivaGateway"` 的 Windows Service 入口；
     - 在 console 模式下支持 `--console` 参数本地验证（直接拉起 `agent-diva gateway run` 子进程）。
   - `agent-diva-cli`：
     - `Cargo.toml` 在 `cfg(windows)` 下依赖 `windows-service`；
     - `src/service.rs` 定义 `service install/start/stop/restart/uninstall/status --json` 子命令；
     - `src/main.rs` 将 `Service { command: ServiceCommands }` 接入 CLI `Commands` 枚举。

3. **GUI 与服务管理 Tauri commands**
   - `agent-diva-gui/src-tauri/src/commands.rs`：
     - 新增 `RuntimeInfo` 与 `ServiceStatusPayload`；
     - `get_runtime_info` 返回 `platform` / `is_bundled` / `resource_dir`；
     - `get_service_status` / `install_service` / `uninstall_service` / `start_service` / `stop_service` 通过定位随包 `agent-diva` 并调用 `agent-diva service *` 实现服务管理；
     - `lib.rs` 中通过 `tauri::generate_handler!` 注册了上述 commands。
   - `agent-diva-gui/src/components/settings/GeneralSettings.vue`：
     - 使用 `invoke('get_runtime_info')` / `invoke('get_service_status')` / `invoke('install_service')` 等命令；
     - 在没有 Tauri runtime（如浏览器故事书）场景下自动降级，不会报错。

4. **CI 集成（不执行实际 CI，仅对照配置）**
   - `.github/workflows/ci.yml`：
     - `gui-build` job 中在 `pnpm install` 前增加：
       - `cargo build -p agent-diva-cli --release`；
       - 如存在 `agent-diva-service/Cargo.toml`，则构建 `agent-diva-service`；
       - `python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui` 以整理 GUI 资源。
     - 与 `wbs-ci-cd-and-automation.md` 中 `WP-CI-MATRIX-02` 的 YAML 片段保持一致。

### Results

- 文档中引用的核心文件路径、命令行示例与仓库当前实现保持一致；
- CA-DIST-GUI-INSTALLER 相关的 WBS（分发 / CI / QA）已具备可复制执行的“先决条件 → 实施步骤 → 测试与验收”结构；
- 本轮未针对多平台安装器与 Windows Service 做实际 smoke（由产品/QA 后续按 WBS 执行），但代码路径和文档契约已经对齐，为后续自动化和人工验收提供了稳定基线。

