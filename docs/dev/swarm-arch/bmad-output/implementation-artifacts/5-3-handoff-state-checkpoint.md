---
story_key: 5-3-handoff-state-checkpoint
story_id: "5.3"
epic: 5
status: done
generated: "2026-03-31T18:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
---

# Story 5.3：Handoff 检查点语义（turn 内基线）

状态：done

## 故事陈述

作为 **运行时**，  
我希望 **在序曲链上定义可序列化的检查点**（角色 id、轮次、摘要哈希或截断正文），  
以便 **在可恢复失败时解释进度**（**用户取消** 的保证范围见验收标准 v0 边界），并为 **未来跨 turn 恢复** 留扩展点。

## 验收标准

1. **Given** 序曲执行中发生 **可恢复失败**（超时、模型错误）  
   **When** 运行时中止序曲并进入错误处理  
   **Then** 开发者可通过日志（目标 `agent_diva_agent::prelude`，字段清单见 `agent-diva-swarm/docs/handoff-checkpoint-v0.md`）查询 **最后成功检查点**（若此前至少一步成功）

   **And（v0 边界 — 用户取消）** 会话取消在实现上于 **序曲 `await` 结束之后**、主 **ReAct** `agent_turn` 循环内轮询；**不在** 序曲串行 `provider.chat` 期间轮询。**v0 不承诺**「仅在某一序曲 `chat` 尚未返回、用户已点取消」时 **必定** 出现带 `checkpoint_json` 的序曲失败路径 `warn!`；若需该保证，留待后续故事。

2. **And** MVP 行为在文档中 **二选一封冻**：**仅报告** vs **同 turn 自动续跑**（禁止未说明的第三种 silently 丢状态）

3. **And** 检查点内容 **限长、消毒**（NFR-S2）；**不** 默认写入用户 transcript（NFR-R2）

## 任务分解（Dev）

- [x] 定义 `SwarmHandoffCheckpointV0`（或等价）结构与存放位置（进程内 vs 会话存储 —— 与 Epic 1 持久化边界一致）
- [x] 与 `ProcessEventV0` 的关联方式（可选：仅日志，不进白名单事件 —— 需 PM/架构确认）
- [x] 测试：模拟序曲第二轮失败，断言检查点内容

## 依赖

- **5.1**（可配置序曲链）优先

## Dev Notes

- DTO：`agent-diva-swarm/src/handoff_checkpoint.rs`，`schemaVersion=0`，字段 `roleId`、`preludeRoundIndex`、`summaryPreview`、`contentFingerprintHex`（FNV-1a 64）。
- 序曲失败：`PreludeRunError.checkpoint`；`tracing` 目标 `agent_diva_agent::prelude`，字段 `checkpoint_json` + `error`。
- **ProcessEventV0：** MVP **不** 新增白名单事件；检查点仅结构化日志（见 `docs/handoff-checkpoint-v0.md`）。
- **MVP 封冻：** **仅报告** — 同 turn 不自动重试序曲，主循环无序曲摘要继续。

## Dev Agent Record

### Implementation Plan

1. 在 swarm crate 新增 handoff DTO 与单元测试。
2. 序曲循环内维护 `last_checkpoint`，`chat` 失败时写入 `PreludeRunError`。
3. AgentLoop 错误分支输出结构化 `warn!`。
4. 文档冻结字段与 MVP；README 链入。

### Debug Log

- 本机默认 `target` 目录曾出现 file lock；使用独立 `CARGO_TARGET_DIR` 可完成 `cargo test`。

### Completion Notes

- 已实现 `SwarmHandoffCheckpointV0`、序曲失败路径检查点、`swarm_prelude_second_chat_failure_*` 测试扩展与 `handoff_checkpoint` 模块测试。
- `agent-diva-swarm/docs/handoff-checkpoint-v0.md` 冻结字段清单与「仅报告」MVP；`README.md` 增加入口链接。

## File List

- `agent-diva/agent-diva-swarm/src/handoff_checkpoint.rs`（新）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/docs/handoff-checkpoint-v0.md`（新）
- `agent-diva/agent-diva-swarm/README.md`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/5-3-handoff-state-checkpoint.md`

## Change Log

- **2026-04-01：** Code review 决议 **(1)** — 收窄 AC1 与 handoff v0 文档中「用户取消」边界；story / sprint `5-3-handoff-state-checkpoint` → **done**；Epic 5 全部 story 已完成 → `epic-5` → **done**。
- **2026-04-01（BMAD code-review 5-3）：** 三层审查（Blind / Edge / Acceptance）复核完成。除「Review Findings」中已登记的 **[Review][Decision] 用户取消 vs AC1** 外，无新增 Patch/Defer；AC2（仅报告 MVP）与 AC3（限长消毒、不经 transcript）与实现及 `handoff-checkpoint-v0.md` 一致。相关测试：`agent-diva-swarm` `handoff_checkpoint::*`、`agent-diva-agent` `swarm_prelude_second_chat_failure_partial_counts_match_pipeline` 均已通过。
- **2026-04-01：** Story 5.3 — Handoff 检查点 v0、序曲失败日志、文档与测试；sprint 状态 → review。

### Review Findings

- [x] [Review][Decision] 验收 AC1 与「用户取消」场景 — 已选 **(1)**：**2026-04-01** 收窄故事 AC1 与 `handoff-checkpoint-v0.md`（v0 边界：取消在序曲 `chat` 挂起期间不纳入书面保证；可恢复失败路径仍保证检查点日志）。

- [x] [Review][Defer] FullSwarm 且无 `process_event_pipeline` 时不调用序曲 — `loop_turn.rs` 约 L131 仅在 `process_event_pipeline` 为 `Some` 时进入 `run_swarm_deliberation_prelude`，与 FR22 / `run_telemetry` 既有语义一致，非 Story 5.3 引入 — deferred, pre-existing

## Status

done
