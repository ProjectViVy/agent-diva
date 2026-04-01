---
module: agent-diva-cli
kind: rust-binary-library
msrv: "1.80.0"
workspace: agent-diva
---

## 模块职责

- 可执行文件 `agent-diva`：面向用户的命令行入口（onboard、gateway、agent、chat、provider、doctor、Windows 服务相关子命令等）。
- 库 crate `agent_diva_cli`：供本仓库其他组件复用的 CLI 运行时（`cli_runtime`、`chat_commands`、`provider_commands`、`client`）。

## 依赖与边界

- **内部**：依赖 `agent-diva-core`、`agent-diva-agent`、`agent-diva-providers`、`agent-diva-channels`、`agent-diva-tools`、`agent-diva-manager`（本地网关 `run_local_gateway` 等）。
- **Workspace**：`tokio`、`serde`、`anyhow`、`tracing`、`clap`、`dirs`、`chrono`、`reqwest`、`futures` 等均应通过 `workspace = true` 引用；额外直接依赖含 `ratatui`、`crossterm`、`eventsource-stream`。
- **平台**：`windows-service` 仅在 `cfg(windows)` 下启用。
- **边界**：业务域逻辑优先放在 core/agent/channels；CLI 负责参数解析、终端 UI、编排与远程 API 客户端；不要在本 crate 重复实现 manager 的 HTTP API 服务端。

## 关键入口

- 二进制：`src/main.rs`（`[[bin]] name = "agent-diva"`）。
- 库根：`src/lib.rs`（导出 `chat_commands`、`cli_runtime`、`client`、`provider_commands`）。

## 实现约定

- **MSRV**：`Cargo.toml` 声明 `rust-version = "1.80.0"`，与 workspace `[workspace.package]` 一致。
- **版本号**：crate `version` 宜与 workspace 成员约定对齐（当前 workspace 根为 `0.4.1`，本包可能仍为 `0.4.0` 时需知悉漂移）。
- 使用 `clap` derive；异步入口遵循 `tokio` 与现有子命令模式；TUI 路径使用 `ratatui` + `crossterm`。
- Debian 打包元数据在 `[package.metadata.deb]`，变更发布物时需同步检查。

## 测试与检查

- `cargo test -p agent-diva-cli`；`main.rs` 末尾含 `#[cfg(test)]` 模块。
- 集成远程模式时留意 `mockito` 等 dev-dependencies 的用法。

## 切勿遗漏

- 修改子命令或全局 flag（如 `config` / `config_dir` / `remote` / `api_url`）时，同步 GUI/Tauri 与文档中对 CLI 行为的描述。
- Windows 服务相关逻辑与 `agent-diva-service` 可执行文件部署路径（同目录 `agent-diva.exe`）需保持一致。
