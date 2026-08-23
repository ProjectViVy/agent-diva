# Summary — PLAN-MODE-PHYSICAL-STATE-MACHINE contract freeze

- 版本：`v0.1.0-contract-freeze`
- 日期：2026-08-23
- 类型：已实现合同验收冻结（不重建 permission mode）

## 当前合同

| 状态 | 允许的 ToolCapability |
|---|---|
| Exploring / Drafting | Inspect, PlanningRecord |
| AwaitingApproval | Inspect |
| Executing | 除 Unknown 外全部 |
| Verifying | Inspect, PlanningRecord, Execute |
| Closed（Completed / Failed / Partial） | 无 |

非法迁移由 `validate_transition` 拒绝。执行 seam 由
`ToolStepPolicy::denial_reason` 二次拒绝（含无 persisted plan 时仅 Inspect /
PlanningRecord）。

## 做了什么

- 独立验收记录（本目录）。
- `ToolStepPolicy` 补一条按 phase 的 fail-closed 矩阵测。
- **未**按旧报告重建 permission mode，未改 `policy.rs` 矩阵。

## 权威测试

- `agent-diva-core/src/planning/policy.rs`：phase 投影、矩阵、合法/非法迁移。
- `agent-diva-agent/src/tool_assembly.rs`：`planning_phase_filters_registry_with_the_core_capability_policy`。
- `agent-diva-agent/src/agent_loop/turn/tool_step.rs`：seam 二次拒绝 + 本 slice 矩阵测。
