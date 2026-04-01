# RunTelemetry（FR22）字段语义 — 维护者单点（Story 5.2）

本文档为 **`RunTelemetrySnapshotV0` 各字段的唯一权威解释**；实现须与 `agent_diva_core::bus::run_telemetry`、`agent_diva_agent::agent_loop::loop_turn` 及 `run_telemetry_from_minimal_turn_trace` 保持一致。

## 实时路径（网关 / `AgentLoop`）

| 字段（JSON camelCase） | 含义 |
|-------------------------|------|
| `schemaVersion` | 当前发射为 **1**（v0 载荷仍可反序列化；缺省字段按 0 / `None` 处理）。 |
| `internalStepCount` | **主 ReAct 循环**迭代次数（`max_iterations` 预算内实际执行轮数），**不包含**蜂群序曲中的 LLM 调用。 |
| `preludeLlmCalls` | FullSwarm 且存在过程管道时，序曲 `run_swarm_deliberation_prelude` 内 **成功完成** 的 LLM `chat` 次数；否则 **0**。 |
| `phaseCount` | **逻辑计数**：序曲内每计 1 次相位 + 主循环每迭代 1 次（与代码路径上每次 `try_emit(swarm_phase_changed)` 的调用次数一致）。**皮层启用（`CortexRuntime::enabled == true`）且管道存在时**，与下游实际收到的 `swarm_phase_changed` 条数一致。若皮层 **OFF**，`ProcessEventPipeline::try_emit` 会丢弃事件（见 [`PROCESS_EVENTS_CORTEX_OFF.md`](./PROCESS_EVENTS_CORTEX_OFF.md)），但 `RunTelemetry` 仍会累加上述逻辑计数 — 订阅方勿仅用管道事件与 `phaseCount` 做严格相等校验。无管道则主循环段为 0。 |
| `fullSwarmConvergenceRounds` | FullSwarm turn 在遥测发射 **之前** 执行的 `execute_full_swarm_convergence_loop` 返回的 `rounds_completed`；非 FullSwarm 或无管道为 **省略** / `None`。 |
| `overSuggestedBudget` | 主循环触顶 `max_iterations` 仍无终局回复时为 `true`（琥珀提示，非阻断）。 |

### 序曲与主循环如何计数

- **序曲：** 每 **成功** 的序曲角色 LLM 调用 → `preludeLlmCalls += 1`；该角色对应的 `swarm_phase_changed` 在 **该次 `chat` 成功之后** 发射，故失败角色不会产生「有相位、遥测却记 0 次序曲 LLM」的错位。跳过/封顶/合并等 **无前置 LLM** 的相位仍各计 1 次 `phaseCount`（与管道一致）。
- **主循环：** 每进入一次迭代并最终在该迭代内对管道发射 `agent_iteration_k` → `internalStepCount` 与主循环段 `phaseCount` 各计一次（有管道时二者主循环部分相等）。

### 皮层 OFF 与 `explicit_full_swarm`

`resolve_execution_tier` 在 `explicit_full_swarm` 为真时仍可返回 **FullSwarm**（与皮层开关无关）。此时序曲 LLM 与主循环仍会执行，`preludeLlmCalls` / `internalStepCount` / `phaseCount` 照常累加，但过程管道在皮层 OFF 时不投递 v0 事件；语义以本表「逻辑计数」为准，并与 [`PROCESS_EVENTS_CORTEX_OFF.md`](./PROCESS_EVENTS_CORTEX_OFF.md) 一致。

## Light / 简化路径（FR19 / 皮层关）

- `preludeLlmCalls` **恒为 0**（不进入 FullSwarm 序曲）。
- `internalStepCount` / `phaseCount` 仍描述 **主代理循环**（有管道时相位与迭代对齐），**不**引入「蜂群步数」的额外误导语义。

## Headless 桩（`run_telemetry_from_minimal_turn_trace`）

最小 turn **无**主 ReAct、无序曲：

- `internalStepCount` = **0**。
- `preludeLlmCalls` = **0**。
- `phaseCount` = `MinimalTurnTrace::process_events_emitted`（终局 `swarm_run_*` 槽位语义，见 `minimal_turn.rs` 字段说明）。
- `fullSwarmConvergenceRounds`：仅当 `layer == FullSwarmOrchestration` 时为 `Some(full_swarm_internal_rounds)`，否则省略。

## 与 CLI / GUI 的关系

- TUI 时间线：`[run_telemetry] mainLoop=… preludeLlm=… phases=… convergence=…`（`agent-diva-cli`）。
- 设置 → 高级：`RunTelemetryHint.vue` 折叠单行与上述维度对齐（`fullSwarmConvergenceRounds` 非空时追加收敛轮次）；`runTelemetry.ts` 类型须与本表一致。
