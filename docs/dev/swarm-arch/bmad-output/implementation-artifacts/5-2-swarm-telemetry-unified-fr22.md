---
story_key: 5-2-swarm-telemetry-unified-fr22
story_id: "5.2"
epic: 5
status: review
generated: "2026-03-31T18:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 5.2：蜂群内部步与 RunTelemetry / FR22 统一

## 故事陈述

作为 **开发者 / 排障者**，  
我希望 **一次 turn 内「序曲 LLM 调用次数 + 主循环迭代」在 RunTelemetry、过程事件语义与 doctor 输出中一致**，  
以便 **FR22 可信、用户不被两套数字搞糊涂**。

## 验收标准

1. **Given** 一次 FullSwarm turn（含序曲）  
   **When** 比较 `RunTelemetrySnapshotV0`（或后继版本）、相关过程事件载荷、可选 `doctor` 块  
   **Then** **内部步数定义** 在维护者文档中只有 **一种** 权威解释（含：序曲每角色是否计 1 步、主循环每 iteration 如何计）

2. **And** Light 路径 **不** 因本故事增加误导性「蜂群步数」（保持 FR19）

3. **And** 若需 DTO 演进，须 **schemaVersion** 或等价版本字段（NFR-I1）

## 任务分解（Dev）

- [x] 审计当前：`loop_turn` 迭代、`RunTelemetrySnapshotV0`、FR22 UI hint 的数据源
- [x] 定义单一计数器或 **显式字段拆分**（如 `prelude_llm_calls` + `main_iterations`），避免强行捏成一个数造成歧义
- [x] 更新 `doctor` / CLI 可选块（若已有）与 GUI hint 文案来源
- [x] 文档 + 测试

### Review Findings

- [x] [Review][Decision] 序曲 LLM 失败时管道 phase 与遥测 prelude 字段不一致 — **已处理**：采用（A）+（B）— 各角色在 `chat` **成功之后** 再 `try_emit(swarm_phase_changed)`；`chat` 失败时通过 `PreludeRunError { llm_calls, phase_events, source }` 回传已累计计数，`loop_turn` 的 `Err` 分支写入 `prelude_llm_calls` / `prelude_swarm_phase_events`。新增单测 `swarm_prelude_second_chat_failure_partial_counts_match_pipeline`；`RUN_TELEMETRY_FR22.md` 已同步语义。

- [x] [Review][Defer] 会话中途取消（`return Ok(None)`）时不发射 `RunTelemetry` — 既存行为，非 Story 5.2 引入；若未来要求「部分 turn 也有遥测」可单独立项。[`loop_turn.rs` 约 165–167、270–272、385–387] — deferred, pre-existing

- [x] [Review][Patch] `phaseCount` 文档与皮层 OFF 行为 — 当 `explicit_full_swarm` 等为 FullSwarm 但 `CortexRuntime::enabled == false` 时，`ProcessEventPipeline::try_emit` 不投递（见 `PROCESS_EVENTS_CORTEX_OFF.md`），而 `loop_turn` 仍按迭代/序曲逻辑累加 `phase_count` / `prelude_swarm_phase_events`。维护者文档表 1 写「与 try_emit 次数对齐」易被理解为与下游可见事件严格一致；应在 `RUN_TELEMETRY_FR22.md` 增加脚注或「皮层启用时」限定，并交叉引用皮层关文档。[`RUN_TELEMETRY_FR22.md`、`process_events.rs` try_emit、`execution_tier.rs`] — **已处理**：表内 `phaseCount` 改为「逻辑计数」+ 皮层 ON 与下游一致；新增「皮层 OFF 与 explicit_full_swarm」小节。

- [x] [Review][Patch] GUI 折叠单行与 CLI 遥测行字段不齐 — CLI `[run_telemetry]` 含 `convergence={:?}`，而 `RunTelemetryHint.vue` 的 `runTelemetryLine` 仅 main/prelude/phases；展开区已显示收敛轮次。若要求「同一摘要维度」，可在 `fullSwarmConvergenceRounds != null` 时向单行追加（并扩展 `en.ts` / `zh.ts` 文案键）。[`RunTelemetryHint.vue`、`locales/en.ts`、`locales/zh.ts`] — **已处理**：`runTelemetryLineConvergencePart` + 折叠行拼接。

## 依赖

- **5.1** 完成更佳（序曲轮次可变后计数才有压力测试场景）

## Dev Agent Record

### Implementation Plan

- 扩展 `RunTelemetrySnapshotV0`：`prelude_llm_calls`、`full_swarm_convergence_rounds`（可选）；`schema_version` 发射为 **1**；明确 `internal_step_count` = 主 ReAct 迭代；`phase_count` = 序曲 `swarm_phase_changed` + 主循环（有管道时）。
- `run_swarm_deliberation_prelude` 返回 `(summary, llm_calls, phase_events)`；失败时为 `PreludeRunError`（含部分计数）。角色相位在 **成功 `chat` 之后** 发射，与 `preludeLlmCalls` 对齐。`loop_turn` 在 **FinalResponse 之前** 执行收敛循环以填入 `full_swarm_convergence_rounds` 并发出终局过程事件。
- Headless 映射：`internal_step_count=0`，收敛轮次进 `full_swarm_convergence_rounds`。
- 维护者文档：`agent-diva-swarm/docs/RUN_TELEMETRY_FR22.md`；GUI/CLI/README 同步。

### Debug Log

- （无）

### Completion Notes

- ✅ AC1：`RUN_TELEMETRY_FR22.md` 为字段语义单点；DTO 与 `loop_turn` / `run_telemetry_from_minimal_turn_trace` 对齐。
- ✅ AC2：Light 路径 `prelude_llm_calls` 恒为 0，文案区分主循环与序曲。
- ✅ AC3：`schema_version` 升级为 **1**，旧 JSON 缺省字段可反序列化（`prelude_llm_calls` default）。
- 已运行：`cargo test -p agent-diva-core --lib`、`cargo test -p agent-diva-swarm run_telemetry`、`cargo test -p agent-diva-agent swarm_prelude`、`cargo test --workspace`（`agent-diva`，exit 0）。
- Code review 闭环：序曲中途失败时遥测与管道 `swarm_phase_changed` 一致（`PreludeRunError` + 相位后置）。

## File List

- `agent-diva/agent-diva-core/src/bus/run_telemetry.rs`
- `agent-diva/agent-diva-core/src/bus/mod.rs`
- `agent-diva/agent-diva-swarm/src/run_telemetry.rs`
- `agent-diva/agent-diva-swarm/docs/RUN_TELEMETRY_FR22.md`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva/agent-diva-gui/src/api/runTelemetry.ts`
- `agent-diva/agent-diva-gui/src/components/RunTelemetryHint.vue`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/README.md`
- `agent-diva/agent-diva-cli/src/main.rs`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

- 2026-03-31：Story 5.2 — FR22 遥测字段拆分、序曲/主循环/收敛语义统一、文档与 GUI/CLI 更新（Dev Agent）。
- 2026-03-31：序曲 `chat` 失败路径 — `PreludeRunError` 部分计数、`swarm_phase_changed` 在成功调用后发射、`RUN_TELEMETRY_FR22.md` 与单测。
- 2026-03-31：Code review 选项 0 批量修复 — `RUN_TELEMETRY_FR22.md` 皮层 OFF / `phaseCount` 语义；`RunTelemetryHint` 折叠行追加收敛轮次与 i18n。

## Status

done
