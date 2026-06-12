# Verification

## Validation Scope

- 本次迭代聚焦于在 macOS 上落地 GUI + CLI 一体化 dmg 打包链路，并给出稳定的一键脚本入口；
- 目标是在不修改 CI 的前提下，明确本地可执行的验证步骤，后续可直接迁移到 macOS CI runner。

## Commands (Recommended on macOS host)

> 由于当前环境中 `just` 未安装，本次验证在沙箱内只完成了局部命令演练。以下为**推荐在真实 macOS 主机上实际执行的完整验证步骤**，用于确认脚本与打包链路可用。

1. **构建 CLI（用于内置网关）**
   - 在 workspace 根目录：
     - `cd /Users/mastwet/agent-diva`
     - `cargo build --release -p agent-diva-cli`
   - 预期结果：
     - 生成 `target/release/agent-diva`；
     - 无编译错误。

2. **为 macOS 准备 GUI 资源**
   - 仍在根目录：
     - 推荐使用 release 产物：
       - `python3 scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os macos`
     - 或显式指定 debug 产物（开发阶段）：
       - `python3 scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os macos --workspace-root . --cli-binary target/debug/agent-diva`
   - 预期结果：
     - `agent-diva-gui/src-tauri/resources/bin/macos/agent-diva` 存在且可执行；
     - 存在 `agent-diva-gui/src-tauri/resources/manifests/gui-bundle-manifest.json`；
     - 如存在 `contrib/launchd`，manifest 中 `macos_launchd` 字段指向 `resources/launchd`。

3. **构建 GUI 安装包 (.app / .dmg)** 
   - 进入 GUI 目录：
     - `cd /Users/mastwet/agent-diva/agent-diva-gui`
     - `pnpm install`（首次或依赖有变更时执行）
     - `pnpm tauri build`
   - 预期结果：
     - 构建成功，无致命错误；
     - 产物存在于：
       - `.app`：`src-tauri/target/release/bundle/macos/Agent Diva.app`
       - `.dmg`：`src-tauri/target/release/bundle/dmg/Agent Diva_<version>.dmg`
     - `.app` 内容中 `Resources/bin/macos/agent-diva` 被正确打包。

4. **GUI 一键启动网关 smoke（人工执行）**
   - 从 `.dmg` 安装 `Agent Diva.app` 到 `Applications`；
   - 双击启动 GUI：
     - 在设置 / 控制面板中点击“启动网关”（或等价按钮）；
     - 观察：
       - GUI 能通过 `start_gateway` 拉起内置 `agent-diva gateway run` 子进程；
       - 状态面板可以通过 `get_gateway_process_status` 显示“运行中/未运行”；
       - `check_health` 通过本地 HTTP / SSE 接口确认网关可用。

5. **（可选）macOS LaunchAgent 服务模式**
   - 在主机上运行：
     - `cd /Users/mastwet/agent-diva/contrib/launchd`
     - `./install.sh`（安装用户级 LaunchAgent）
     - `launchctl list | grep com.agent-diva.gateway`（确认已加载）
     - 如需卸载：`./uninstall.sh`
   - 预期结果：
     - `~/Library/LaunchAgents/com.agent-diva.gateway.plist` 存在且内容指向正确的 `agent-diva` 路径；
     - 日志目录 `~/Library/Logs/agent-diva` 中生成 `gateway.log` / `gateway.error.log`（有活动时）。

## Sandbox Observations (This Run)

- 在受限沙箱环境中已完成：
  - `cargo build --release -p agent-diva-cli --bin agent-diva`（验证依赖完整性与 CLI 二进制可构建性）；
  - 使用 debug 产物调用：
    - `python3 scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os macos --workspace-root . --cli-binary target/debug/agent-diva`
    - 成功生成 `resources/bin/macos/agent-diva` 与对应 manifest；
  - 尝试 `pnpm tauri build` 时，由于沙箱注入的 `--ci=1` 导致 CLI 参数解析报错，此问题源自运行环境，而非项目配置本身。

## Conclusion

- 从代码与脚本链路上看，macOS 下的 GUI + CLI 一体化打包已经具备完整路径：
  - CLI 构建 → 资源准备 → Tauri 打包 → GUI 控制内置网关；
  - 同时预留了通过 LaunchAgent 的长期运行模式（由 GUI 的 service commands 或脚本触发）。
- 建议在真实 macOS 主机上按上述步骤至少完整执行一次，以便在后续 CI / Release 集成时可以直接复用这些命令作为 smoke 与构建脚本的基线。 

