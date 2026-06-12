---
title: Agent Diva 跨平台 GUI 独立应用构建 WBS（Tauri 路线）
---

> 使用说明（面向 Agent）：  
> 当你（Agent 或子 Agent）承担“GUI 构建与打包执行器”角色时，可以将本文件视为执行脚本：  
> - 先选择对应的控制账户（CA），例如 `CA-GUI-ARCH`；  
> - 再逐个按工作包（WP）中的“先决条件 → 实施步骤 → 测试与验收”顺序执行；  
> - 每个命令/代码片段都可以通过你的工具链（Shell/编辑器等）直接应用到仓库。

## 1. 控制账户（CA）概览

- **CA-GUI-ARCH：GUI 控制面架构与后端集成**
  - 目标：在不破坏现有 Rust workspace 结构的前提下，将 `agent-diva-gui` 作为“控制面板”接入现有 `agent-diva-*` crates，统一通过 Manager API / gateway 进程管理网关生命周期。
  - 边界：
    - 不在 `agent-diva-core` / `agent-diva-agent` / `agent-diva-manager` 内部做重构式修改。
    - 平台差异只体现在 GUI 打包与安装（见分发文档），不在业务逻辑层散落 `cfg(target_os)`。

- **CA-GUI-CMDS：Tauri commands 与网关通信通道**
  - 目标：在 Tauri 后端实现一组稳定的 commands，供前端 Vue 控制面调用，用于：
    - 启动 / 停止本地 gateway 进程或服务；
    - 查询健康状态 / 运行信息；
    - 读取与更新配置、查看日志。

- **CA-GUI-BUNDLE：多平台打包与安装包产物**
  - 目标：基于 Tauri bundler 一次配置，产出 Windows / macOS / Linux 的桌面安装包，满足“安装即用”的最小体验。
  - 边界：复杂安装流程（系统服务自动安装、企业级签名策略）细节落在分发 WBS 文档中，这里只定义 GUI 构建与打包基本面。

后续所有 WP 都按“**概述 → 先决条件 → 实施步骤（细化到具体命令）→ 测试与验收**”四段式给出，便于按步骤直接执行。

---

## 2. CA-GUI-ARCH：GUI 控制面架构与后端集成

### WP-GUI-ARCH-01：Tauri 后端依赖集成

- **概述**
  - 将现有 Rust workspace 中的核心 crates 以本地路径依赖方式接入 `agent-diva-gui/src-tauri`，为后续 commands / IPC 提供编译期可见的类型与函数。

- **先决条件**
  - Rust toolchain 已安装（与 workspace 其他 crate 一致）。
  - `agent-diva-gui` crate 已存在且使用 Tauri 2。

- **实施步骤**
  1. 打开 `agent-diva-gui/src-tauri/Cargo.toml`。
  2. 在 `[dependencies]` 小节中新增或确认以下依赖（路径根据实际目录层级调整）：

     ```toml
     [dependencies]
     agent-diva-core = { path = "../../agent-diva-core" }
     agent-diva-agent = { path = "../../agent-diva-agent" }
     agent-diva-providers = { path = "../../agent-diva-providers" }
     agent-diva-channels = { path = "../../agent-diva-channels" }
     agent-diva-tools = { path = "../../agent-diva-tools" }
     agent-diva-manager = { path = "../../agent-diva-manager" }

     tauri = { version = "2", features = ["macros", "shell", "http"] }
     serde = { version = "1", features = ["derive"] }
     tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
     ```

  3. 在 workspace 根目录执行：

     ```bash
     # Windows PowerShell
     just build
     # 或
     cargo build --all
     ```

  4. 如遇到依赖冲突，优先按 workspace 顶层 `Cargo.toml` 中的版本对齐，避免在 GUI crate 中单独指定不同版本。

- **测试与验收**
  - 条件 1：`just build` 或 `cargo build --all` 成功，无新增的依赖解析错误。
  - 条件 2：进入 `agent-diva-gui` 目录：

    ```bash
    pnpm install        # 首次运行或依赖更新后执行
    pnpm tauri dev      # 或 cargo tauri dev
    ```

    GUI 能成功启动，终端无 crate 链接/解析类错误。

---

### WP-GUI-ARCH-02：GUI 与 gateway/manager 通信方式确定

- **概述**
  - 明确 GUI 与后端 gateway/manager 的通信方式：**始终通过本地 HTTP 访问 Manager API**，不在 GUI 内直接操作 channel/provider，降低耦合度。

- **先决条件**
  - `agent-diva-manager` 已实现基本的 `/health`、`/runtime` 等 HTTP 端点（按现有设计为控制面接口）。
  - gateway + manager 可以通过 CLI 方式在本地启动。

- **实施步骤**
  1. 在 `agent-diva-gui/src-tauri/src` 下新增或编辑 `commands.rs` 文件，引入健康检查结构体与 command：

     ```rust
     use serde::{Deserialize, Serialize};

     #[derive(Debug, Serialize, Deserialize)]
     pub struct HealthStatus {
       pub ok: bool,
       pub version: String,
       pub details: Option<String>,
     }

     #[tauri::command]
     pub async fn get_gateway_health(base_url: String) -> Result<HealthStatus, String> {
       let url = format!("{}/health", base_url);
       let resp = reqwest::get(&url)
         .await
         .map_err(|e| format!("request failed: {e}"))?;

       if !resp.status().is_success() {
         return Err(format!("gateway unhealthy: {}", resp.status()));
       }

       resp.json::<HealthStatus>()
         .await
         .map_err(|e| format!("invalid health payload: {e}"))
     }
     ```

  2. 在 `src-tauri/src/main.rs` 中注册该 command（示意）：

     ```rust
     fn main() {
       tauri::Builder::default()
         .invoke_handler(tauri::generate_handler![
           get_gateway_health,
           // 其他 commands ...
         ])
         .run(tauri::generate_context!())
         .expect("error while running tauri application");
     }
     ```

  3. 在前端 `src/api/gateway.ts` 中封装调用：

     ```ts
     import { invoke } from "@tauri-apps/api/core";

     export async function fetchGatewayHealth(baseUrl: string) {
       return await invoke<{
         ok: boolean;
         version: string;
         details?: string;
       }>("get_gateway_health", { baseUrl });
     }
     ```

  4. 在某个 Vue 页面（如 `Dashboard.vue`）中调用 `fetchGatewayHealth`，并将结果绑定到 UI 卡片上。

- **测试与验收**
  - 步骤 1：在终端通过 CLI 启动 gateway + manager，例如：

    ```bash
    just run -- gateway
    # 或等价的 agent-diva gateway run
    ```

  - 步骤 2：启动 GUI 开发模式，并在 Dashboard 中配置 `baseUrl`（如 `http://127.0.0.1:8080`）。
  - 期望结果：
    - 正常情况下，页面显示“运行中”状态与版本号；
    - 当手动停止 gateway/manager 后刷新页面，出现明确的“无法连接 / 网关未运行”等错误提示，而不是空白或崩溃。

---

### WP-GUI-ARCH-03：GUI 控制面信息架构

- **概述**
  - 为 GUI 设计清晰的信息架构（IA），对应后端能力，确保开发者能按页面粒度拆解任务。

- **先决条件**
  - 前端使用 Vue 3 + Vite + TailwindCSS，已初始化基础项目结构。

- **实施步骤**
  1. 在 `src/router/routes.ts` 中定义主导航路由：

     ```ts
     export const routes = [
       { path: "/", name: "dashboard", component: () => import("@/views/Dashboard.vue") },
       { path: "/config", name: "config", component: () => import("@/views/Config.vue") },
       { path: "/logs", name: "logs", component: () => import("@/views/Logs.vue") },
       { path: "/skills", name: "skills", component: () => import("@/views/Skills.vue") },
     ];
     ```

  2. 在 `src/views` 目录下创建对应的 Vue 组件文件，至少保持基础结构（示例：`Dashboard.vue`）：

     ```vue
     <template>
       <div class="p-4 space-y-4">
         <h1 class="text-xl font-semibold">Agent Diva 仪表盘</h1>
         <!-- 健康状态卡片 / 运行摘要等 -->
       </div>
     </template>

     <script setup lang="ts">
     // 后续在这里接入 fetchGatewayHealth 等 API
     </script>
     ```

  3. 在顶层布局组件（如 `App.vue`）中渲染导航和 `<router-view />`，使用 Tailwind 实现基础布局。

- **测试与验收**
  - GUI 启动后，点击导航菜单可在四个页面之间切换，无报错；
  - 每个页面在尚未接入后端逻辑前，仍然展示占位内容（空态），不会全白或闪烁；
  - 对于配置与日志页面，在后续 WP 中接入 commands 后仍复用该架构，不需要大幅改路由结构。

### CA-GUI-ARCH 服务管理 UI 设计文档

服务生命周期管理（ServiceManagementPanel）的完整 UI 设计、技术增强版 WBS 与实施规范见：

- **[ui-ca-gui-arch-service-management-panel.md](ui-ca-gui-arch-service-management-panel.md)** — 设置页 ServiceManagementPanel 的布局、交互、状态机、i18n、主题适配、验收标准及 WP-GUI-ARCH-SMP-01/02/03 实施步骤。

实现状态表中，WP-GUI-ARCH-SMP-01（前端组件）、WP-GUI-ARCH-SMP-02（Tauri 后端对接）、WP-GUI-ARCH-SMP-03（测试与验收）对应该文档第 11 章。

---

## 3. CA-GUI-CMDS：Tauri commands 与网关通信通道

### 实现状态记录

| WP | 状态 | 落仓文件 / 说明 |
|----|------|-----------------|
| WP-GUI-CMDS-00 | 已完成 | `agent-diva-gui/src/components/settings/GeneralSettings.vue`：服务管理面板按 `is_bundled` 与 `platform` 控制显示；Windows / Linux 可执行动作，macOS 为受控降级提示 |
| WP-GUI-CMDS-01 | 已完成 | `agent-diva-gui/src-tauri/src/commands.rs`：新增 `start_gateway` / `stop_gateway` / `get_gateway_process_status`；`agent-diva-gui/src/components/ConsoleView.vue` 提供 GUI 操作入口 |
| WP-GUI-CMDS-02 | 已完成 | `agent-diva-gui/src-tauri/src/commands.rs`：新增 `load_config` / `save_config`；`agent-diva-gui/src/components/ConsoleView.vue` 提供原始 JSON 配置编辑器 |
| WP-GUI-CMDS-03 | 已完成 | `agent-diva-gui/src-tauri/src/commands.rs`：新增 `tail_logs` 并解析当前日志目录；`agent-diva-gui/src/components/ConsoleView.vue` 提供刷新与级别着色 |
| WP-GUI-CMDS-04 | 已完成 | `agent-diva-gui/src/api/desktop.ts` 统一封装 runtime/service/gateway/config/logs commands；Windows 走 CLI service 子命令，Linux 走 `systemd`/`pkexec`，macOS 返回明确“待接入”提示 |

**与其它 CA 的接口点：**

- **CA-HL-WIN-SERVICE**：Windows 服务管理继续通过 `agent-diva.exe service *` 子命令桥接，GUI 只做状态展示与按钮触发。
- **CA-HL-LNX-SYSTEMD**：Linux GUI bundle 通过 `scripts/ci/prepare_gui_bundle.py` 携带 `contrib/systemd/*` 到 Tauri `resources/systemd/`，供 `install_service` / `uninstall_service` 调用。
- **CA-HL-MAC-LAUNCHD**：当前仅暴露运行时状态与受控降级文案，待 launchd 模板和安装脚本落地后再打开实际安装动作。

### WP-GUI-CMDS-00：服务管理板块（Service Management Panel）界面与交互（新增）

- **概述**
  - 当你（Agent）需要在 GUI 中提供本机服务管理能力时，应在“设置/通用设置”下新增一个 `ServiceManagementPanel` 区域，用于展示和操作当前平台的网关服务。
  - 该面板仅在打包后的独立应用中启用，在开发模式（`tauri dev`）下默认隐藏或以只读灰显形式存在。

- **先决条件**
  - 已在 Tauri 后端实现 `get_runtime_info` command，返回：
    - `platform`: `\"windows\" | \"linux\" | \"macos\"`；
    - `is_bundled`: `bool`。

- **实施步骤**
  1. 在前端路由或设置页组件中，为“通用设置”添加服务管理子区块：

     ```ts
     // 示例：Settings 页面中增加一个 ServiceManagementPanel 区域挂载点
     // 伪代码，仅作为结构参考
     const isBundledApp = ref(false);
     const platform = ref<"windows" | "linux" | "macos" | null>(null);

     onMounted(async () => {
       const info = await getRuntimeInfo(); // 由 Tauri command 提供
       isBundledApp.value = info.isBundled;
       platform.value = info.platform;
     });
     ```

  2. 设计服务管理面板的 UI 结构（以 Vue 模板形式描述），你应创建一个类似下面结构的组件（伪代码）：

     ```vue
     <template>
       <section v-if="isBundledApp" class="space-y-3">
         <h2 class="text-lg font-semibold">服务管理</h2>
         <p class="text-sm text-muted">
           管理本机 Agent Diva 网关服务（仅适用于当前操作系统）。
         </p>

         <div v-if="platform === 'windows'">
           <!-- Windows 服务行：状态 + 安装/卸载按钮 -->
         </div>
         <div v-else-if="platform === 'linux'">
           <!-- systemd 服务行 -->
         </div>
         <div v-else-if="platform === 'macos'">
           <!-- launchd 服务行 -->
         </div>
       </section>

       <section v-else class="space-y-2">
         <h2 class="text-lg font-semibold text-muted">服务管理（仅打包应用可用）</h2>
         <p class="text-xs text-muted">
           当前运行在开发模式，服务管理功能已禁用。
         </p>
       </section>
     </template>
     ```

  3. 为各平台定义统一的交互按钮和文案（推荐）：
     - **Windows：**
       - 状态文本：`已安装/正在运行`、`已安装/未运行`、`未安装`；
       - 操作按钮：`安装服务`、`卸载服务`（可选：`启动服务`、`停止服务`）；
     - **Linux（systemd）：**
       - 状态文本：`unit 已启用/active`、`unit 已启用/inactive`、`未安装（无 unit）`；
       - 操作按钮：`安装 systemd 服务`、`卸载 systemd 服务`；
     - **macOS（launchd）：**
       - 状态文本：`Plist 已存在/已加载`、`Plist 已存在/未加载`、`未安装`；
       - 操作按钮：`安装 launchd 服务`、`卸载 launchd 服务`。

  4. 当用户点击按钮时，由你（Agent）调用对应的 Tauri commands（见后续 WP 中的 `get_service_status/install_service/uninstall_service`），并在返回结果后更新状态与错误提示。

- **测试与验收**
  - 在打包应用中打开设置页时：
    - `isBundledApp === true` 且 `platform` 合法时，“服务管理”板块可见；
    - 三个平台上展示的文案和按钮符合上述定义；
  - 在开发模式下：
    - `isBundledApp === false`，服务管理板块隐藏或仅显示“仅打包应用可用”的灰显文案；
    - 即使通过前端调试强行调用，也不会在后端执行安装/卸载逻辑（后端返回“仅在打包模式可用”的错误）。 

---

### WP-GUI-CMDS-01：启动 / 停止本地 gateway 子进程

- **概述**
  - 为桌面模式提供“一键启动 / 停止”本地 gateway 子进程的能力，方便普通用户无需了解 CLI。

- **先决条件**
  - 本机已存在可执行的 `agent-diva` 二进制，并且通过 `agent-diva gateway run` 可正常启动。

- **实施步骤**
  1. 在 `commands.rs` 中添加进程句柄和 commands（示意）：

     ```rust
     use std::{path::PathBuf, sync::Mutex};
     use tokio::process::Command;

     struct GatewayHandle {
       child: tokio::process::Child,
     }

     lazy_static::lazy_static! {
       static ref GATEWAY_HANDLE: Mutex<Option<GatewayHandle>> = Mutex::new(None);
     }

     #[tauri::command]
     pub async fn start_gateway(bin_path: Option<String>) -> Result<(), String> {
       let exe = bin_path
         .map(PathBuf::from)
         .unwrap_or_else(|| PathBuf::from("agent-diva"));

       let child = Command::new(exe)
         .arg("gateway")
         .arg("run")
         .spawn()
         .map_err(|e| format!("failed to spawn gateway: {e}"))?;

       let mut handle = GATEWAY_HANDLE.lock().unwrap();
       *handle = Some(GatewayHandle { child });
       Ok(())
     }

     #[tauri::command]
     pub async fn stop_gateway() -> Result<(), String> {
       let mut handle = GATEWAY_HANDLE.lock().unwrap();
       if let Some(gw) = handle.as_mut() {
         gw.child.kill().await.map_err(|e| format!("kill failed: {e}"))?;
         *handle = None;
       }
       Ok(())
     }
     ```

  2. 在前端 API 层封装调用，并在 GUI 中控台页面放置“启动 / 停止”按钮，绑定到上述 commands；建议同时暴露 `get_gateway_process_status`，把 PID、可执行路径与健康检查结果展示给用户。

- **测试与验收**
  - 正常路径：
    - 点击“启动网关”后，系统进程列表中出现 `agent-diva` / `agent-diva.exe`；
    - 点击“停止网关”后，该进程消失，且不再占用监听端口。
  - 异常路径：
    - 当 `agent-diva` 不存在或无法执行时，GUI 显示明确错误提示，说明可能的原因与解决方式，而不是静默失败。

---

### WP-GUI-CMDS-02：配置读取与更新

- **概述**
  - 提供“可视化配置编辑器”，让用户无需手动打开 JSON 文件即可调整 Agent Diva 行为。

- **先决条件**
  - `agent-diva-core` 已提供 `config::load_default` / `config::save_default` 等工具函数。

- **实施步骤**
  1. 在 `commands.rs` 中添加配置相关 commands：

     ```rust
     #[tauri::command]
     pub fn load_config() -> Result<String, String> {
       let config = agent_diva_core::config::load_default()
         .map_err(|e| format!("load config failed: {e}"))?;
       serde_json::to_string_pretty(&config)
         .map_err(|e| format!("serialize config failed: {e}"))
     }

     #[tauri::command]
     pub fn save_config(raw: String) -> Result<(), String> {
       let cfg: agent_diva_core::config::Config =
         serde_json::from_str(&raw).map_err(|e| format!("parse config failed: {e}"))?;
       agent_diva_core::config::save_default(&cfg)
         .map_err(|e| format!("save config failed: {e}"))?;
       Ok(())
     }
     ```

  2. 在前端配置视图（如 `ConsoleView.vue` 或等价页面）中：
    - 首次进入时调用 `load_config`，将返回的 JSON 字符串填入编辑器（如 `monaco-editor` 或简单 `<textarea>`）。
    - 用户点击“保存”按钮时，将当前文本通过 `save_config` 发送给后端：
       - 若后端返回错误，将错误信息展示给用户并保持原内容；
       - 若成功，提示“保存成功”，并可选触发“重载配置”（调用 Manager API 的 `/ops/reload`）。

- **测试与验收**
  - 正向场景：
    - 修改某个简单配置项（如日志级别）并保存；
    - 重启 gateway 或触发热重载后，确认行为变化（如日志输出级别）。
  - 错误场景：
    - 故意将 JSON 编辑成不合法格式，点击保存；
    - 预期 GUI 显示“解析配置失败”类错误，并高亮问题内容，配置文件不会被破坏。

---

### WP-GUI-CMDS-03：日志查看与过滤

- **概述**
  - 在 GUI 中提供“实时查看网关日志”的能力，支持简单 tail 与级别筛选，作为日常运维最小闭环。

- **先决条件**
  - 日志目录与文件命名在后端已统一（如 `~/.agent-diva/logs/gateway.log`）。

- **实施步骤**
  1. 在 `commands.rs` 中添加日志 tail command：

     ```rust
     #[tauri::command]
     pub fn tail_logs(lines: usize) -> Result<Vec<String>, String> {
       let path = agent_diva_core::paths::logs_dir().join("gateway.log");
       let content = std::fs::read_to_string(&path)
         .map_err(|e| format!("read log failed: {e}"))?;
       let all_lines: Vec<_> = content.lines().map(|s| s.to_owned()).collect();
       let len = all_lines.len();
       Ok(all_lines.into_iter().skip(len.saturating_sub(lines)).collect())
     }
     ```

  2. 在前端日志视图（如 `ConsoleView.vue` 或等价页面）中：
     - 提供一个“刷新”按钮，每次点击调用 `tail_logs(lines)`（如 200 行）并渲染结果；
     - 可选提供下拉框选择显示最近 100 / 200 / 500 行；
     - 简单通过字符串匹配标记日志级别（INFO / WARN / ERROR）并用不同颜色展示。

- **测试与验收**
  - 启动 gateway 并触发一些操作（例如向绑定的聊天平台发送测试消息）；
  - 打开日志页面多次刷新，确认新日志能被正确拉取且顺序正确；
  - 日志文件不存在或为空时，页面显示“暂无日志 / 日志文件未生成”，而不是 panic 或卡死。

---

### WP-GUI-CMDS-04：运行时信息与服务状态命令（get_runtime_info / get_service_status / install_service / uninstall_service）

- **概述**
  - 为了让你（Agent）能够从 GUI 控制服务管理行为，需要在 Tauri 后端实现一组稳定的 commands，向前端暴露当前运行模式、平台信息以及服务的安装/运行状态，并允许触发安装/卸载动作。

- **先决条件**
  - Headless WBS 中已定义各平台服务行为（Windows Service / systemd / launchd）与 CLI 子命令/脚本；
  - 分发 WBS 中已约定 CLI/Service 二进制或脚本在打包应用中的路径。

- **实施步骤**
  1. 在 `commands.rs` 中实现 `get_runtime_info`，返回平台与打包状态（示意结构）：  

     ```rust
     #[derive(serde::Serialize)]
     pub struct RuntimeInfo {
       pub platform: String,   // "windows" | "linux" | "macos"
       pub is_bundled: bool,   // true 表示打包应用，false 表示 dev
     }

     #[tauri::command]
     pub fn get_runtime_info() -> RuntimeInfo {
       let platform = if cfg!(target_os = "windows") {
         "windows"
       } else if cfg!(target_os = "linux") {
         "linux"
       } else {
         "macos"
       };

       let is_bundled = !cfg!(debug_assertions); // 示例：根据构建模式粗略区分，实际可按项目需求调整

       RuntimeInfo {
         platform: platform.to_string(),
         is_bundled,
       }
     }
     ```

  2. 在 `commands.rs` 中实现 `get_service_status` / `install_service` / `uninstall_service` 的统一入口（仅示意接口形状，具体实现由 Headless/分发 WBS 细化）：  

     ```rust
     #[derive(serde::Serialize)]
     pub struct ServiceStatus {
       pub installed: bool,
       pub running: bool,
       pub details: Option<String>,
     }

     #[tauri::command]
     pub async fn get_service_status() -> Result<ServiceStatus, String> {
       let info = get_runtime_info();
       if !info.is_bundled {
         return Err("service management is only available in bundled app".into());
       }
       match info.platform.as_str() {
         "windows" => {
           // 调用 Windows CLI 子命令或 API 查询状态
         }
         "linux" => {
           // 调用 systemd 查询（如 systemctl is-active/is-enabled）
         }
         "macos" => {
           // 调用 launchctl / 检查 Plist
         }
         _ => Err("unsupported platform".into()),
       }
     }

     #[tauri::command]
     pub async fn install_service() -> Result<(), String> {
       let info = get_runtime_info();
       if !info.is_bundled {
         return Err("service install is only available in bundled app".into());
       }
       // 根据平台调用 Headless WBS 约定的安装入口
       Ok(())
     }

     #[tauri::command]
     pub async fn uninstall_service() -> Result<(), String> {
       let info = get_runtime_info();
       if !info.is_bundled {
         return Err("service uninstall is only available in bundled app".into());
       }
       // 根据平台调用 Headless WBS 约定的卸载入口
       Ok(())
     }
     ```

  3. **Tauri commands ↔ CLI / 脚本 ↔ Service 行为映射**（当前以 Windows / Linux 为主，macOS 受控降级）：

     | 平台 | Tauri command | CLI / 脚本入口 | Service 行为 |
     |------|---------------|----------------|--------------|
     | Windows | `get_service_status` | `agent-diva.exe service status --json` | 查询 AgentDivaGateway 安装/运行状态 |
     | Windows | `install_service` | `agent-diva.exe service install --auto-start` | 安装并配置自启动 |
     | Windows | `uninstall_service` | `agent-diva.exe service uninstall` | 停止并删除服务 |
     | Windows | `start_service` | `agent-diva.exe service start` | 启动已安装服务 |
     | Windows | `stop_service` | `agent-diva.exe service stop` | 停止运行中服务 |
     | Linux | `get_service_status` | `systemctl show agent-diva ...` | 查询 unit 是否存在及 active 状态 |
     | Linux | `install_service` / `uninstall_service` | `pkexec bash resources/systemd/install.sh` / `uninstall.sh` | 安装或卸载 systemd 服务 |
     | Linux | `start_service` / `stop_service` | `pkexec systemctl start|stop agent-diva` | 启停已安装 unit |
     | macOS | `get_service_status` | `launchctl list` + `~/Library/LaunchAgents/com.agent-diva.gateway.plist` | 查询 launchd 模板/加载状态 |
     | macOS | `install_service` / `uninstall_service` / `start_service` / `stop_service` | 当前返回“待接入” | 受控降级，不做静默失败 |

     GUI 通过 `resolve_cli_binary(app)` 或打包资源路径定位内嵌二进制/脚本，并统一要求 `is_bundled == true` 才允许执行实际服务操作。

  4. 在 `main.rs` 中将上述 commands 注册到 `invoke_handler` 中，确保前端可以通过 `invoke` 调用：

     ```rust
     fn main() {
       tauri::Builder::default()
         .invoke_handler(tauri::generate_handler![
           get_runtime_info,
           get_service_status,
           install_service,
           uninstall_service,
           // 其他 commands ...
         ])
         .run(tauri::generate_context!())
         .expect("error while running tauri application");
     }
     ```

  4. 在前端 API 层封装上述 commands，供服务管理面板统一使用：

     ```ts
     import { invoke } from "@tauri-apps/api/core";

     export async function getRuntimeInfo() {
       return await invoke<{ platform: string; isBundled: boolean }>("get_runtime_info");
     }

     export async function getServiceStatus() {
       return await invoke<{ installed: boolean; running: boolean; details?: string }>(
         "get_service_status"
       );
     }

     export async function installService() {
       return await invoke<void>("install_service");
     }

     export async function uninstallService() {
       return await invoke<void>("uninstall_service");
     }
     ```

  5. **Tauri commands 与 CLI 子命令映射（CA-HL-WIN-SERVICE 已落地）**

     | Tauri command | CLI 子命令 | Service 行为 |
     |---------------|------------|--------------|
     | `get_service_status` | `agent-diva.exe service status --json` | 查询服务是否已安装、是否运行，返回 JSON |
     | `install_service` | `agent-diva.exe service install --auto-start` | 安装并配置自启动 |
     | `uninstall_service` | `agent-diva.exe service uninstall` | 停止并删除服务 |
     | `start_service` | `agent-diva.exe service start` | 启动已安装的服务 |
     | `stop_service` | `agent-diva.exe service stop` | 停止运行中的服务 |

     实现位置：`agent-diva-gui/src-tauri/src/commands.rs` 中的 `run_service_cli` 统一调用 CLI；仅在 `is_bundled == true` 且 `platform == "windows"` 时执行，否则返回明确错误。

- **测试与验收**
  - 在 dev 模式下调用上述 commands 时：  
    - `get_runtime_info.is_bundled` 为 `false`；  
    - `get_service_status/ install_service/ uninstall_service` 返回明确错误，且不会更改系统服务状态；  
  - 在打包应用中：  
    - `get_runtime_info.is_bundled` 为 `true` 且 `platform` 与实际 OS 匹配；  
    - 三个平台上均可通过 `get_service_status` 看到正确的“未安装/已安装/运行中”等状态，并可通过 GUI 按钮触发安装/卸载动作。 


## 4. CA-GUI-BUNDLE：多平台打包与安装包产物

### WP-GUI-BUNDLE-00：构建环境与现有 CI 能力对齐

- **概述**
  - 在真正执行 GUI 打包前，先确认本地环境、现有锁文件状态与 CI 产物契约一致，避免把环境问题误判为 Tauri bundler 问题。

- **先决条件**
  - 已存在 `agent-diva-gui` 工程目录；
  - 当前仓库已具备 `just fmt-check`、`just check`、`just test` 这些 workspace 级质量门；
  - 已有 `CA-CI-MATRIX` 文档定义 GUI artifact 目录与命名契约。

- **实施步骤**
  1. 在仓库根目录执行基础校验：

     ```bash
     just fmt-check
     just check
     just test
     ```

     - 若失败，先区分失败来源是否来自本次 GUI 打包工作。
     - 对于当前仓库已存在的历史失败项，应在 iteration log 的 `verification.md` 中记录，不要为了打包工作去修改无关核心代码。

  2. 在 `agent-diva-gui` 目录检查 Node 依赖状态：

     ```bash
     pnpm import
     pnpm install --frozen-lockfile --registry=https://registry.npmjs.org
     ```

     - 当 `package-lock.json` 与 `pnpm-lock.yaml` 不一致时，优先使用 `pnpm import` 从现有 `package-lock.json` 同步一次 pnpm 锁文件。
     - 若团队后续统一只保留 pnpm，可再单独清理 `package-lock.json`，但不属于本 WP 的强制动作。

  3. 在 `agent-diva-gui` 目录执行一次开发模式启动：

     ```bash
     pnpm tauri dev
     ```

     - 观察是否能拉起 Vite 与 Tauri 窗口；
     - 开发模式主要用于确认前端构建与 Tauri host 没有明显断裂，不替代正式 bundling。

- **测试与验收**
  - `just check` 通过；
  - `pnpm install --frozen-lockfile` 可以在当前依赖状态下完成；
  - `pnpm tauri dev` 至少在当前开发平台可启动主窗口；
  - 若 `fmt-check` / `test` 失败，必须在日志中明确标注是否为仓库既有问题。

---

### WP-GUI-BUNDLE-01：Tauri 多平台打包配置

- **概述**
  - 统一配置 Tauri bundler 的多平台打包目标，产出 Windows / macOS / Linux 安装包。

- **先决条件**
  - 本地已安装 Tauri 2 所需的系统依赖（详见官方文档）。

- **实施步骤**
  1. 编辑 `agent-diva-gui/src-tauri/tauri.conf.json`（或 `.toml`），在 `bundle` 段配置目标、资源与 Windows 安装器 hook：

     ```json
     {
       "productName": "Agent Diva",
       "identifier": "com.agentdiva.desktop",
       "bundle": {
         "active": true,
         "targets": ["nsis", "msi", "app", "dmg", "deb", "appimage"],
         "icon": [
           "icons/icon.png",
           "icons/32x32.png",
           "icons/128x128.png",
           "icons/128x128@2x.png",
           "icons/icon.icns",
           "icons/icon.ico"
         ],
         "resources": ["resources/"],
         "windows": {
           "webviewInstallMode": {
             "type": "downloadBootstrapper"
           },
           "nsis": {
             "installMode": "both",
             "displayLanguageSelector": true,
             "installerHooks": "./windows/hooks.nsh"
           }
         }
       }
     }
     ```

  2. 在 GUI 根目录增加一个显式的资源准备步骤，把 CLI/Service 二进制 staged 到 Tauri `resources/`：

     ```bash
     pnpm bundle:prepare
     ```

     - 对应脚本：`scripts/ci/prepare_gui_bundle.py`
     - 目标目录：`agent-diva-gui/src-tauri/resources/`
     - 预期产物：
       - `resources/bin/<target-os>/agent-diva(.exe)`
       - `resources/bin/<target-os>/agent-diva-service(.exe)`（可选）
       - `resources/manifests/gui-bundle-manifest.json`

  3. 在不同平台上执行构建命令：

     ```bash
     # 先准备 resources
     pnpm bundle:prepare

     # Windows
     pnpm tauri build --target x86_64-pc-windows-msvc

     # macOS（通用二进制示例）
     pnpm tauri build --target universal-apple-darwin

     # Linux
     pnpm tauri build --target x86_64-unknown-linux-gnu
     ```

- **测试与验收**
  - `bundle` 目标与安装器 hook 全部可从 `tauri.conf.json` 直接解析，不依赖额外手工步骤；
  - `bundle:prepare` 执行后，`resources/` 中存在跨平台打包所需的 staged 文件；
  - 在各自平台上完成一次完整安装周期：
    - 安装：运行安装包，确认应用出现在系统应用列表 / 开始菜单 / Launchpad；
    - 启动：从系统入口启动 GUI，确认主界面能正常加载；
    - 卸载：通过系统卸载机制或安装器提供的卸载入口移除应用，验证：
      - 应用二进制与快捷方式已删除；
      - 用户数据目录是否按预期保留（配置 / 会话 / 日志）。

---

### WP-GUI-BUNDLE-02：跨平台品牌与图标统一

- **概述**
  - 为 GUI 应用设置一致的名称、图标和版本号，提升品牌一致性。

- **先决条件**
  - 已有设计好的图标资源（建议至少 256×256 分辨率）。

- **实施步骤**
  1. 在 `agent-diva-gui/src-tauri/icons/` 下维护单一 SVG 源文件，例如：

     ```bash
     agent-diva-gui/src-tauri/icons/icon-source.svg
     ```

  2. 使用 Tauri CLI 从 SVG 生成多格式图标：

     ```bash
     pnpm tauri icon src-tauri/icons/icon-source.svg src-tauri/icons
     ```

     - 生成后至少应存在：
       - `icon.ico`（Windows）
       - `icon.icns`（macOS）
       - `icon.png`（Linux / 通用）
       - `32x32.png`
       - `128x128.png`
       - `128x128@2x.png`

  3. 在 `tauri.conf.json` 中统一元数据：
     - `"productName": "Agent Diva"`
     - `"identifier": "com.agentdiva.desktop"`
     - `app.windows[*].title` 统一为 `Agent Diva`

  4. 统一版本号来源：
     - 在 `agent-diva-gui/src-tauri/Cargo.toml` 中维护 `[package] version = "x.y.z"`；
     - 从 `tauri.conf.json` 中移除硬编码 `version` 字段，让打包版本默认跟随 Cargo。

- **测试与验收**
  - 在三平台上安装后，检查：
    - 任务栏 / Dock / 应用菜单中的图标是否统一；
    - 打开的窗口标题栏是否显示统一名称（如 “Agent Diva”）；
    - 系统应用信息中显示的版本号与 `Cargo.toml` 一致。

---

### WP-GUI-BUNDLE-03：与 CI / 分发文档的结构对齐

- **概述**
  - 将本地 GUI 打包结果与 `CA-CI-MATRIX`、`CA-CI-ARTIFACTS`、`CA-DIST-GUI-INSTALLER` 的输入输出契约对齐，避免后续在 CI 或安装器层重复修目录结构。

- **先决条件**
  - 已按 `WP-GUI-BUNDLE-01` 产出至少一个平台的 bundle；
  - CI WBS 与分发 WBS 已存在并可作为对照文档。

- **实施步骤**
  1. 对照 `docs/app-building/wbs-ci-cd-and-automation.md` 中 GUI artifact 的上传路径：

     ```yaml
     path: agent-diva-gui/src-tauri/target/release/bundle/**
     ```

  2. 确认本地 `pnpm tauri build` 结果落在同一路径下，不额外引入私有中间目录。
  3. 对照 `docs/app-building/wbs-distribution-and-installers.md` 中 `CA-DIST-GUI-INSTALLER` 的目标，确认以下约定稳定：
     - 安装器来自 Tauri bundle 原生产物；
     - `resources/bin/<target-os>/` 为安装器运行时内嵌资源根；
     - Windows 可复用 `src-tauri/windows/hooks.nsh` 中的可选服务安装逻辑。
  4. 若目录结构与文档约定不一致，优先调整 GUI bundling 文档或 `tauri.conf.json`，不要先改 CI。

- **测试与验收**
  - 产物路径与 CI 文档中的 artifact 上传路径一致；
  - 分发文档可直接引用当前 GUI bundle 目录结构，而无需新增“特例说明”；
  - Windows 安装器 hook 可以定位到 `resources/bin/windows/agent-diva.exe`。

---

### WP-GUI-BUNDLE-04：与桌面 QA smoke 的映射

- **概述**
  - 将 GUI 打包交付与桌面 smoke 验收路径绑定，确保每次 bundle 行为变更都能回落到固定的手工/自动化验证脚本。

- **先决条件**
  - 至少完成一个平台的 GUI 构建；
  - `docs/app-building/wbs-validation-and-qa.md` 已定义桌面 smoke 测试矩阵。

- **实施步骤**
  1. 按平台映射 GUI smoke WP：
     - Windows：`WP-QA-DESKTOP-01`
     - macOS：`WP-QA-DESKTOP-02`
     - Linux：`WP-QA-DESKTOP-03`
  2. 每次完成 `pnpm tauri build` 后，至少在当前平台执行一次最小 smoke：
     - 安装；
     - 启动主窗口；
     - 检查主界面是否可加载；
     - 卸载并确认用户数据目录按设计保留。
  3. 将执行结果写入当前迭代的 `docs/logs/.../verification.md`，标明：
     - 实际执行的平台；
     - 构建命令；
     - smoke 观察点；
     - 失败是否属于环境限制或仓库既有问题。

- **测试与验收**
  - 至少一条 GUI smoke 路径被真实执行并留下记录；
  - 任一平台若因系统依赖或签名限制无法完整验证，日志中必须写明阻塞点；
  - 后续 PR 或交付说明可直接引用对应 QA WP 编号。

---

## 5. GUI 相关测试与自动化集成建议

- **本地开发者日常检查**
  - `just fmt-check && just check && just test`（整个 workspace）；
  - `cd agent-diva-gui && pnpm install --frozen-lockfile --registry=https://registry.npmjs.org`；
  - `cd agent-diva-gui && pnpm tauri dev`，验证 GUI 主流程可用；
  - `cd agent-diva-gui && pnpm bundle:prepare && pnpm tauri build --target x86_64-pc-windows-msvc`（当前平台示例）。

- **CI 最小集成建议**
  - 在 GitHub Actions / 其他 CI 中为 GUI 增加以下 job（示例思路，具体 YAML 见 CI 文档）： 
    - 安装 Node.js 与 pnpm；
    - 先执行 `pnpm bundle:prepare`，保证安装器内嵌资源与本地流程一致；
    - 构建前端资源（`pnpm install --frozen-lockfile && pnpm build`）；
    - 在至少一个平台构建 GUI 安装包（如 `windows-latest` 上运行 `pnpm tauri build`），并将产物上传为 CI 工件。

