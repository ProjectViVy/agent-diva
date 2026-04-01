---
module: agent-diva-gui
kind: vue-tauri-desktop
msrv_rust: "1.80.0"
workspace: agent-diva
frontend_package: agent-diva-gui
---

## 模块职责

- **桌面应用**：Tauri 2 + Vue 3，提供网关控制、聊天、设置（提供商/频道/MCP/技能等）、向导与国际化。
- **前后端分工**：前端负责 UI 与调用 Tauri command / HTTP API；Rust 侧负责进程管理、与 `agent-diva-cli`/`agent-diva-core` 等集成、系统能力（如 opener）。

## 依赖与边界

### 前端（仓库内 `agent-diva-gui/`）

- **运行时**：`vue`、`vue-i18n`、`@tauri-apps/api`、`@tauri-apps/plugin-opener`、`markdown-it`、`highlight.js`、`lucide-vue-next`。
- **构建**：Vite 6、`@vitejs/plugin-vue`、`vue-tsc`、`typescript` ~5.6、Tailwind 3、PostCSS、Autoprefixer。
- **脚本**：`npm run dev`（Vite）、`build`（类型检查 + `vite build`）、`tauri`（CLI）、`bundle:prepare`（调用 `scripts/ci/prepare_gui_bundle.py`）。
- **边界**：默认开发服务器端口 **1420**（`vite.config.ts`，`strictPort: true`）；通过 `src/api/desktop.ts` 等与后端通信，避免在 Vue 中硬编码 Rust 类型。

### Tauri（`agent-diva-gui/src-tauri/`）

- **Workspace 成员**：`Cargo.toml` 在 workspace 中，可使用 `tokio`、`tracing`、`tracing-subscriber`、`serde`、`dirs`、`once_cell`、`which`、`reqwest`、`futures`、`eventsource-stream` 等（部分版本在 manifest 内显式写 `1`/`0.11` 与 workspace 对齐时注意一致性）。
- **内部 path**：`agent-diva-core`、`agent-diva-cli`、`agent-diva-neuron`、`agent-diva-providers`。
- **边界**：注释已说明 Manager 能力以 **HTTP** 消费；`agent-diva-cli` 仅用于本地运行时/状态类辅助。发布构建在非 debug 下会管理网关生命周期（见 `lib.rs` `should_manage_gateway_lifecycle`）。

## 关键入口

### 前端

- `src/main.ts`：`createApp(App).use(i18n).mount("#app")`。
- `src/App.vue`：根布局；功能视图在 `src/components/`（如 `ChatView`、`GatewayControlPanel`、`SettingsView`）。
- `index.html` + Vite 入口由脚手架约定（与 `vite.config.ts` 一致）。

### Tauri

- `src-tauri/src/main.rs`：仅调用 `agent_diva_gui_lib::run()`；release 下 `windows_subsystem = "windows"`。
- `src-tauri/src/lib.rs`：`run()` 注册插件、`AgentState`、启动屏逻辑、网关自动启动与孤儿进程清理（release）。
- `src-tauri/src/commands.rs`、`app_state.rs`、`process_utils.rs`：command 与进程工具。

## 实现约定

- **Rust MSRV**：工程整体以 workspace `rust-version = "1.80.0"` 为准；`src-tauri` 包未单独写 `rust-version` 时仍按 1.80 工具链构建。
- **版本**：npm 与 Tauri `package` 版本当前均为 `0.4.1`，与 workspace 根 `0.4.1` 一致；若只升一侧需同步发布说明。
- **依赖一致性**：Tauri 中 `reqwest`/`serde` 若与 workspace 重复声明，优先统一到 workspace 依赖以免双版本。
- 前端使用 ES modules（`"type": "module"`）；Tauri API 使用 v2 插件模型。

## 测试与检查

- 前端：`npm run build`（含 `vue-tsc --noEmit`）。
- Rust：`cargo test -p agent-diva-gui`（如 `src-tauri/tests/gateway_process_management_bugfix.rs`）。
- 开发：`npm run tauri dev`；debug 模式下网关生命周期由外部/手动预期，勿依赖自动拉起行为排查问题。

## 切勿遗漏

- 修改 Tauri command 签名必须同步前端 `src/api/*.ts` 调用处。
- `bundle:prepare` 与 CI 脚本路径相对于 GUI 根目录；移动目录结构时更新脚本与文档。
- 与 CLI/manager 的端口、API 路径约定变更时，同时改前端 API 层与 `commands.rs`。
