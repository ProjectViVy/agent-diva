---
project_name: agent-diva-neuron
date: '2026-03-30'
module: agent-diva/agent-diva-neuron
status: complete
parent_workspace: agent-diva
optimized_for_llm: true
---

# agent-diva-neuron — crate 上下文

面向 LLM 的精简说明；全仓库规则见 `agent-diva/project-context.md`。

## 模块职责

**单次、非循环**的 LLM「神经元」抽象：请求/响应契约、`NeuronNode` trait、基于 `LLMProvider` 的默认实现，以及供上层图编排预留的**本地事件协议**。

- 不包含 agent 主循环、工具执行闭环；`NeuronResponse` 中 tool 仅为透传意图。
- 设计目标：可组合、可替换执行器，便于未来图级编排消费 `NeuronEvent` 流。

## 依赖与边界

- **MSRV**：以工作区根 `rust-version = "1.80.0"` 为准（本 crate `Cargo.toml` 未单独写 `rust-version` 时仍按工作区执行）。
- **工作区依赖**：`async-trait`、`tokio`、`futures`、`serde`、`serde_json`、`thiserror`、`tracing`、`uuid` 均 `{ workspace = true }`。
- **唯一 path 依赖**：`agent-diva-providers` — 使用 `Message`、`ToolCallRequest`、`LLMProvider`、`LLMResponse`、`LLMStreamEvent`、`ProviderError`。
- **不依赖** `agent-diva-core`、`agent-diva-agent`：保持神经元层仅面向 Provider 抽象，避免循环依赖。
- **错误**：对外 `NeuronError`（`thiserror`），含 `Provider`/`InvalidInput`/`Internal`；与根上下文「库内 thiserror」一致。本 crate **未**使用 `anyhow`。

## 关键类型/入口

| 符号 | 说明 |
|------|------|
| `NeuronRequest` / `NeuronResponse` | 单次调用的输入输出（serde）；含 `model`、`max_tokens`、`temperature`、`metadata`；响应含 `content`、`reasoning_content`、`tool_calls`、`finish_reason`、`usage` |
| `NeuronNode` | `async_trait` trait：`run_once`、`run_once_stream(..., event_tx)`；`Send + Sync` |
| `LlmNeuron` | 默认实现：持有 `Arc<dyn LLMProvider>` + `neuron_id`（`new` / `with_id` / `neuron_id()`） |
| `NeuronEvent` | `Started`、`TextDelta`、`ReasoningDelta`、`Completed`、`Failed`；可序列化，供流式观测 |
| `NeuronError` | 公开错误类型 |

## 实现约定

- `run_once` 默认委托 `run_once_stream(req, None)`。
- 空 `messages` 必须返回 `NeuronError::InvalidInput`（与 `LlmNeuron` 现逻辑一致）。
- 流式路径：消费 `chat_stream`，聚合文本与 reasoning；`LLMStreamEvent::ToolCallDelta` 当前保留、**忽略**（注释标明 v0）；失败时向 `event_tx` 发 `Failed` 并返回 `Provider` 错误。
- 若流结束无 `Completed` 事件，使用聚合文本构造兜底 `LLMResponse`（`finish_reason` 等默认值与现有代码一致）。
- 事件发送对 `mpsc` 使用 `let _ = tx.send(...)` — 通道关闭时静默丢弃；上层若需强保证需自行背压策略。
- 新功能优先扩展 `metadata` / 事件变体，避免破坏 `NeuronRequest`/`NeuronResponse` 的 serde 向后兼容。

## 测试与检查

- **当前无**模块内 `#[cfg(test)]` 与顶层 `tests/`；新增逻辑时建议用 **mock provider** 或 `agent-diva-providers` 现有测试模式补单元测试。
- 随工作区：`cargo test -p agent-diva-neuron`、`clippy -D warnings`。
- `dev-dependencies` 仅 `tokio`（测试运行时）；扩展异步测试时可对齐 core 使用 `tokio-test` 等工作区 dev 依赖。

## 切勿遗漏

- 不在此 crate 直接调用具体渠道或工具 runner；仅通过 `LLMProvider`。
- 变更 `NeuronNode` 或 `NeuronEvent` 为破坏性 API 时，全局搜索 `agent-diva-neuron` 的引用方（agent / service 等）。
- 流式事件与 tracing：需要可观测性时在调用方补充 `tracing` span，本 crate 已依赖 `tracing` 但未强制在每个路径打点。
