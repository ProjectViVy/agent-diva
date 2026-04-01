---
project_name: agent-diva-agent
date: 2026-03-30
module: agent-diva-agent
status: complete
parent_workspace: agent-diva
---

# agent-diva-agent — 项目上下文

## 模块职责

- **Agent 主循环**：`AgentLoop` 从 `MessageBus` 收 `InboundMessage`，经多轮 LLM + 工具调用后发布 `OutboundMessage`；可选 `RuntimeControlCommand` 与入站消息 `tokio::select!` 合并处理。
- **上下文**：`ContextBuilder` 组装 system prompt（身份、时间、工作区、Soul、技能摘要/全量、记忆等），对接 `agent-diva-core` 的 `MemoryManager` / `SoulStateStore`。
- **技能**：`SkillsLoader` 解析工作区与内置 `SKILL.md`（YAML frontmatter + 运行时 JSON 元数据）。
- **子代理**：`SubagentManager` 后台任务，共享 `Arc<dyn LLMProvider>`，独立工具集与 `SpawnTool` 回调。
- **记忆整理**：`consolidation` 在会话消息超过 `DEFAULT_MEMORY_WINDOW`（100）时触发，通过 LLM + `save_memory` 工具 schema 写回长期记忆与 HISTORY。
- **工具配置**：`tool_config`（如 `NetworkToolConfig`）与 `SoulGovernanceSettings` 等运行时策略。

## 依赖与边界

| 依赖 | 用途 |
|------|------|
| `agent-diva-core` | 总线、会话、配置、Cron、错误上下文、Soul/记忆类型 |
| `agent-diva-providers` | 仅 `LLMProvider` trait 与消息/流类型（不实现具体 HTTP 客户端逻辑） |
| `agent-diva-tools` | `ToolRegistry`、文件/Shell/网络/MCP/Cron/Spawn 等工具注册 |
| **workspace** | `tokio`、`futures`、`async-trait`、`serde`/`serde_json`、`anyhow`/`thiserror`、`tracing`、`chrono`、`regex`、`uuid`、`which` |

**边界**：不定义 LLM HTTP 协议；不实现通道传输细节（由 core bus + 上层服务负责）。对外主要类型：`AgentLoop`、`ToolConfig`、`RuntimeControlCommand`、`AgentEvent`（re-export）。

## 关键类型/入口

- `AgentLoop::new` / `with_tools`：构造；`with_tools` 注册全套工具、MCP、cron、子代理与 `runtime_control_rx`。
- `AgentLoop::run`：异步主循环（取走 inbound receiver 后循环 `recv`）。
- `process_inbound_message` / `process_direct` / `process_direct_stream`：单条处理与 CLI/测试入口；流式路径向 `AgentEvent` 通道发事件。
- `SoulGovernanceSettings` + `soul_change_turns`：SOUL 频繁变更软提示窗口。
- `pub mod`：`agent_loop`、`consolidation`、`context`、`runtime_control`、`skills`、`subagent`、`tool_config`。

## 实现约定

- 异步运行时统一 **Tokio**；跨 await 的共享状态用 `Arc` + 内部可变性（如子代理 `Mutex`/`RwLock`）。
- 日志与追踪用 **tracing**（如 `AgentSpan` + `trace_id`）。
- 错误：对外 API 常见 `Box<dyn Error>`；工具层映射为 `ToolError`；入站处理失败时 `ErrorContext` + `AgentEvent::Error`。
- 序列化：工具 schema、消息体用 **serde_json**；时间展示用 **chrono::Local**。
- 子模块：`agent_loop` 内 `loop_turn` / `loop_tools` / `loop_runtime_control` 拆分回合、工具执行与运行时命令。

## 测试与检查

- **单元/集成**：`agent_loop.rs` 内 `#[cfg(test)]` — 创建循环、`process_direct` 结构、Soul 治理默认值、`FailingStreamProvider` 验证错误事件上报。
- **dev-deps**：`tokio-test`、`tempfile`（临时工作区）。
- 修改后建议：`cargo test -p agent-diva-agent`、`cargo clippy -p agent-diva-agent -- -D warnings`（与 workspace 策略一致时）。

## 切勿遗漏

- `with_tools` 与 `new` 行为差异：仅 `with_tools` 注册文件/Shell/网络/MCP/Cron/Spawn；测试若需完整工具链须用 `with_tools`。
- `run()` 会 **take** `MessageBus` 的 inbound receiver，重复调用或与其他消费者抢 receiver 会失败。
- `SkillsLoader` 默认内置技能目录为 crate 上级 `../skills`（相对 `CARGO_MANIFEST_DIR`）。
- 记忆整理依赖真实或可 mock 的 provider；窗口大小常量 `consolidation::DEFAULT_MEMORY_WINDOW`。
