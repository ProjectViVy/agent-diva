---
project_name: agent-diva-core
date: '2026-03-30'
module: agent-diva/agent-diva-core
status: complete
parent_workspace: agent-diva
optimized_for_llm: true
---

# agent-diva-core —  crate 上下文

面向 LLM 的精简说明；全仓库规则见 `agent-diva/project-context.md`。

## 模块职责

共享内核：类型、trait、工具与横切能力，供 agent / channels / tools / service 等 crate 依赖。

| 子模块 | 作用 |
|--------|------|
| `error` / `error_context` | 统一 `Error`/`Result`（`thiserror`）；错误现场截取与调试上下文（长度上限、元数据） |
| `config` | `loader` / `schema` / `validate`：配置加载、结构与校验 |
| `logging` | 基于 `tracing-subscriber`、`tracing-appender` 的初始化（EnvFilter、JSON/文本、按日滚动文件） |
| `bus` | 入站/出站消息与 `MessageBus` 双队列，解耦 channel 与 agent |
| `session` / `memory` | 会话与记忆存储、管理 |
| `cron` | 定时类型与服务 |
| `heartbeat` | 心跳类型与服务 |
| `soul` | 与「人格/内核」相关的共享逻辑 |
| `utils` | 通用辅助 |

## 依赖与边界

- **MSRV**：crate 内 `rust-version = "1.80.0"`，与工作区一致。
- **工作区依赖**：版本只在根 `Cargo.toml` 的 `[workspace.dependencies]`；本 crate 用 `{ workspace = true }`。
- **错误**：本库对外主错误为 `thiserror` 的 `error::Error`，并 `impl From` 衔接 `serde_json::Error`、`config::ConfigError`。
- **可观测**：`tracing` + `tracing-subscriber`（env-filter、json）+ `tracing-appender`；与根上下文「结构化日志、少 `println!`」一致。
- **异步**：`tokio`、`tokio-util`、`futures`、`async-trait`。
- **配置与路径**：`config`、`dirs`；勿硬编码用户数据路径。
- **不反向依赖**上层 crate（agent、cli、gui 等）；保持底层库角色。

## 关键类型/入口

- **`pub use error::{Error, Result};`** — 全工作区常用的结果与错误枚举。
- **`config`**：`ConfigLoader`、schema 重导出；配置校验入口在 `validate`。
- **`bus`**：`AgentBusEvent`、`AgentEvent`、`InboundMessage`、`OutboundMessage`、`MessageBus`。
- **`logging`**：`init_logging` / `init_logging_with_terminal_output(config, enable_terminal)`，入参为配置中的 `LoggingConfig`。

## 实现约定

- 新增跨 crate 共享类型优先放此处，避免在 agent/gui 重复定义。
- 扩展 `Error` 变体时保持可诊断字符串一致，并考虑是否需新的 `From` 实现。
- `error_context` 中敏感/过长内容已有截断常量；新增日志或上下文时勿绕过截断泄露完整密钥或超大 payload。
- 配置字段变更需同步 `schema` / `validate` 与加载逻辑，避免破坏现有 `config.json` 契约。
- 异步 API 保持 `Send`/`Sync`/`'static` 与调用方（Tokio）一致。

## 测试与检查

- **集成测试目录**：当前无顶层 `tests/`；测试以各模块内 `#[cfg(test)]` 为主（如 `bus/queue`、`config/loader`、`cron/service`、`session`、`memory`、`heartbeat` 等）。
- **dev-dependencies**：`tokio-test`、`tempfile`。
- 提交前随工作区执行：`cargo fmt`、`cargo clippy --all -- -D warnings`、`cargo test --all`（或 `just check` / `just test`）。

## 切勿遗漏

- 不在此 crate 引入 GUI、CLI 或单一 channel 专用依赖。
- 不在生产路径对配置/IO/网络结果随意 `unwrap`/`expect`；与根上下文「anyhow 在应用边界、thiserror 在库内」分工时，**本 crate 侧以 `Error`/`Result` 表达可恢复失败**。
- 修改 `MessageBus` 或事件枚举会影响所有经总线的消息流，需评估 agent 与 channels。
