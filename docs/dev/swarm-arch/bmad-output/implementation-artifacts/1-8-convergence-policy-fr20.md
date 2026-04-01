---
story_key: 1-8-convergence-policy-fr20
story_id: "1.8"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 1.8: 收敛策略与终局语义（FR20、NFR-P3）

Status: done

## Story

As a **开发者**,  
I want **蜂群路径具备最大内部轮次（或等价预算）与完成定义**,  
So that **编排不会无限「思考—推翻」且满足 NFR-P3**。

## Acceptance Criteria

1. **Given** `ConvergencePolicy`（或等价）在代码或配置中 **有默认值** 并写入维护者文档  
   **When** 皮层 **开** 且走 **FullSwarm**（或等价命名）路径  
   **Then** 每步检查预算；触顶时产生 **`StopReason`**（如 `Done` | `BudgetExceeded` | `Timeout` | `Error`）并 **emit 白名单事件**（如 `swarm_run_capped` / `swarm_run_finished`）

2. **And** **禁止** 仅依赖无上限内部对话作为唯一完成手段（**FR20**、**NFR-P3**）

3. **And** 无 GUI 测试覆盖 **触顶路径** 至少一种（例如预算耗尽 → `BudgetExceeded` + `swarm_run_capped`）

## Tasks / Subtasks

- [x] **默认策略与文档**（AC: #1, #2）  
  - [x] 在蜂群编排入口或共享类型中定义 **`ConvergencePolicy`**（或 ADR 冻结的等价名）：至少含 **`max_internal_rounds`（或等价预算）** 与 **完成定义（done）**；提供 **合理默认值**（常量或默认配置）  
  - [x] 维护者文档（README、`docs/` 或实现说明片段）写明：默认值含义、如何调整、与 **FR20 / NFR-P3** 的关系  
  - [x] 与 **`architecture.md` ADR-E** 中类型名、事件名 **对齐或在本 story 内冻结首版**（若与架构示例不同须在文档中显式对照表）

- [x] **每步预算检查**（AC: #1）  
  - [x] 在 **FullSwarm**（或等价）编排循环的 **每一步**（或每个可计费的内部 tick）递增/检查计数器，对照 `ConvergencePolicy`  
  - [x] 正常 **done** 路径：设置 **`StopReason::Done`**（或等价），并 emit **`swarm_run_finished`**（payload 含 **StopReason**，字段白名单与 serde 版本化约定与 **NFR-I2** 一致）

- [x] **触顶事件与 StopReason**（AC: #1）  
  - [x] 预算触顶：**`StopReason::BudgetExceeded`**（或架构枚举子集）+ emit **`swarm_run_capped`**  
  - [x] 超时 / 错误等：映射到 **`Timeout` / `Error`**（与 ADR-E 列举一致）；事件名与 **`swarm_run_finished` / `swarm_run_capped`** 的语义边界在代码注释或文档中写清（何种情况发哪一种）

- [x] **NFR-P3 门禁**（AC: #2）  
  - [x] 确认默认编排路径 **不可能** 仅靠「无上限内部多轮对话」结束；若存在可配置「无限」开关，**默认关闭** 且文档标注为 **非生产默认**

- [x] **无 GUI 测试：触顶路径**（AC: #3）  
  - [x] 新增 **`cargo test`**（或 workspace 等价）用例：**构造** 极低 `max_internal_rounds` 或 stub 循环，使运行 **必然触顶**  
  - [x] 断言：观察到 **`StopReason::BudgetExceeded`**（或等价）且 **`swarm_run_capped`**（或事件总线/收集器上的等价记录）被发出 **至少一次**  
  - [x] 不依赖 Tauri / 浏览器；可与 Story 1.5 事件契约的测试夹具共用（若已存在）

## Dev Notes

### 需求溯源

| 条款 | 含义摘要 |
|------|----------|
| **FR20** | 大脑皮层 **开** 且走蜂群编排时须有 **内置收敛策略**（最大内部轮次或等价预算、**done**），禁止无终止「思考—推翻—再思考」为 **默认**。 |
| **NFR-P3** | 编排默认值须在 **可接受延迟与调用次数** 下完成任务；**禁止** 将 **无上限内部多轮对话** 作为 **唯一** 完成手段（与 FR19–FR21 一致）。 |

### ADR-E（架构）— 须一致实现

- **ConvergencePolicy：** `max_internal_rounds`（或等价预算）、**完成定义（done）**；编排循环 **每步检查**。  
- **触顶 / 正常结束：** emit **白名单事件** — 示例名 **`swarm_run_finished`**、**`swarm_run_capped`** — 并携带 **`StopReason`**：`Done` | `BudgetExceeded` | `Timeout` | `Error` 等。  
- **可测性（架构原文）：** 无 GUI 用例覆盖 **触顶 StopReason** 等；本 story 落实 **触顶** 分支。

### Epic / 交叉故事

- **Story 2.3** 将订阅本 story 的终局/触顶事件，展示 **`lightweight`** / **`capped`** 与主对话区说明（**UX-DR4**）；本 story **不** 实现 GUI，但 **事件名与 payload 形状** 应稳定、可文档化。  
- 依赖：同 Epic 内仅依赖 **序号更小** 的故事；若编排骨架在 **1.1** 之后故事已建立，在本 crate / 模块内 **挂载** 策略与事件即可。

### 质量与栈

- MSRV **1.80.0**，`clippy -D warnings`，与 `agent-diva` workspace 一致。  
- 事件与 DTO：**serde**、字段 **白名单**、版本化（**NFR-I2**）。

## Dev Agent Record

### Implementation Plan

- 在 `agent-diva-swarm` 新增 `convergence.rs`：`ConvergencePolicy`、`execute_full_swarm_convergence_loop`、默认桩 `default_full_swarm_stub_is_done`。
- 扩展 `process_events.rs`：`SwarmRunStopReason`（ADR-E `StopReason`）、`swarm_run_finished` / `swarm_run_capped` 事件名与 DTO 字段 `stop_reason`；终局事件与工具事件同策略立即 flush。
- `minimal_turn.rs`：FullSwarm 走收敛循环；新增 `run_minimal_turn_headless_with_full_swarm_events`；`neuro_overview.rs` 映射新事件。
- 文档：`README.md`、`docs/process-events-v0.md`、`docs/CORTEX_OFF_SIMPLIFIED_MODE.md`。

### Debug Log

- **2026-03-31：** `cargo test -p agent-diva-swarm`、`cargo clippy -p agent-diva-swarm -- -D warnings` 通过（45 tests）；收敛循环在 code review 中修正 `is_done` / 预算检查顺序并新增 `max_internal_rounds_one_allows_stub_done_before_cap`。

### Completion Notes

- 默认 `max_internal_rounds = 256`，`allow_unbounded_internal_rounds = false`；完成谓词由调用方 `is_done` 注入（headless 桩为「一轮后 done」）。
- `SwarmRunStopReason` 与架构 `StopReason` 对齐；wire 名 `SwarmRunStopReason` 见 README 对照表。
- 无 GUI：`convergence::tests::budget_zero_never_done_emits_capped_with_budget_exceeded` 与 `minimal_turn::full_swarm_cap_observable_via_pipeline_without_gui` 覆盖触顶路径。
- **Code review 跟进：** `SwarmRunStopReason::as_stop_reason_wire_str` 与 serde camelCase 一致；神经总览 `NeuroActivityRowV0.detail` 对终局事件使用 `done` / `budgetExceeded` 等 wire 字面量（非 `Debug`）；`process_events` / `neuro_overview` 单测锁定该契约。
- **Code review（bmad-code-review）：** 有界模式下先 `is_done` 再预算触顶，修复 `max_internal_rounds == 1` 时误报 `BudgetExceeded`；`convergence::tests::max_internal_rounds_one_allows_stub_done_before_cap` 回归锁定。

## File List

- `agent-diva/agent-diva-swarm/src/convergence.rs`（新建）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/src/minimal_turn.rs`
- `agent-diva/agent-diva-swarm/src/process_events.rs`
- `agent-diva/agent-diva-swarm/src/neuro_overview.rs`
- `agent-diva/agent-diva-swarm/README.md`
- `agent-diva/agent-diva-swarm/docs/process-events-v0.md`
- `agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

- **2026-03-31：** bmad-code-review — 收敛循环 `is_done` 先于预算检查 + 边界单测；故事标为 `done`。
- **2026-03-31：** 评审跟进 — 神经总览终局行 `detail` 与 `stopReason` JSON camelCase 对齐；`SwarmRunStopReason::as_stop_reason_wire_str` + 单测。
- **2026-03-30：** Story 1.8 — FR20 收敛策略、`swarm_run_*` 终局事件、无 GUI 触顶测试与维护者文档。

### Review Findings

- [x] [Review][Patch] 神经总览行 `detail` 中 `stop_reason` 使用 `Debug`（如 `Done`），与过程事件 JSON 的 `camelCase`（`done` / `budgetExceeded` 等）不一致，可能误导 UI — [`agent-diva-swarm/src/neuro_overview.rs`](../../agent-diva/agent-diva-swarm/src/neuro_overview.rs) 约第 118 行
- [x] [Review][Defer] `SwarmRunStopReason::Timeout` 已在 DTO 与 `emit_swarm_terminal` 中预留，但 `execute_full_swarm_convergence_loop` 当前不产生 `Timeout`（仅 `Done` / `BudgetExceeded` / `Error` 熔断）；墙钟或外部取消接线后补测 — [`convergence.rs`](../../agent-diva/agent-diva-swarm/src/convergence.rs) — deferred, pre-existing gap for stub scope
- [x] [Review][Patch] 有界模式下先检查预算再 `is_done`，当 `max_internal_rounds == 1` 且完成谓词在 `rounds_completed == 1` 为真时会误发 `BudgetExceeded` — [`agent-diva-swarm/src/convergence.rs`](../../agent-diva/agent-diva-swarm/src/convergence.rs) `execute_full_swarm_convergence_loop` — **已修复**（先 `is_done` 再预算；新增 `max_internal_rounds_one_allows_stub_done_before_cap`）

### Senior Developer Review (AI)

- **Outcome:** Approve（残余为已登记的 Timeout 接线 defer）
- **Date:** 2026-03-31
- **Action items:** 1 patch（已修复并勾选）、0 decision、1 defer（沿用上文 Timeout）

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.8  
- `_bmad-output/planning-artifacts/architecture.md` — **ADR-E**（ExecutionTier、ConvergencePolicy、StopReason、`swarm_run_*`）  
- `_bmad-output/planning-artifacts/prd.md` — **FR20**、**NFR-P3**  
