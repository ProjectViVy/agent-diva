---
module: agent-diva-manager
kind: rust-library
msrv: "1.80.0"
workspace: agent-diva
---

## 模块职责

- **本地网关运行时**：`run_local_gateway` + `GatewayRuntimeConfig`（默认 HTTP 端口 `DEFAULT_GATEWAY_PORT = 3000`），集成 `AgentLoop`、频道、Cron、`MessageBus` 等。
- **HTTP API 服务**：Axum `run_server`，REST 路由覆盖 chat、events、config、providers、channels、cron、MCP、skills、tools 等（见 `server.rs` / `handlers.rs`）。
- **状态与编排**：`AppState`、`Manager`、MCP/Skill 服务、运行时任务与优雅关闭（`runtime/*`）。

## 依赖与边界

- **内部**：`agent-diva-core`、`agent-diva-agent`、`agent-diva-providers`、`agent-diva-channels`、`agent-diva-tools`。
- **HTTP 栈**：`axum`、`tower-http`（cors、trace、fs）、`tokio`（features 含 `full`）、`tokio-stream`、`futures`。
- **其他**：`zip`、`chrono`、`serde`/`serde_json`、`tracing`、`anyhow`、`dirs`、`clap`（derive）。
- **边界**：本包为 **库**，无独立 `main`；可执行入口在 `agent-diva` CLI 中调用 `run_local_gateway` 等。GUI 通过 HTTP 调 manager API，避免在 manager 内直接依赖 GUI/Tauri。

## 关键入口

- 库根：`src/lib.rs` — 导出 `Manager`、`run_local_gateway`、`GatewayRuntimeConfig`、`DEFAULT_GATEWAY_PORT`、`run_server`、`AppState` 等。
- 服务构建：`server.rs` 的 `run_server` / `build_app`。
- 运行时：`runtime.rs` 及子模块 `bootstrap`、`shutdown`、`task_runtime`。

## 实现约定

- **MSRV**：`rust-version = "1.80.0"`，与 workspace 一致。
- 新增 API 时在 `handlers` 与 `server` 路由同时注册，并保持 CORS/Trace 层行为一致。
- 版本号与 path 依赖的兄弟 crate 保持 `0.4.x` 约定。

## 测试与检查

- `cargo test -p agent-diva-manager`；`server.rs`、`manager.rs`、`mcp_service.rs`、`skill_service.rs`、`runtime/task_runtime.rs` 等含 `#[cfg(test)]`。
- dev-dependencies：`tempfile`、`tower`（util）。

## 切勿遗漏

- 变更端口常量或 shutdown 广播语义时，同步 CLI 远程默认 URL 与 GUI 配置。
- 长任务与流式响应须与 `stop_chat`、events SSE 等行为一致，避免僵尸任务。
