# Plan/TODO P1–P3 Historical Review Disposition

归档时间：2026-07-30。

本表处置根 `TODOLIST.md` 中 `Superseded Review Evidence` 下、基于
2026-07-11 tool-oriented 基线的三路评审条目。它们**不是**当前 Active Plan。

**总体结论：** 2026-07 中下旬 revision-bound report/store/runtime 闭环
（提交 `82c25856`、`a1c1389e`、`91be604b`、`52db71ac` 及后续 Plan 路径演进）
已取代该基线。主清单仅保留对照**当前代码**仍成立的 residual。

相关日志：

- `docs/logs/2026-07-plan-todo-p2-p3/v0.0.1-approval-runtime-enforcement/`
- `docs/archive/todolist/completed-through-2026-07-29.md`（已记 P2/P3 boundary 完成）

## Disposition Legend

| 状态 | 含义 |
|------|------|
| CLOSED | 当前实现已满足原意图 |
| SUPERSEDED | 旧基线问题被新架构消解，不再按原文返工 |
| OBSOLETE | 相关 API/路径已删除 |
| OPEN-RESIDUAL | 仍可能成立；主清单有改写后的条目 |

## P1 core policy（2026-07-11）

| 历史标题（摘要） | Disposition | 证据 / 说明 |
|------------------|-------------|-------------|
| Executing must fail-closed for future ToolCapability | CLOSED | `agent-diva-core/src/planning/policy.rs` `allows()` 对 Executing 使用显式 allowlist；Unknown 全状态拒绝 |
| Verifying capability vs Verify exits | CLOSED | Verifying 允许 Inspect/PlanningRecord/Execute |
| dual transition matrix vs PlanOrchestrator | CLOSED | `orchestrator.rs` 委托 `policy::is_valid_transition` |
| expand invalid-transition / terminal-reentry tests | OPEN-RESIDUAL | 有合法边与 Failed 规则测试，但非完整 8×8 笛卡尔；见主清单 Plan residual |
| document or tighten Failed bailout / identity edges | CLOSED | 终端态不可再 Failed；Failed 应急仅非终端 from |
| decide AwaitingApproval freeze for PlanningRecord | CLOSED | AwaitingApproval 仅 Inspect；store reopen 使旧 revision 失效 |
| Closed projection collapses Completed/Failed/Partial | SUPERSEDED | 运行时 `policy_phase_for` 对终端返回 `None`，不靠 Closed 锁死会话 |
| Unknown-during-Execute vs architecture edge table | CLOSED | Unknown 一律 deny，与 fail-closed 一致 |
| API usability helpers (`allows_for_phase`) | CLOSED | `allows_for_phase` 已存在并被 assembly/tool_step 使用 |
| simplify Executing match arms | CLOSED | 随 fail-closed allowlist 一并收敛 |
| dead commented orchestrator matrix | OPEN-RESIDUAL | `orchestrator.rs` 仍保留大段注释旧矩阵；见 residual chore |

## P2 approval materialization（2026-07-11）

| 历史标题（摘要） | Disposition | 证据 / 说明 |
|------------------|-------------|-------------|
| register `plan_submit` and allow in plan mode | SUPERSEDED | 当前 `tool_assembly` 不再注册 legacy plan_create/submit/transition 工具面；执行态仅注册 execution todo 工具。产品已转向 report/runtime 审批 |
| close dual lifecycle into AwaitingApproval/Execute | SUPERSEDED | revision-bound submit/approve 与 runtime 路径取代双 authority |
| freeze content or reopen-on-edit | CLOSED | `store.rs` content_changed → reopen；单测 `content_edit_reopens_submitted_plan_*` |
| enforce AwaitingApproval inspect-only at dispatch | CLOSED | policy + `allows_for_phase` 过滤 registry/invoke |
| approve API compatibility + surface revision | SUPERSEDED | 旧 empty-POST / global approve 路径已演进；`ApproveActivePlan` 硬失败 |
| Always/Optional materialize vs pre-existing TODOs | OPEN-RESIDUAL | `TodoAlreadyMaterialized` 仍存在；产品策略（成功 vs 永久卡住）需明确 |
| approval/submit audit events typed | SUPERSEDED | 事件模型随 report/store 演进；不以 7/11 工具路径返工 |
| collapse dual transition authority | CLOSED | orchestrator 委托 core |
| submit_plan phase CAS must not skip Plan | SUPERSEDED | 与 store submit 实现及 report 路径绑定；旧工具入口不再是主路径 |
| AC matrix tests incomplete | OPEN-RESIDUAL | assembly 有分阶段子集测试，非完整 phase×tool 矩阵 |
| stop silent full-replace of TODOs | SUPERSEDED/部分 CLOSED | materialize 保护存在；完整产品语义随 execution todo 路径演进 |
| empty TODO Execute→Verify/Completed gate | OPEN-RESIDUAL | `PlanVerifier::verify` 在 `total == 0` 时判定 Pass |
| wrong-phase submit as ApprovalConflict | SUPERSEDED | 旧错误面；不按 7/11 工具 API 返工 |
| persist todo_policy with stable serde | SUPERSEDED | 随 store 演进；无独立活跃投诉 |
| unregister dead `plan_approve` tool | SUPERSEDED | legacy plan 工具面已不在 assembly 注册路径 |
| return ApprovalReceipt on approve-execute | SUPERSEDED | 治理 receipt 模型在 GMH-10/12；非旧 plan_approve 工具响应 |

## P3 runtime capability gate（2026-07-11）

| 历史标题（摘要） | Disposition | 证据 / 说明 |
|------------------|-------------|-------------|
| terminal active plan must not deny all tools | CLOSED | `policy_phase_for` 对 Completed/Failed/Partial 返回 `None`；有单测 |
| rebuild registry after ApproveActivePlan → Execute | OBSOLETE | `ApproveActivePlan` 返回 `legacy global plan approval has been removed` |
| mid-turn rebuild same policy_phase derivation | CLOSED | tool_step 使用共享 `policy_phase_for` |
| agent-loop integration fixture for AC1/AC2 | OPEN-RESIDUAL | 单元/装配测试为主，完整 agent-loop 集成夹具仍薄 |
| iteration log at required path | SUPERSEDED | 有 `2026-07-plan-todo-p2-p3` 日志；旧绝对路径要求不再强制 |
| full phase×tool I/O matrix in assembly tests | OPEN-RESIDUAL | 子集覆盖存在；spawn/cron/enqueue 等未全表 |
| prove denied invoke has no side effects | OPEN-RESIDUAL | 需补充 denial 前后 store 快照断言类测试 |
| record verification command evidence | SUPERSEDED | 后续多次 `just` 门禁已在其它日志记录 |
| plan_mode vs active-plan precedence | CLOSED | `policy_phase_for`：显式 plan_mode 优先；文档注释说明 |
| MCP/network re-register must re-apply phase filter | OPEN-RESIDUAL | `rebuild_tools_for_active_phase` 传 `plan_phase=None` |
| align plan-guard prompt with WorkItem policy | SUPERSEDED | 随 prompt/工具面演进 |
| delete dead commented orchestrator matrix | OPEN-RESIDUAL | 同 P1 residual chore |

## Mapping to main TODOLIST residuals

| Residual ID | 对应历史项 |
|-------------|------------|
| Plan residual: matrix tests | P1 expand tests；P2/P3 AC matrix / denial side-effect |
| Plan residual: phase-aware rebuild | P3 MCP/network rebuild phase filter |
| Plan residual: empty execution list gate | P2 empty TODO verify Pass |
| Plan residual: materialize pre-existing policy | P2 TodoAlreadyMaterialized |
| Plan residual: delete dead comment matrix | P1/P3 orchestrator comments |
