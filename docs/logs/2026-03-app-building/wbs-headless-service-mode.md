---
title: Agent Diva Headless 服务模式构建 WBS
---

> 使用说明（面向 Agent）：  
> 当你（Agent 或子 Agent）负责“纯后端/服务化部署”时，本文件给出了在不同平台上如何一步步完成 CLI 入口、Windows Service、Linux systemd 与 macOS launchd 部署的执行脚本。  
> 按 CA → WP 的顺序选择目标，再依次执行“先决条件 → 实施步骤 → 测试与验收”即可。

## 1. 控制账户（CA）概览

- **CA-HL-CLI-GATEWAY：gateway 纯后端运行模式**
  - 目标：通过 `agent-diva` 可执行文件，在无 GUI 的环境中以前台/后台方式运行网关。
  - 边界：不依赖任何桌面组件；配置与会话目录与 GUI 模式共享。

- **CA-HL-WIN-SERVICE：Windows Service 守护进程**
  - 目标：在 Windows 上将 Agent Diva 网关以 Windows Service 形式长期运行，支持开机自启与优雅停机。
  - 边界：只封装服务生命周期，不改动核心业务逻辑。
  - **实现状态**：已完成。`agent-diva-service` crate 与 `agent-diva-cli service *` 子命令已落地，GUI Tauri commands 与 NSIS 安装器 hook 已接入。

- **CA-HL-LNX-SYSTEMD：Linux systemd 服务**
  - 目标：在常见 Linux 发行版上通过 systemd 管理 Agent Diva 守护进程。

- **CA-HL-MAC-LAUNCHD：macOS launchd 服务**
  - 目标：在 macOS 上通过 LaunchAgent/LaunchDaemon 管理 Agent Diva 守护进程。
  - **实现状态**：已完成。`contrib/launchd/` 提供 Plist 模板与 install/uninstall 脚本，`package_headless.py` 将 launchd 文件纳入 macOS Headless 包，GUI Tauri commands 已接入 install/uninstall/start/stop。

所有 WP 采用统一结构：**概述 → 先决条件 → 实施步骤 → 测试与验收**。

### CA-HL-WIN-SERVICE 实现状态记录（与 优先级.md 同步）

| WP | 状态 | 说明 |
|----|------|------|
| WP-HL-WIN-00 | 已完成 | GUI 通过 Tauri commands 调用 `agent-diva.exe service status --json` / `install --auto-start` / `uninstall` / `start` / `stop`；CLI 输出 JSON schema：`{installed, running, state, executable_path}` |
| WP-HL-WIN-01 | 已完成 | `agent-diva-service` crate 已落地，通过子进程方式托管 `agent-diva gateway run`，支持 SCM 状态机与优雅停机 |
| WP-HL-WIN-02 | 已完成 | `agent-diva-cli` 提供 `service install|start|stop|restart|uninstall|status` 子命令，基于 `windows-service::ServiceManager` |

**与其它 CA 的接口点：**

- **CA-DIST-GUI-INSTALLER**：NSIS hook（`hooks.nsh`）在用户勾选时执行 `agent-diva.exe service install --auto-start` 与 `service start`；二进制路径约定为 `$INSTDIR\resources\bin\windows\`。
- **CA-GUI-CMDS**：Tauri commands 在 `is_bundled` 时按平台桥接服务能力：
  - Windows：通过 `run_service_cli` 调用 `agent-diva.exe service *`；
  - Linux：通过 `systemctl` 状态查询与 `resources/systemd/*.sh` 安装脚本执行；
  - macOS：通过 `resources/launchd/*.sh` 与 `launchctl` 执行 install/uninstall/start/stop，状态探测已接入。

---

## 2. CA-HL-CLI-GATEWAY：gateway 纯后端运行模式

### WP-HL-CLI-01：定义 gateway CLI 入口与运行方式

- **概述**
  - 为 Headless 模式定义清晰的 CLI 入口（例如 `agent-diva gateway run`），用于在前台/后台运行网关。

- **先决条件**
  - `agent-diva-cli` 已存在并作为 workspace 中的二进制 crate。

- **实施步骤**
  1. 在 `agent-diva-cli/src/main.rs` 或对应子模块中，使用 `clap`/`argh` 等库定义 `gateway` 子命令。第一阶段至少落地 `run`，并允许把裸命令 `agent-diva gateway` 兼容映射到 `run`：

     ```rust
     #[derive(clap::Subcommand)]
     pub enum Commands {
       Gateway {
         #[clap(subcommand)]
         cmd: Option<GatewayCmd>,
       },
       // 其他子命令...
     }

     #[derive(clap::Subcommand)]
     pub enum GatewayCmd {
       Run,
       // ...
     }
     ```

  2. 在 `GatewayCmd::Run` 分支中调用统一的网关启动函数（如 `agent_diva_manager::gateway_main().await` 或等价入口），并确保：
     - 使用 Tokio 多线程 runtime；
     - 读取默认配置与会话目录；
     - 优雅处理 Ctrl+C 信号。

  3. 在帮助信息中明确说明该命令是 Headless 模式入口：

     ```bash
     agent-diva gateway run    # 以当前配置启动网关（前台阻塞）
     ```

- **测试与验收**
  - 在开发机终端运行：

    ```bash
    cargo run -p agent-diva-cli -- gateway run
    ```

  - 预期：
    - 进程启动后阻塞在前台，日志正常输出；
    - 按 Ctrl+C 后能优雅退出（释放端口、刷新日志）。

---

### WP-HL-CLI-02：提供简易后台运行脚本示例

- **概述**
  - 为无 systemd/launchd 的最小场景提供“后台运行”示例脚本（不等同于系统服务）。

- **先决条件**
  - CLI 子命令 `gateway run` 已可用。

- **实施步骤**
  1. 在文档中提供 Windows PowerShell 示例：

     ```powershell
     # 后台启动
     Start-Process -FilePath "agent-diva.exe" -ArgumentList "gateway run" -WindowStyle Hidden

     # 简易停止（示例：按名称杀进程，生产环境应更精细）
     Stop-Process -Name "agent-diva" -Force
     ```

  2. 提供 Linux/macOS shell 示例：

     ```bash
     nohup agent-diva gateway run > ~/.agent-diva/logs/gateway.out 2>&1 &
     echo $! > ~/.agent-diva/gateway.pid

     # 停止
     kill "$(cat ~/.agent-diva/gateway.pid)"
     ```

- **测试与验收**
  - 在各自平台按脚本运行一次：
    - 确认进程在关闭终端后仍然存在；
    - 确认日志文件有内容；
    - 按脚本停止后进程消失。

---

## 3. CA-HL-WIN-SERVICE：Windows Service 守护进程

### 实现状态记录

| WP | 状态 | 落仓文件 / 说明 |
|----|------|-----------------|
| WP-HL-WIN-00 | 已完成 | `agent-diva-gui/src-tauri/src/commands.rs`：`get_service_status`、`install_service`、`uninstall_service`、`start_service`、`stop_service` 通过 `agent-diva.exe service *` 调用 |
| WP-HL-WIN-01 | 已完成 | `agent-diva-service/src/main.rs`：Windows Service 入口，spawn `agent-diva gateway run` 子进程，处理 Stop/Shutdown 优雅退出 |
| WP-HL-WIN-02 | 已完成 | `agent-diva-cli/src/service.rs`：`service install/start/stop/restart/uninstall/status`，含 `--json` 输出供 Tauri 解析 |

**与其它 CA 的接口**：

- **CA-DIST-GUI-INSTALLER**：NSIS hook（`windows/hooks.nsh`）在用户勾选时执行 `agent-diva.exe service install --auto-start` 与 `service start`；二进制路径约定为 `$INSTDIR\resources\bin\windows\`。
- **CA-GUI-CMDS**：Tauri commands 调用 `agent-diva.exe service status --json` 解析 `ServiceStatusPayload`；`install_service` / `uninstall_service` 等调用对应 CLI 子命令。
- **CA-CI-MATRIX**：`prepare_gui_bundle.py` 将 `agent-diva-service.exe` 入包；CI 可增加 `service status --dry-run` 验证（见 `wbs-ci-cd-and-automation.md`）。

### WP-HL-WIN-00：为 GUI 服务管理面板暴露统一的服务命令（get_service_status/install_service/uninstall_service）

- **概述**
  - 当你（Agent）需要让 GUI 服务管理面板在 Windows 平台上工作时，应确保 CLI/Service 组合支持以下操作入口，供 Tauri commands 调用：
    - 查询服务状态：是否已安装/是否正在运行；
    - 安装服务：创建 `AgentDivaGateway` 服务并配置自启动策略；
    - 卸载服务：停止并删除服务。

- **先决条件**
  - `agent-diva-service` crate 已实现 Windows Service 入口（参见后续 WP-HL-WIN-01）；
  - `agent-diva-cli` 中已可通过某种方式定位到 `agent-diva-service.exe` 所在路径（由分发 WBS 约定）。

- **实施步骤**
  1. 在 `agent-diva-cli` 中扩展 `service` 子命令逻辑，使其成为 GUI/Tauri 与 Windows Service 之间的统一桥接层（例如）：  
     - `agent-diva.exe service status`：返回当前服务是否存在和运行状态；  
     - `agent-diva.exe service install --auto-start`：安装服务并设置为自动启动；  
     - `agent-diva.exe service uninstall`：卸载服务。
  2. 在 Tauri 后端中，实现下列 commands 并调用上述 CLI 子命令（仅示意逻辑）：  
     - `get_service_status`：调用 `agent-diva.exe service status` 并解析输出成结构化 JSON（`installed`/`running`/`details`）；  
     - `install_service`：调用 `agent-diva.exe service install --auto-start`；  
     - `uninstall_service`：调用 `agent-diva.exe service uninstall`。
  3. 在实现这些 Tauri commands 时，你应先调用 `get_runtime_info` 检查：  
     - `is_bundled == true` 且 `platform == "windows"` 时才允许执行实际操作；  
     - 否则返回带有“仅在打包 Windows 应用中可用”的错误信息。

- **测试与验收**
  - 从 GUI 面板点击“安装服务”时，Tauri 实际触发 `agent-diva.exe service install --auto-start`，`services.msc` 中可看到 `AgentDivaGateway`；
  - 从 GUI 面板点击“卸载服务”时，对应服务被删除，下一次 `get_service_status` 返回“未安装”；
  - 在 dev 模式或非 Windows 平台调用这些操作时，不会对系统服务造成任何修改。 

### WP-HL-WIN-01：新增 agent-diva-service crate 封装服务入口

- **概述**
  - 使用 `windows-service` crate 单独封装 Windows Service 入口逻辑，避免把 SCM 相关代码散落在 CLI/核心业务中。

- **先决条件**
  - Windows 开发环境（MSVC toolchain）已就绪。

- **实施步骤**
  1. 在 workspace 根 `Cargo.toml` 中增加 crate 声明（如尚未存在）：

     ```toml
     [workspace.members]
     # ...
     "agent-diva-service"
     ```

  2. 创建 `agent-diva-service/Cargo.toml`，添加依赖：

     ```toml
     [package]
     name = "agent-diva-service"
     version = "0.1.0"
     edition = "2021"

     [dependencies]
     windows-service = "0.7"
     windows = { version = "0.58", features = [
       "Win32_Foundation",
       "Win32_System_Services",
       "Win32_Security",
     ] }
     tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
     agent-diva-manager = { path = "../agent-diva-manager" }
     ```

  3. 在 `agent-diva-service/src/main.rs` 中实现服务入口（参考调研示例）：

     ```rust
     use windows_service::{
       define_windows_service,
       service::{
         ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
         ServiceType,
       },
       service_control_handler::{self, ServiceControlHandlerResult},
       service_dispatcher,
     };

     define_windows_service!(ffi_service_main, service_main);

     fn service_main(_args: Vec<std::ffi::OsString>) {
       if let Err(e) = run_service() {
         eprintln!("service error: {e:?}");
       }
     }

     fn run_service() -> windows_service::Result<()> {
       let event_handler = move |control_event| -> ServiceControlHandlerResult {
         match control_event {
           ServiceControl::Stop => {
             // TODO: 触发优雅停机信号
             ServiceControlHandlerResult::NoError
           }
           ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
           _ => ServiceControlHandlerResult::NotImplemented,
         }
       };

       let status_handle =
         service_control_handler::register("AgentDivaGateway", event_handler)?;

       status_handle.set_service_status(ServiceStatus {
         service_type: ServiceType::OWN_PROCESS,
         current_state: ServiceState::Running,
         controls_accepted: ServiceControlAccept::STOP,
         exit_code: ServiceExitCode::Win32(0),
         checkpoint: 0,
         wait_hint: std::time::Duration::default(),
       })?;

       let rt = tokio::runtime::Runtime::new().unwrap();
       rt.block_on(async {
         // 统一网关入口
         agent_diva_manager::gateway_main().await;
       });

       Ok(())
     }

     fn main() -> Result<(), windows_service::Error> {
       service_dispatcher::start("AgentDivaGateway", ffi_service_main)?;
       Ok(())
     }
     ```

- **测试与验收**
  - 在 Windows 上以 console 模式直接运行 `agent-diva-service.exe`：
    - 确认不会 panic，且能看到网关正常启动日志；
    - 手工关闭进程时，能优雅退出。

---

### WP-HL-WIN-02：在 CLI 中提供 service 安装/控制子命令

- **概述**
  - 在 `agent-diva-cli` 中新增 `service` 子命令，用于安装/卸载/启动/停止 Windows Service。

- **先决条件**
  - `agent-diva-service` 已可编译，并放置于预期安装目录。

- **实施步骤**
  1. 在 CLI 的子命令定义中增加 `service`：

     ```rust
     #[derive(clap::Subcommand)]
     pub enum Commands {
       Service {
         #[clap(subcommand)]
         cmd: ServiceCmd,
       },
       // ...
     }

     #[derive(clap::Subcommand)]
     pub enum ServiceCmd {
       Install { #[clap(long)] auto_start: bool },
       Start,
       Stop,
       Uninstall,
     }
     ```

  2. 使用 `windows-service::service_manager::ServiceManager` 完成安装逻辑（见调研文档示例），约定：
     - 服务名：`AgentDivaGateway`；
     - DisplayName：`Agent Diva Gateway Service`；
     - 启动类型：默认 AutoStart（建议 Delayed）。

- **测试与验收**
  - 在提升权限的 PowerShell 中执行：

    ```powershell
    agent-diva.exe service install --auto-start
    agent-diva.exe service start
    ```

  - 通过 `services.msc` 或 `Get-Service AgentDivaGateway` 验证服务已安装并运行；
  - 执行 `agent-diva.exe service stop && agent-diva.exe service uninstall` 后，服务被完全移除。

---

## 4. CA-HL-LNX-SYSTEMD：Linux systemd 服务

### WP-HL-LNX-01：systemd unit 文件模板定义

- **概述**
  - 为常见 Linux 服务器场景提供标准 unit 文件模板，统一运行方式与安全基线。

- **模板来源说明**
  - 仓库中的权威模板文件位于：`contrib/systemd/agent-diva.service`。
  - 修改模板时，必须同步更新本文档中的内联示例，确保两者内容一致。
  - Headless Linux 压缩包（Phase 2 后）会将此模板与安装/卸载脚本一并打入 `systemd/` 子目录。

- **先决条件**
  - 目标系统使用 systemd（大多数现代发行版）。

- **实施步骤**
  1. 在文档中给出推荐的 unit 文件内容（与 `contrib/systemd/agent-diva.service` 保持一致）：

     ```ini
     [Unit]
     Description=Agent Diva Gateway - AI Assistant Service
     After=network-online.target
     Wants=network-online.target

     [Service]
     Type=simple
     User=agent-diva
     Group=agent-diva
     ExecStart=/usr/bin/agent-diva gateway run
     ExecReload=/bin/kill -HUP $MAINPID
     Restart=on-failure
     RestartSec=5s

     LimitNOFILE=65536
     LimitNPROC=4096

     NoNewPrivileges=true
     PrivateTmp=true
     ProtectSystem=strict
     ProtectHome=true
     ReadWritePaths=/var/lib/agent-diva /var/log/agent-diva

     Environment="RUST_LOG=info"
     Environment="AGENT_DIVA_CONFIG_DIR=/etc/agent-diva"

     [Install]
     WantedBy=multi-user.target
     ```

  2. 建议将该模板在安装时复制到 `/etc/systemd/system/agent-diva.service`。

- **测试与验收**
  - 在 Linux 服务器上执行：

    ```bash
    sudo systemctl daemon-reload
    sudo systemctl enable agent-diva
    sudo systemctl start agent-diva
    sudo systemctl status agent-diva
    ```

  - 预期：
    - 服务状态为 active (running)；
    - `journalctl -u agent-diva -f` 能看到网关日志。

---

### WP-HL-LNX-02：安装与卸载脚本示例

- **概述**
  - 提供简单的安装/卸载脚本，方便运维以“一条命令”完成部署。

- **先决条件**
  - 已将 `agent-diva` 二进制与 unit 模板打包（详见分发文档）。
  - Linux Headless 压缩包内包含 `systemd/agent-diva.service`、`systemd/install.sh`、`systemd/uninstall.sh`（由 `scripts/ci/package_headless.py` 在打包时自动纳入）。

- **模板来源**
  - 安装脚本：`contrib/systemd/install.sh`
  - 卸载脚本：`contrib/systemd/uninstall.sh`
  - 上述脚本随 Linux Headless 包打入 `systemd/` 子目录，详见 `wbs-headless-cli-package.md`。

- **实施步骤**
  1. 从 Headless 压缩包解压后，进入 `systemd/` 目录执行安装：

     ```bash
     tar -xzf agent-diva-<version>-linux-<arch>.tar.gz
     cd agent-diva-<version>-linux-<arch>/systemd
     sudo ./install.sh
     ```

  2. 卸载时在同一目录执行：

     ```bash
     cd agent-diva-<version>-linux-<arch>/systemd
     sudo ./uninstall.sh
     ```

  3. 脚本行为概要：
     - `install.sh`：将 `bin/agent-diva` 安装到 `/usr/bin/agent-diva`，创建 `/var/lib/agent-diva` 与 `/var/log/agent-diva`，安装 unit 文件并启用、启动服务；
     - `uninstall.sh`：停止并禁用服务，删除 unit 与二进制，**默认保留** `/var/lib/agent-diva` 与 `/var/log/agent-diva`（避免误删业务数据）。

- **测试与验收**
  - 在干净的 Linux VM 上执行 `install.sh`，确认服务成功安装并运行；
  - 执行 `uninstall.sh` 后，确认服务已从 `systemctl list-unit-files` 中移除，二进制被删除；
  - 确认 `/var/lib/agent-diva` 与 `/var/log/agent-diva` 在卸载后仍保留。

---

## 5. CA-HL-MAC-LAUNCHD：macOS launchd 服务

### WP-HL-MAC-01：LaunchAgent Plist 模板定义

- **概述**
  - 为 macOS 用户提供以当前登录用户身份运行的 LaunchAgent 模板，在登录时自动启动 Agent Diva 网关。

- **先决条件**
  - 目标用户具备 `~/Library/LaunchAgents` 写入权限。

- **实施步骤**
  1. 提供 Plist 模板内容：

     ```xml
     <?xml version="1.0" encoding="UTF-8"?>
     <!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
     <plist version="1.0">
     <dict>
       <key>Label</key>
       <string>com.agent-diva.gateway</string>

       <key>ProgramArguments</key>
       <array>
         <string>/usr/local/bin/agent-diva</string>
         <string>gateway</string>
         <string>run</string>
       </array>

       <key>RunAtLoad</key>
       <true/>

       <key>KeepAlive</key>
       <dict>
         <key>SuccessfulExit</key>
         <false/>
         <key>Crashed</key>
         <true/>
       </dict>

       <key>StandardOutPath</key>
       <string>/usr/local/var/log/agent-diva/gateway.log</string>
       <key>StandardErrorPath</key>
       <string>/usr/local/var/log/agent-diva/gateway.error.log</string>
     </dict>
     </plist>
     ```

  2. 指导用户放置路径：`~/Library/LaunchAgents/com.agent-diva.gateway.plist`。

- **测试与验收**
  - 在 macOS 上执行：

    ```bash
    launchctl load ~/Library/LaunchAgents/com.agent-diva.gateway.plist
    launchctl start com.agent-diva.gateway
    ```

  - 验证：
    - `launchctl list | grep agent-diva` 能看到条目；
    - 日志文件按配置路径生成；
    - 注销/重启后自动拉起。

---

### WP-HL-MAC-02：LaunchDaemon（系统级）方案（可选）

- **概述**
  - 为需要系统级服务（非用户会话）的场景提供 LaunchDaemon 模板，仅在明确需求时使用。

- **先决条件**
  - 需要 root 权限配置 `/Library/LaunchDaemons`。

- **实施步骤**
  - 在文档中提供与 LaunchAgent 类似的 Plist 模板，但：
    - 放置路径改为 `/Library/LaunchDaemons/com.agent-diva.gateway.plist`；
    - 注意用户/组配置与目录权限。

- **测试与验收**
  - 仅在服务端 macOS 环境中验证：重启后服务是否自动运行，日志路径是否可写。

