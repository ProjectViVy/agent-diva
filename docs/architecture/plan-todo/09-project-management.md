# Plan/TODO Architecture Project Management Plan

## 1. Active goal

Implement the Plan/TODO architecture described in `docs/architecture/plan-todo/01-architecture-exploration.md` through `13-acceptance-criteria.md`. This is the only active project-management stream. The runtime goal is: read-only exploration → complete reviewable plan → explicit approval → optional execution TODO materialization → writable execution → verification.

## 2. Work breakdown and dependency order

| Wave | Deliverable | Main locations | Depends on |
| --- | --- | --- | --- |
| P1 | Core state machine and capability policy | `agent-diva-core/src/planning/` | None |
| P2 | Plan submission, revision-bound approval, optional TODO materialization | `agent-diva-tools/src/planning/`, `agent-diva-agent/src/agent_loop/loop_runtime_control.rs` | P1 |
| P3 | Agent-loop enforcement at tool assembly and invocation | `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/tool_assembly.rs` | P1, P2 |
| P4 | GUI phase projection and approval UX | `agent-diva-gui/src/App.vue`, planning components | P2, P3 interface contract |
| P5 | Regression, migration and release validation | crate tests, GUI tests, `TODOLIST.md` | P1–P4 |

Each wave is one atomic change set. Code and documentation changes remain separate commits. Branches must start from `pro`; do not include unrelated cleanup.

## 3. Execution checklist

- [ ] P1: define `PlanModeState`, `ToolCapability`, legal transitions, and table-driven policy tests.
- [ ] P2: add plan completeness validation, `plan_revision` approval receipts, stale-approval rejection, and `TodoPolicy::{Never,Optional,Always}`.
- [ ] P3: replace the current name-only plan allowlist with the capability policy at registry, pre-call, and post-call boundaries.
- [ ] P4: make the backend state authoritative in the GUI; show read-only capability, frozen revision, complete plan, approval, and optional TODO choice.
- [ ] P5: cover no-write exploration, approval conflicts, no-TODO execution, generated-TODO atomicity, migration, and full CI.

## 4. TODO policy

TODO is optional. The planner may propose candidate work items, but execution TODOs are materialized only after approval and only when the user or plan selects `Optional` or `Always`. A task with no useful decomposition may proceed without a TODO list.

## 5. Deferral policy

All previously activated work is deferred until P1–P5 completes. Deferred work must remain recorded in `TODOLIST.md`; completed items must not be deleted or reopened. Re-activation requires a new focused plan and dependency review.

## 6. Validation and exit criteria

For each code wave run `just fmt-check && just check && just test` (or the focused crate equivalent), then update the iteration log. The project stream is complete only when every pre-approval mutating capability is denied by runtime policy, approval is revision-bound and auditable, optional TODO behavior is tested, and the GUI reflects backend state without synthetic approval messages.
