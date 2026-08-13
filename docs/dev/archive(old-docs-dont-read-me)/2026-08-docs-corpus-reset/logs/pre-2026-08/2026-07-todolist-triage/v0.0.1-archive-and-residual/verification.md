# Verification

## Scope

Documentation and backlog triage only. No `cargo` / `just` runtime validation
required for behavior change. Spot-checks used repository search against current
sources.

## Spot-check evidence (OPEN-RESIDUAL)

| Residual | Check | Result |
|----------|-------|--------|
| policy fail-closed Executing | `agent-diva-core/src/planning/policy.rs` `allows()` | Explicit Executing allowlist; Unknown denied → historical P1 CLOSED |
| dual transition | `orchestrator.rs` `is_valid_transition` | Delegates to core policy → CLOSED |
| terminal phase unlock | `agent_loop.rs` `policy_phase_for` | Completed/Failed/Partial → `None` → CLOSED |
| ApproveActivePlan rebuild | `loop_runtime_control.rs` | Hard-fails “legacy global plan approval has been removed” → OBSOLETE |
| phase-aware rebuild | `rebuild_tools_for_active_phase` | Still passes `plan_phase=None` → OPEN residual |
| empty TODO verify | `verifier.rs` `total == 0` → Pass | OPEN residual |
| plan_submit registration | `tool_assembly.rs` | No legacy plan_create/submit registration; execution todos only → SUPERSEDED |
| content reopen | `store.rs` tests `content_edit_reopens_*` | Present → CLOSED |

## Doc validation

- `git diff --check` on staged documentation paths (run at commit time).
- Confirm main `TODOLIST.md` contains no full 2026-07-11 P1/P2/P3 review body.
- Confirm archive index links resolve under `docs/archive/todolist/`.

## Not run (N/A)

- `just fmt-check` / `just check` / `just test` — no Rust/GUI source edits.
