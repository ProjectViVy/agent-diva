# Acceptance

## Product Acceptance Steps

> 本文档从“产品/交付视角”对 `v0.0.1-macos-gui-bundle` 进行接受性检查，重点在于：  
> 用户是否可以在 macOS 上通过单一 dmg 获取可用的 GUI + 网关体验，并在需要时启用长期运行模式。

1. **文档与脚本对齐**
   - 打开 `docs/user-guide/commands.md`：
     - 在“平台构建与打包指南（GUI + CLI）”章节中，确认包含：
       - macOS 一键脚本：`scripts/build-macos-gui-bundle.sh` 的使用说明；
       - Windows GUI 安装包与 Linux/CLI 构建的基本路径。
   - 打开 `scripts/build-macos-gui-bundle.sh`：
     - 确认脚本步骤与文档描述一致：CLI 构建 → 资源准备 → Tauri 打包。
   - 打开 `scripts/ci/prepare_gui_bundle.py`：
     - 确认 `--target-os macos` 时，会将 CLI 复制到 `src-tauri/resources/bin/macos/agent-diva`，并生成 manifest 与 launchd 相关模板路径。

2. **macOS 构建与产物检查**
   - 在 macOS 主机上，从仓库根目录执行：
     - `chmod +x scripts/build-macos-gui-bundle.sh`
     - `./scripts/build-macos-gui-bundle.sh`
   - 期望结果：
     - 脚本执行顺序清晰、无中断错误；
     - 产物存在于：
       - `agent-diva-gui/src-tauri/target/release/bundle/macos/Agent Diva.app`
       - `agent-diva-gui/src-tauri/target/release/bundle/dmg/Agent Diva_<version>.dmg`
     - `agent-diva-gui/src-tauri/resources/bin/macos/agent-diva` 存在。

3. **安装与 GUI 一键启动网关（E2E smoke）**
   - 双击 `.dmg` 安装 `Agent Diva.app` 到 `Applications`；
   - 从 Launchpad 启动应用：
     - 首次启动时，如系统提示“来自未受信任开发者”，按 macOS 指引在“隐私与安全性”中放行一次；
   - 在 GUI 中：
     - 打开网关控制面板；
     - 点击“启动网关”按钮：
       - 预期 GUI 能在几秒内完成 `agent-diva gateway run` 子进程的拉起；
       - “网关状态”显示为“运行中”，且健康检查（`check_health`）为 OK；
     - 尝试发送一条简单对话，看是否能得到响应（可使用本地或远端 provider）。

4. **（可选）LaunchAgent 服务模式验收**
   - 在 macOS 主机上：
     - `cd contrib/launchd`
     - `./install.sh` 安装用户级 LaunchAgent；
     - 重启当前用户 session 或显式执行 `launchctl start com.agent-diva.gateway`；
   - 预期结果：
     - 机器重启后，`com.agent-diva.gateway` 仍通过 LaunchAgent 自动启动；
     - GUI 中的“服务状态”页面可以反映当前 launchd 状态（Installed/Loaded 等），并允许通过 `start_service` / `stop_service` 与之交互。

## Acceptance Result

- **从代码与脚本视角**：
  - macOS 平台的 GUI + CLI 一体化打包链路已经具备：
    - 一键构建脚本；
    - 清晰的资源准备与 Tauri 配置；
    - 与 GUI 内部网关控制逻辑（`start_gateway` 等）的契约对齐。
- **从产品交付视角（本迭代结论）**：
  - 当前版本可以视为“macOS GUI 一体打包的基础版本（foundation）”，满足：
    - 用户通过单一 dmg 安装即可获得 GUI 与内置网关能力；
    - 具备向长运行模式（launchd）演进的脚本与接口基础；
    - 文档层面已覆盖构建与安装的关键步骤，便于后续 CI/QA 及最终用户文档扩展。 

