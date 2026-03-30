# 过程事件白名单 v0（FR2 / NFR-I2）

**范围：** Rust 侧可订阅的 **最小过程事件** DTO，与 `ProcessEventV0` / `ProcessEventNameV0`（`src/process_events.rs`）一致。GUI 经 Tauri 等适配层消费时须仅依赖本清单字段，禁止「通用 JSON blob」式无限膨胀。

**版本：** `schemaVersion`（JSON **camelCase**）与皮层契约相同语义；v0 固定为 **`0`**。新增事件名或字段须 bump 版本或经 ADR。

## 白名单事件名（wire：`snake_case`）

| `name` | 含义 |
|--------|------|
| `swarm_phase_changed` | 执行阶段推进（当前实现：Agent 迭代开始）。 |
| `tool_call_started` | 工具调用开始（仅摘要级 `message`，完整参数在会话/总线）。 |
| `tool_call_finished` | 工具调用结束（摘要级结果预览，大块内容不进入事件体）。 |

> **预留：** Story 1.8 等可扩展 `swarm_run_finished` 等终局事件；本 v0 不实现。

## 载荷字段（`ProcessEventV0`）

| 字段（camelCase） | 类型 | 说明 |
|-------------------|------|------|
| `schemaVersion` | `u32` | 契约版本，v0 = `0`。 |
| `name` | 上表枚举 | 事件种类。 |
| `message` | `string` | 短句、预览级说明。 |
| `phaseId` | `string?` | 阶段相关 id（如 `agent_iteration_1`）。 |
| `correlationId` | `string?` | 与工具调用等关联（如 LLM `call_id`）。 |
| `toolName` | `string?` | 工具名（工具类事件）。 |

## 单一出口与接线

- **发射逻辑：** `agent-diva-agent` 的 `AgentLoop` 在迭代开始与工具起止处调用 `ProcessEventPipeline::try_emit`（见 Story 1.5 实现）。
- **门控：** 管道持有 `Arc<CortexRuntime>`；**皮层关** 时不发射（见 [`PROCESS_EVENTS_CORTEX_OFF.md`](./PROCESS_EVENTS_CORTEX_OFF.md)）。

## 节流（NFR-P2）

默认配置见 `ProcessEventThrottleConfig::default()`：

- **`swarm_phase_changed`：** 合并窗口 **100ms** 或缓冲 **32** 条（先达者）触发一批 `deliver_batch`。
- **`tool_call_started` / `tool_call_finished`：** 每次入缓冲后 **立即 flush** 当前整批，避免工具里程碑长时间延迟。

Turn 结束时通过 `ProcessEventPipeline::flush_pending`（由 `AgentLoop` 内 `Drop` 守卫触发）刷出未达时间窗的 phase 事件。

Tauri `emit` 等重逻辑须在 **非 UI 阻塞** 上下文执行（与 `architecture.md` NFR-P1/P2 一致）。

## 相关文档

- [`../README.md`](../README.md) — 蜂群 crate 总览  
- [`../../../docs/swarm-cortex-contract-v0.md`](../../../docs/swarm-cortex-contract-v0.md) — 皮层 command / `cortex_toggled`  
- [`./PROCESS_EVENTS_CORTEX_OFF.md`](./PROCESS_EVENTS_CORTEX_OFF.md) — 皮层关与过程事件  
