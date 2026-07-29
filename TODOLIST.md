# TODOLIST

项目级待办、延期项、评审计划与已完成事项记录。

## Active Plan

No active delivery plan is selected. Sandbox command approval Phases 1-3 are complete; backend, GUI approval, and persistent validated command-rule evidence are recorded under `docs/logs/2026-07-sandbox-command-approval/`.

## Superseded Review Evidence

The following Plan/TODO P1–P3 review packets are retained for audit only. Their 2026-07-11 tool-oriented baseline was superseded by the revision-bound report/store/runtime implementation completed in commits `82c25856`, `a1c1389e`, `91be604b`, and `52db71ac`. They are not active backlog entries; any regression against the current architecture must be recorded as a new, reproducible TODO.

### Plan/TODO P1 core policy — historical triple-review findings (2026-07-11)

Source reviews (forced packet process):

- `_bmad-output/implementation-artifacts/review-result-todo-p1-blind-hunter.md` (verdict: approve-with-nits; 0 bug / 3 suggestion / 1 nit)
- `_bmad-output/implementation-artifacts/review-result-todo-p1-edge-case-hunter.md` (verdict: conditional / request-changes; 2 bug / 5 suggestion / 1 nit)
- `_bmad-output/implementation-artifacts/review-result-todo-p1-acceptance-auditor.md` (verdict: partial; 1 bug / 0 suggestion / 0 nit)

Related implementation: `agent-diva-core/src/planning/policy.rs`, `agent-diva-core/src/planning/mod.rs`, compatibility target `agent-diva-agent/src/planning/orchestrator.rs`, frozen spec `_bmad-output/implementation-artifacts/spec-plan-todo-p1-core-policy.md`.

**P0 — merge blockers / AC deviations**

- [ ] **P1 policy: Executing must fail-closed for future ToolCapability variants (AC5)** Acceptance auditor bug. `(PlanModeState::Executing, _) => true` open-by-default permits any new enum arm that is not yet matrix-listed. Expected: explicit operational allowlist + unified `_ => false` so “no matrix entry ⇒ denied”.
  - Related: `agent-diva-core/src/planning/policy.rs` (~65-66), AC5 / I/O matrix “unknown capability denied”
  - Suggested fix: allow only Inspect|PlanningRecord|WorkItem|WorkspaceWrite|Execute|External under Executing; keep Unknown denied; optionally assert exhaustiveness in tests.

- [ ] **P1 policy: Verifying capability set vs legal Verify exits (lifecycle trap)** Edge-case hunter bug. Legal edges include `Verify → Completed|Partial|Failed`, but `allows(Verifying, …)` only permits Inspect|Execute; PlanningRecord/WorkItem are denied. If P3 gates `plan_transition` / todo tools solely via `allows`, plans can stick in Verify.
  - Related: `agent-diva-core/src/planning/policy.rs` (~67-87), `agent-diva-agent/src/planning/tools.rs` (PlanTransitionTool)
  - Suggested fix: either extend Verifying allows with PlanningRecord (and WorkItem if todos update during verify), or document/enforce that phase exits use a privileged path not subject to the capability matrix; add a test that every legal edge has a capability path for the tool that performs it.

**P1 — correctness / single source of truth**

- [ ] **P1 policy: dual transition matrix vs PlanOrchestrator not cross-checked** Edge-case hunter bug. Core `is_valid_transition` duplicates orchestrator edges but is not delegated from `PlanOrchestrator::is_valid_transition`; unit tests hardcode a local allow-list only. Later one-sided edits will desync pure policy from runtime.
  - Related: `agent-diva-core/src/planning/policy.rs` (~73-87), `agent-diva-agent/src/planning/orchestrator.rs` (~117-133)
  - Suggested fix: orchestrator calls core (or share one table); add full `PlanPhase` cartesian equality test between both functions.

- [ ] **P1 policy: expand invalid-transition / terminal-reentry test coverage** Blind + edge suggestion. Happy-path + any→Failed + single invalid pair leave reverse edges, self-transitions, skips, and terminal re-entry unenforced.
  - Related: `agent-diva-core/src/planning/policy.rs` tests (~169-210)
  - Suggested fix: table-drive full 8×8 product (or all illegal edges); assert `!is_valid_transition` and typed `InvalidTransition` errors.

- [ ] **P1 policy: document or tighten Failed bailout / identity edges** Blind + edge suggestion. `to == Failed` accepts `Completed|Partial|Failed → Failed` (including self-loop) while other identity edges are rejected; capability side maps terminals to Closed and denies all caps—lifecycle vs capability disagree on “closed means done”.
  - Related: `agent-diva-core/src/planning/policy.rs` (~73-76, ~30, ~68)
  - Suggested fix: either restrict Failed bailout to non-terminal `from`, or keep emergency semantics with explicit docs + tests for terminal×targets.

**P2 — product policy / API ergonomics (confirm before P2/P3 contracts freeze)**

- [ ] **P1 policy: decide AwaitingApproval freeze for PlanningRecord** Blind + edge suggestion. Exploring/Drafting/AwaitingApproval all allow PlanningRecord; if approval freezes submitted revision, plan content can still change pre-`plan_approve`.
  - Related: `policy.rs` (~61-64), architecture acceptance/revision freeze language
  - Suggested fix: deny PlanningRecord under AwaitingApproval (Inspect only), or document revision-bound invalidation of approval on edit; lock with matrix tests.

- [ ] **P1 policy: Closed projection collapses Completed|Failed|Partial** Edge suggestion. Single `Closed` mode cannot authorize “Partial may Inspect residual work” without re-reading raw `PlanPhase`.
  - Related: `policy.rs` (~30), `From<&PlanPhase> for PlanModeState`
  - Suggested fix: authorize from `PlanPhase` overload, split terminal variants, or allow Inspect on Closed if product needs it.

- [ ] **P1 policy: Unknown-during-Execute vs architecture edge table** Edge suggestion. Module fail-closed denies Unknown even in Executing; `docs/architecture/plan-todo/06-error-handling-edge-cases.md` may describe unknown tools as Execute-class during Execute. Align docs and mapping-completeness tests.
  - Related: `policy.rs` (~65-66), `docs/architecture/plan-todo/06-error-handling-edge-cases.md`

- [ ] **P1 policy: API usability helpers** Edge nit. Only `From<&PlanPhase>`; no `allows_for_phase`; easy stale mode cache after transition.
  - Related: `policy.rs` (~22-32, ~59)
  - Suggested fix: `impl From<PlanPhase>`, `allows_for_phase(phase, cap)`.

- [ ] **P1 policy: simplify Executing match arms** Blind nit. Dedicated `(Executing, Unknown) => false` is redundant once allowlist/fail-closed rewrite lands.
  - Related: `policy.rs` (~65-66)

### Plan/TODO P2 approval materialization — historical triple-review findings (2026-07-11)

Source reviews (independent sessions; forced packet process):

- `_bmad-output/implementation-artifacts/review-result-todo-p2-blind-hunter.md` (verdict: request-changes; 8 bug / 3 suggestion / 2 nit)
- `_bmad-output/implementation-artifacts/review-result-todo-p2-edge-case-hunter.md` (verdict: not ready; 9 bug / 3 suggestion / 2 nit)
- `_bmad-output/implementation-artifacts/review-result-todo-p2-acceptance-auditor.md` (verdict: fails-spec; 8 bug / 2 suggestion / 2 nit)

Baseline: `4f3168e`. Related implementation (working tree at review time): `agent-diva-core/src/planning/{approval,store,policy,mod}.rs`, `agent-diva-agent/src/planning/{orchestrator,tools}.rs`, `agent-diva-agent/src/{runtime_control,agent_loop/loop_runtime_control}.rs`, `agent-diva-tools/src/planning/mod.rs`, `agent-diva-manager/src/{handlers/planning,manager,state}.rs`. Spec: `_bmad-output/implementation-artifacts/spec-plan-todo-p2-approval-materialization.md`.

**P0 — merge blockers / fails-spec**

- [ ] **P2: register `plan_submit` and allow it in plan mode** Blind + edge + acceptance. `PlanSubmitTool` exists but is not registered in `tool_assembly.rs`; `is_plan_mode_allowed_tool` also omits `plan_submit`. Completeness + revision freeze never run on the live agent path.
  - Related: `agent-diva-agent/src/tool_assembly.rs` (~280-292), `agent-diva-agent/src/agent_loop/loop_turn.rs` (~49-60), `agent-diva-tools/src/planning/mod.rs`
  - Suggested fix: register `PlanSubmitTool`; add `"plan_submit"` to plan-mode allowlist; optionally expose manager submit API for GUI parity.

- [ ] **P2: close dual lifecycle path into AwaitingApproval/Execute** Blind + edge + acceptance. `plan_transition` can still move `Plan → AwaitingApproval` without `submit_plan` (no completeness, no `plan_submissions` revision) while emitting ready-for-approval; `store.approve_plan` is a second authority vs orchestrator memory gate.
  - Related: `agent-diva-agent/src/planning/tools.rs` (~126-139), `orchestrator.rs`, `store.rs` approve/submit, `loop_runtime_control.rs`
  - Suggested fix: only `submit_plan`/`approve_plan` write AwaitingApproval/Execute; narrow or block agent transitions into those phases; single source of truth for approval (store CAS).

- [ ] **P2: freeze content or reopen-on-edit (revision++ → Plan)** Acceptance AC failed; blind + edge. After submit, `update_plan`/steps can change body without bumping revision; `approve_plan` only CASes revision number + phase. Edit-after-submit/approve (revision++, back to Plan, old rev dead) is not implemented; re-submit from AwaitingApproval is blocked.
  - Related: `agent-diva-core/src/planning/store.rs` (submit/approve/update_plan/steps), AC “Scope edit” / frozen revision
  - Suggested fix: reject mutations under AwaitingApproval (or auto-reopen: revision++, phase→Plan); allow controlled re-submit/withdraw; ensure old `expected_revision` cannot authorize.

- [ ] **P2: enforce AwaitingApproval inspect-only at store/tool dispatch** Edge + acceptance. Policy matrix is inspect-only but nothing calls `allows_for_phase` on the hot path; agents can still mutate plans/todos while awaiting approval.
  - Related: `policy.rs`, tool dispatch / mutating store methods
  - Suggested fix: gate mutating planning tools and store writes by phase; freeze or reopen as above.

- [ ] **P2: approve API compatibility + surface revision** Blind. `POST .../approve-execute` now requires `Json<ApprovalRequest>`; GUI still empty-POST; `PlanRuntimeState` has no revision field for clients to supply `expected_revision`.
  - Related: `agent-diva-manager/src/handlers/planning.rs` (~224-227), GUI approve client, `PlanRuntimeState` / snapshot APIs
  - Suggested fix: temporary body-less compat or update GUI+snapshot together; return revision (and ideally receipt) on approve/list.

**P1 — correctness / audit / materialization**

- [ ] **P2: Always/Optional materialize vs pre-existing TODOs** Blind + edge. Non-empty `todo_items` + materialize aborts with `TodoAlreadyMaterialized` forever; draft `todo_write` is still allowed pre-approve. No silent overwrite (good) but Always path can be permanently stuck.
  - Related: `store.rs` approve materialize (~736-747), `todo_write` / `replace_todos`
  - Suggested fix: treat pre-existing as already-materialized success under explicit policy, or forbid work-item writes until Execute, or clear API for pre-approve cleanup.

- [ ] **P2: approval/submit audit events visible and typed** Blind + edge + acceptance. `event_type = 'PlanApproval'` is filtered out by `get_events` (`LIKE 'PlanEvent::%'`); submit/approve omit `PhaseTransition`/`StatusChanged`/`TodoGenerated`.
  - Related: `store.rs` get_events / approve_plan / submit_plan, `events.rs`
  - Suggested fix: emit first-class `PlanEvent` variants; include phase + receipt in event stream or document `plan_approvals` as sole audit API and stop orphan rows.

- [ ] **P2: collapse dual transition authority (orchestrator vs policy)** Acceptance + blind nit/edge. `PlanOrchestrator::is_valid_transition` still diverges (any→Failed including terminals) while `transition_to` uses policy; public dead/divergent API.
  - Related: `orchestrator.rs` (~116-134), `policy.rs`
  - Suggested fix: delete or thin-wrap to `policy::is_valid_transition` only.

- [ ] **P2: submit_plan phase CAS must not skip Plan phase** Blind. Conditional update allows Explore|Plan → AwaitingApproval; shared policy only documents Plan → AwaitingApproval.
  - Related: `store.rs` submit_plan (~671-679), `policy.rs` transitions
  - Suggested fix: restrict to `phase = Plan` (or validate_transition first).

- [ ] **P2: AC matrix tests incomplete** Acceptance. Missing tests for incomplete multi-field submit, Always/Optional+true one-TODO-per-step, stale approve snapshot equality, edit-after-submit, existing-TODO materialize reject without side effects.
  - Related: `agent-diva-core` planning tests; manager/agent transport tests
  - Suggested fix: table-drive frozen I/O matrix from `spec-plan-todo-p2-approval-materialization.md`.

- [ ] **P2: stop silent full-replace of TODOs while materialization invariants apply** Acceptance. `todo_write`/`replace_todos` can still wipe lists; contradicts Always “never silently delete/overwrite” for execution lists.
  - Related: `agent-diva-tools/src/planning/mod.rs`, store replace path
  - Suggested fix: refuse full-delete when materialized execution list exists; prefer revision-bearing patch APIs.

- [ ] **P2: empty TODO Execute→Verify/Completed gate** Edge. Never/Optional without materialize leaves zero todos; verify gate passes on empty lists.
  - Related: orchestrator verify gate, todo policy product intent
  - Suggested fix: force materialize when steps exist, or explicit plan-only execution policy in gates.

**P2 — nits / ergonomics**

- [ ] **P2: wrong-phase submit surfaces as `ApprovalConflict { expected_revision: 0 }`** Mislabels phase/state errors as revision conflicts.
  - Related: `store.rs` (~681-687)
  - Suggested fix: dedicated `InvalidPhase` / `NotSubmittable` with current phase.

- [ ] **P2: persist `todo_policy` with stable string/serde, not `Debug`**
  - Related: `store.rs` (~769)

- [ ] **P2: unregister or reword dead `plan_approve` tool** Still registered with approve description but hard-fails (Never self-approve).
  - Related: `tools.rs`, `tool_assembly.rs`

- [ ] **P2: return ApprovalReceipt on approve-execute transport** Tasks say snapshot/receipt; response is still `{ status, plan }` only.
  - Related: `loop_runtime_control.rs`, manager planning handler

### Plan/TODO P3 runtime capability gate — historical triple-review findings (2026-07-11)

Source reviews (independent sessions; forced packet process):

- `_bmad-output/implementation-artifacts/review-result-todo-p3-blind-hunter.md` (verdict: request-changes; 2 bug / 2 suggestion / 1 nit)
- `_bmad-output/implementation-artifacts/review-result-todo-p3-edge-case-hunter.md` (verdict: not-ready; 2 bug / 2 suggestion / 2 nit)
- `_bmad-output/implementation-artifacts/review-result-todo-p3-acceptance-auditor.md` (verdict: fails-spec; 2 bug / 5 missing-verification / 1 quality)

Baseline: `a0e80ba`. Related implementation (working tree at review time): `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`, `agent-diva-agent/src/agent_loop/loop_tools.rs`, `agent-diva-agent/src/planning/{mod,orchestrator,verifier}.rs`, `agent-diva-agent/src/tool_assembly.rs`, `agent-diva-core/src/planning/policy.rs`. Spec: `_bmad-output/implementation-artifacts/spec-plan-todo-p3-runtime-gate.md`. Review packets: `review-plan-todo-p3-{blind-hunter,edge-case-hunter,acceptance-auditor}.md`.

**P0 — merge blockers / fails-spec**

- [ ] **P3: terminal active plan must not deny all tools (Closed lockdown)** Blind + edge + acceptance. Any active plan phase becomes `policy_phase`, including `Completed`/`Failed`/`Partial` → `PlanModeState::Closed`, which denies **all** capabilities (including Inspect / `plan_create`). Active slot is not cleared when no non-terminal successor exists (`promote_next_active_plan` / orchestrator terminal transition). Pre-P3 only guarded `plan_mode || AwaitingApproval`.
  - Related: `agent-diva-agent/src/agent_loop/loop_turn.rs` (~290-295, ~1012-1023), `agent-diva-core/src/planning/policy.rs` (~30-37, ~64-85), `agent-diva-agent/src/planning/verifier.rs` (~147-160)
  - Suggested fix: treat terminal/`Closed` as `policy_phase = None`, or `clear_active_plan` on terminal entry when no successor; apply same helper at turn start and mid-turn rebuild; add regression test (active Completed + ordinary message still exposes normal tools).

- [ ] **P3: rebuild registry after runtime ApproveActivePlan → Execute** Edge + acceptance (AC2/AC3). `handle_approve_active_plan` persists Execute but never calls `rebuild_tools_for_turn`. Mid-turn approval leaves inspect-only registry while invoke checks may already allow Execute tools that are not registered.
  - Related: `agent-diva-agent/src/agent_loop/loop_runtime_control.rs` (~339-362), `loop_turn.rs` (~553-554, ~861, ~1012-1023), store approve path
  - Suggested fix: rebuild with `Some(PlanPhase::Execute)` after successful approve (or on any drain-detected phase change); integration test approve → next iteration Execute tools visible.

- [ ] **P3: mid-turn rebuild must use same policy_phase derivation as turn start** Blind. Post-tool rebuild passes raw `planning_after.phase` only — drops `plan_mode` synthetic `Plan` fallback and re-applies terminal freeze.
  - Related: `loop_turn.rs` (~292-294 vs ~1019-1022, ~904-909)
  - Suggested fix: shared `policy_phase_for(active_snapshot, plan_mode)` used at turn start, mid-turn rebuild, and invoke gate.

**P1 — verification / AC proof (missing-verification)**

- [ ] **P3: agent-loop integration fixture for AC1/AC2** Acceptance Tasks failed. No fixture proves: plan-mode + ordinary follow-up under AwaitingApproval deny write/exec/MCP/custom/todo; denied call has no persistence side effects; submit → Inspect-only next iteration; matching runtime-control approve → Execute next iteration.
  - Related: `agent-diva-agent/tests/` (missing), `loop_turn.rs` / `tool_assembly.rs` unit coverage only
  - Suggested fix: preload AwaitingApproval plan; assert registry + invoke denial + unchanged revision/events; cover submit/approve refresh paths.

- [ ] **P3: iteration log at required path** Acceptance Tasks failed. Spec requires `docs/logs/2026-07-runtime-plan-gate/v0.0.1-runtime-gate-closure/` with summary/verification/release/acceptance; directory absent (existing P2/P3 mixed log does not satisfy path).
  - Related: `_bmad-output/implementation-artifacts/spec-plan-todo-p3-runtime-gate.md` Tasks
  - Suggested fix: create four required docs; record command outputs and AC matrix.

- [ ] **P3: full phase×tool I/O matrix assertions in assembly tests** Acceptance. Current test only checks subset (`read_file`/`write_file`/`exec`/`plan_submit`/`todo_write`/`custom_tool`) and omits terminal phases and web/spawn/cron/enqueue/plan_create/edit_file.
  - Related: `agent-diva-agent/src/tool_assembly.rs` (~516-541)
  - Suggested fix: table-drive allow/deny per phase matching frozen I/O matrix; include Completed/Failed/Partial expectations after P0 terminal rule is chosen.

- [ ] **P3: prove denied invoke has no side effects** Acceptance AC1. Policy denial returns error string but no test asserts denial happens before `tools.execute` and leaves plan revision/todos/events unchanged.
  - Related: `loop_turn.rs` (~919-923)
  - Suggested fix: unit/integration assert store snapshot equality on denied call.

- [ ] **P3: record verification command evidence** Acceptance Verification. Required `cargo test -p agent-diva-core planning::policy`, agent planning/tool_assembly/loop_turn tests, and `just fmt-check && just check && just test` lack recorded evidence for this wave.
  - Related: spec Verification; iteration `verification.md`
  - Suggested fix: run and log results; separate pre-existing failures into TODOLIST.

**P2 — product edges / defense-in-depth**

- [ ] **P3: document or tighten plan_mode vs active-plan precedence** Blind + edge suggestion. Priority flipped from plan_mode-first to active-plan-first; `exec_mode=plan` while active Execute still allows mutations.
  - Related: `loop_turn.rs` (~292-294, ~897-899)
  - Suggested fix: document "persisted phase wins", or intersect with Plan when plan_mode is set if product wants a hard non-mutating switch.

- [ ] **P3: runtime MCP/network/custom re-register must re-apply phase filter** Edge suggestion. `drain_runtime_control_commands` can re-inject tools without phase unregister; invoke still fail-closes Unknown, but registry is not a true capability boundary until full rebuild.
  - Related: `loop_tools.rs` (~28-64), `tool_assembly.rs` (~264-325), `loop_turn.rs` (~861)
  - Suggested fix: route tool-config updates through `rebuild_tools_for_turn(..., current_policy_phase, ...)`.

- [ ] **P3: align plan-guard system prompt with WorkItem policy** Edge nit. Prompt mentions `todo_write` under plan guard, but WorkItem is only allowed in Execute — rejected under synthetic Plan.
  - Related: `loop_turn.rs` (~476-479), `policy.rs` WorkItem matrix
  - Suggested fix: prompt only Inspect/PlanningRecord tools in draft/await, or product decision to allow draft WorkItem.

- [ ] **P3: delete dead commented orchestrator transition matrix** Blind nit + acceptance quality. Core delegation is correct; large commented former matrix remains.
  - Related: `agent-diva-agent/src/planning/orchestrator.rs` (~117-137)
  - Suggested fix: delete comment block; keep single-line core delegate + docs.

## Deferred (previously Open)

- [ ] **Mentle: repair runtime prompt activation regressions** After Windows native-open isolation, `cargo test -p agent-diva-agent --features mentle --lib mentle` is mostly green (31 pass). Remaining failure: `test_register_default_tools_rebuild_keeps_active_mentle_prompt` — runtime is active / tools register, but system prompt still lacks `L2 Palace Memory` after tool rebuild. Investigate the Mentle runtime/context boundary before treating the full Mentle lane as green.
  - Related files: `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/agent_loop/loop_tools.rs`

- [ ] **Provider: run StepFun real endpoint E2E for model pass-through** The runtime now keeps model IDs opaque and unit coverage verifies `provider_name = stepfun`, `api_base = https://api.stepfun.com/step_plan/v1`, and `model = step-3.7-flash` pass through unchanged. Real StepFun E2E could not be run in this checkout because `keys.txt` is absent and no StepFun API key is available in the environment.
  - Related files: `agent-diva-providers/src/openai_compatible.rs`, `agent-diva-e2e/src/config.rs`, `docs/logs/2026-07-provider-model-pass-through/v0.0.1-provider-model-pass-through/verification.md`
  - Suggested validation: set `E2E_PROVIDER_NAME=stepfun`, `E2E_API_BASE=https://api.stepfun.com/step_plan/v1`, `E2E_MODEL=step-3.7-flash`, and `E2E_API_KEY`/`DEEPSEEK_API_KEY` to a StepFun key, then run `just e2e-test`.
## Deferred (existing backlog)

- [ ] **Mask feature plan acceptance** Deferred while the Plan/TODO architecture is the sole active stream. The remaining acceptance items in `.sisyphus/plans/mask-feature-implementation.md` are preserved for later reactivation.

- [ ] **Memory: publish a current-baseline interfaces spec after the `vrm-memory-test` audit** Deferred. The `origin/vrm-memory-test` branch does not contain an `agent-diva-memory` crate, but it does contain still-useful design intent around diary domain boundaries, future recall slots, and diary tool contracts. The current mainline preserves that intent only indirectly across legacy docs and evolved runtime code, so a fresh spec is needed to map those ideas onto today's `MemoryProvider` / `MemoryManager` / `memory_boundary` / Laputa-Mentle architecture without reviving a nonexistent crate.
  - Related files: `agent-diva-core/src/memory/`, `agent-diva-agent/src/memory_boundary.rs`, `docs/dev/past/legacy-docs/dev/archive/memory-evolution/`, `docs/logs/2026-07-vrm-memory-audit/v0.0.1-vrm-memory-test-audit/summary.md`
  - Suggested fix: write a dedicated `Memory Framework Interfaces Spec` on top of the current `agent-diva-pro` baseline, then split any real implementation work into separate stories such as diary-domain formalization or future-recall contracts.
- [ ] **GUI: migrate `lucide-vue-next` to `@lucide/vue`** Deferred. `lucide-vue-next@0.575.0` is deprecated; npm install warns to use `@lucide/vue` instead. Migration touches ~71 Vue/TS files that import from `lucide-vue-next`, so it needs a dedicated pass and import-name verification.
  - Related files: `agent-diva-gui/src/**/*.vue`, `agent-diva-gui/src/**/*.ts`, `agent-diva-gui/package.json`, `agent-diva-gui/pnpm-lock.yaml`
- [ ] **Wave 3 residual: JsonlTodoStore concurrent rewrite data loss** Deferred. `create/update/archive` share one JSONL file but `update_status()` and `archive_completed()` still do read-then-truncate rewrites without mutual exclusion, so concurrent writes can drop freshly appended or updated todos.
  - Related files: `agent-diva-core/src/todo/store.rs`, `agent-diva-manager/src/handlers/todo.rs`, `agent-diva-cli/src/commands/todo.rs`
- [ ] **Wave 3 residual: todo API/CLI status contract drift** Deferred. `open/pending/active/done/completed` semantics are inconsistent across CLI help, CLI parsing, API list filtering, and API patch validation.
  - Related files: `agent-diva-core/src/todo/types.rs`, `agent-diva-manager/src/handlers/todo.rs`, `agent-diva-cli/src/commands/todo.rs`
- [ ] **Wave 3 residual: todo API error semantics mismatch** Deferred. `GET /api/todos` still returns HTTP 200 on store failure and accepts invalid `status` filters by silently returning all todos.
  - Related files: `agent-diva-manager/src/handlers/todo.rs`
- [ ] **Wave 3 residual: enqueue_background_task production wiring is dead** Deferred. The builtin tool requires `ToolAssembly::with_run_store(...)`, but no production `AgentLoop` construction path injects a `RunStore`, so the tool never appears in real registries.
  - Related files: `agent-diva-agent/src/tool_assembly.rs`, `agent-diva-agent/src/agent_loop.rs`, `agent-diva-tools/src/enqueue_background_task.rs`
- [ ] **Wave 3 residual: supervised subagent worker is not bootstrapped** Deferred. `SubagentRunHandler` and `TaskExecutor` exist, but no reviewed runtime startup path registers `RunKind::Subagent` and drains queued supervised runs in production.
  - Related files: `agent-diva-agent/src/subagent_run_handler.rs`, `agent-diva-core/src/supervised/executor.rs`, `agent-diva-manager/src/runtime/`
- [ ] **Wave 3 residual: supervised subagent run closes before real work finishes** Deferred. `SubagentRunHandler` marks the supervised run completed once `SubagentManager::spawn()` returns, even though the detached subagent task is still running and can later fail or time out.
  - Related files: `agent-diva-agent/src/subagent_run_handler.rs`, `agent-diva-agent/src/subagent.rs`, `agent-diva-core/src/supervised/executor.rs`
- [ ] **Wave 3 residual: background task context and budget inheritance is incomplete** Deferred. Enqueued tasks do not persist `chat_id/session_key/trace_id`, default their reply route to `supervised`, and do not inherit token-ledger semantics from the parent session.
  - Related files: `agent-diva-tools/src/enqueue_background_task.rs`, `agent-diva-agent/src/subagent_run_handler.rs`, `agent-diva-agent/src/subagent.rs`, `agent-diva-agent/src/agent_loop/loop_turn.rs`
- [ ] **Wave 3 residual: workspace CLI managed-path model drift** Deferred. The path traversal/delete guard issues are now closed, but the managed `config_dir/workspaces/*` model still diverges from the broader runtime support for arbitrary workspace paths and needs an explicit product contract.
  - Related files: `agent-diva-cli/src/commands/workspace.rs`, `agent-diva-cli/src/cli_runtime.rs`, `agent-diva-cli/src/main.rs`
- [ ] **UX-DR-3/4/7** Deferred. UX gaps from sprint review remain postponed until a dedicated design pass.
  - Context: Sprint closure review items 3, 4, and 7

## Deferred Review Program

### Scope

- Range: `94baa4b..HEAD`
- Exclude: `docs:*`, research/archive-only changes, `cbab379`, `ba1d17b`
- Include: `feat:*`, `merge:*`, behavior-relevant `refactor:*`, validation-relevant `chore:*`, and `style:*` only for semantic-risk sampling

### Wave Plan

- [x] **Wave A - 基础设施与 Harness 基线**
  - Commits: `e3cd30c`, `3322aac`, `1627ea3`, `0183c3c`
  - Focus: `ToolRegistry::execute()` 调用链迁移、`PokeEvent` 广播安全、Harness 与真实运行语义一致性、E2E 迁移后的断言与环境假设
- [x] **Wave B - 安全 / 监督运行 / 预算治理**
  - Commits: `45b6aa6`, `9242579`, `ce70902`, `7ca1c92`, `f43ff96`, `0c1d1bd`, `8ecf041`
  - Focus: Token ledger 记账时机、subagent budget 强制生效、`UsageRecord`/`ContextBudgetPolicy`/`OverflowAction` 落点、安全收口默认行为、merge 带来的逻辑分叉
- [x] **Wave C - 观测性 / Audit 主线**
  - Commits: `c58054c`, `28b00ce`, `41c829a`, `bd96dd2`, `de031fa`, `0851c4c`, `6ac4056`, `166fd16`, `b3fd4e8`
  - Focus: Audit schema 一致性、`GLOBAL_SINK` 初始化和线程安全、JSONL rolling 并发写、`/api/logs` 查询闭环、parser refactor 兼容性
- [x] **Wave D - 上下文压缩 / 限流**
  - Commits: `0eb0cce`, `e2e2ad3`
  - Focus: TokenBucket refill/burst 边界、MetaCompactor 语义保持、触发顺序、失败路径稳定性
- [x] **Wave E - Todo 数据面**
  - Commits: `909a573`, `84c5803`, `2445984`
  - Focus: API/CLI/store 状态机一致、archive/purge 误删风险、并发写入与重复 ID 边界
- [x] **Wave F - 后台任务 / 子代理 / workspace CLI**
  - Commits: `c0f2712`, `c705538`, `b633b0d`
  - Focus: background task 生命周期、`SubagentRunHandler` 覆盖度、预算/权限/审计继承、workspace 路径隔离
- [x] **Wave G - 横切补强**
  - Commits: `11728fa`, `9438b25`, `48dd875`, `e9336d9`, `e2941a8`
  - Focus: usage fallback 统计准确性、`ErrorCategory` 分类失真、timeout wrapper 语义变化、feature gate 漏检、rustfmt 大提交夹带逻辑改动

### Execution Order

- [x] **Priority 1** Review `Wave B`
- [x] **Priority 2** Review `Wave C`
- [x] **Priority 3** Review `Wave F`
- [x] **Priority 4** Review `Wave E`
- [x] **Priority 5** Review `Wave D`
- [x] **Priority 6** Review `Wave A`
- [x] **Priority 7** Review `Wave G`

### Wave 1 Parallel Review

- [x] **Wave 1 scope lock**
  - Coverage: `Wave A` and `Priority 1` (`Wave B`)
  - Goal: launch parallel review for the foundation path and the first release-risk path together
  - Output: one `Wave A` report, one `Wave B` report, and one cross-wave risk matrix
- [x] **Lead-Agent**
  - Responsibilities: freeze commit map, assign tasks, deduplicate findings, arbitrate overlaps, publish final summary
  - Required outputs: commit-to-owner table, merged findings list, release/block recommendation
- [x] **A1-Infrastructure**
  - Commits: `e3cd30c`, `3322aac`
  - Focus: `PokeEvent` fan-out, subscriber lifecycle, backpressure, `ToolRegistry::execute()` error propagation and caller migration completeness
- [x] **A2-Harness**
  - Commits: `1627ea3`, `0183c3c`
  - Focus: Harness V2 entrypoints, fixture realism, CI baseline assumptions, E2E migration assertion drift and environment contract changes
- [x] **B1-Budget**
  - Commits: `45b6aa6`, `9242579`, `7ca1c92`
  - Focus: token ledger write timing, budget enforcement on hot paths, subagent per-task budget coverage, policy definitions vs real execution
- [x] **B2-SupervisedRun**
  - Commits: `ce70902`
  - Focus: supervised-run state transitions, persistence, cancellation ownership, budget/audit/security wiring
- [x] **B3-SecurityMerge**
  - Commits: `f43ff96`, `0c1d1bd`, `8ecf041`
  - Focus: `SecurityDecision`, `check_security()`, audit emissions, default policy semantics, merge regression risk, high-risk drift sampling
- [x] **Wave 1 execution flow**
  - Phase 0: `Lead-Agent` publishes commit map and review packet
  - Phase 1: `A1`, `A2`, `B1`, `B2`, `B3` review in parallel
  - Phase 2: `Lead-Agent` runs cross-checks for budget/security, harness/supervised-run, and error propagation gaps
  - Phase 3: `Lead-Agent` publishes `Wave A` and `Wave B` summaries plus one shared priority pool
- [x] **Wave 1 report template**
  - Required sections: `Scope`, `Findings`, `No-finding checks`, `Open questions`, `Verdict`
  - Required finding fields: severity, title, commit, files, why it matters, reasoning or repro, expected behavior, suggested fix direction, test gap
  - Verdict format: `Block release: yes|no`, `Confidence: high|medium|low`

### Standard Checklist

- [ ] **接口契约** Public types, traits, and APIs changed in the wave do not break downstream callers.
- [ ] **状态一致性** In-memory state, persisted state, and event state remain aligned.
- [ ] **并发/异步安全** Channels, background jobs, globals, locks, cancellation, and task ownership are safe.
- [ ] **错误传播** Errors preserve context and classification; no silent swallowing or lossy wrapping.
- [ ] **测试覆盖** Success paths, failure paths, boundaries, and regressions are covered.
- [ ] **集成闭环** Producer, storage, consumer, CLI/API/UI links are fully wired.

### Commit Checklist

- [x] **Wave A / `e3cd30c`** Verify `PokeEvent` broadcast fan-out, subscriber lifecycle, and backpressure assumptions.
- [x] **Wave A / `3322aac`** Verify every `ToolRegistry::execute()` caller correctly handles `Result<ToolError>`.
- [x] **Wave A / `1627ea3`** Review Harness V2 entrypoints, fixture realism, and CI baseline assumptions.
- [x] **Wave A / `0183c3c`** Review E2E migration for assertion drift and environment contract changes.
- [x] **Wave B / `45b6aa6`** Verify token ledger writes exactly once per billable usage and enforces budgets on the real hot path.
- [x] **Wave B / `9242579`** Verify per-task subagent budget is enforced, not merely logged.
- [x] **Wave B / `ce70902`** Review supervised-run MVP state transitions, persistence, and cancellation ownership.
- [x] **Wave B / `7ca1c92`** Verify usage source mapping and context budget policy integration points.
- [x] **Wave B / `f43ff96`** Review `SecurityDecision`, `check_security()`, audit emissions, and default-deny/default-allow semantics.
- [x] **Wave B / `0c1d1bd`** Audit merge integration for duplicate paths, stale gates, and branch-resolution regressions.
- [x] **Wave B / `8ecf041`** Sample high-risk files for accidental behavior drift in the pre-merge catch-up commit.
- [x] **Wave C / `c58054c`** Verify skill upload/delete/block audit events are emitted on all outcome paths.
- [x] **Wave C / `28b00ce`** Verify cron start/completion/failure events do not double-fire or miss failures.
- [x] **Wave C / `41c829a`** Review `ProviderTap` timing, streaming accumulation, and token accounting correctness.
- [x] **Wave C / `bd96dd2`** Review `ToolExecutionTap` around success/error/timeout and nested tool calls.
- [x] **Wave C / `de031fa`** Verify JSONL daily rolling, concurrent writes, and write-error swallowing behavior are intentional.
- [x] **Wave C / `0851c4c`** Review `AuditSink` registration, `GLOBAL_SINK` initialization ordering, and no-op fallback behavior.
- [x] **Wave C / `6ac4056`** Verify `/api/logs` query fields match persisted audit schema exactly.
- [x] **Wave C / `166fd16`** Review parser refactor for GUI/manager compatibility and malformed-line handling.
- [x] **Wave C / `b3fd4e8`** Verify health endpoint criteria and benchmark assumptions are stable and meaningful.
- [x] **Wave D / `0eb0cce`** Review token bucket refill math, monotonic-time assumptions, and burst depletion edges.
- [x] **Wave D / `e2e2ad3`** Review MetaCompactor summary fidelity, fallback behavior, and serialization compatibility.
- [x] **Wave E / `909a573`** Verify todo CRUD routes, filters, 404 paths, and store integration.
- [x] **Wave E / `84c5803`** Verify todo CLI behavior matches HTTP and store semantics.
- [x] **Wave E / `2445984`** Review archive/purge for active-item safety, historical retention, and concurrent update behavior.
- [x] **Wave F / `c0f2712`** Review enqueue background task lifetime, observability, cancellation, and error return path.
- [x] **Wave F / `c705538`** Verify `RunKind::Subagent` dispatch covers every subagent execution path.
- [x] **Wave F / `b633b0d`** Review workspace CLI path resolution, isolation, and failure output quality.
- [x] **Wave G / `11728fa`** Verify usage fallback metrics/warnings neither double-report nor mask real provider usage.
- [x] **Wave G / `9438b25`** Review `ErrorCategory` trait adoption and risk of over-generalized classification.
- [x] **Wave G / `48dd875`** Verify timeout wrapper preserves cancellation, retry, and tool-specific error identity.
- [x] **Wave G / `e9336d9`** Review feature-gate CI script for missing crate/feature combinations.
- [x] **Wave G / `e2941a8`** Sample rustfmt-only commit for accidental semantic edits in touched modules.

### Deliverables

- [ ] **Wave reports** Each wave should end with scope, risk summary, findings, and release/block recommendation.
- [ ] **Cross-wave matrix** Aggregate findings by security, observability, persistence, concurrency, CLI/API consistency, and test gaps.
- [ ] **Priority pool** Classify review findings into `P0`/`P1`/`P2`/`P3`.

### Timebox (Deferred)

- [ ] **Day 1** `Wave B` + `Wave C`
- [ ] **Day 2** `Wave F` + `Wave E`
- [ ] **Day 3** `Wave D` + `Wave A` + `Wave G`
- [ ] **Day 4** Cross-wave regression pass, unified conclusion, and priority-pool cleanup

## Active Governance Research Program

- [ ] **RG-CODE-GOV: Agent Loop / Manager / GUI 原位治理** 深度治理分支的 clean-break 产品替代路线已判定失败；仅保留其单一副作用权威链、薄 Manager、GUI Host 边界和 fail-closed 能力登记原则。后续治理必须以当前 `agent-diva-pro` 的 crate、`/api`、存储、事件和产品能力为唯一基线，采用先刻画行为、再按 seam 分阶段收口的方式，不得回迁 deep crates、另建第二套 runtime/API/store 或大爆炸重写。
  - 状态：研究/设计完成；实现未授权。
  - 第一实施入口：G0 characterization tests 与 capability ledger；未完成 G0 不得开始拆 AgentLoop。
  - 分期：G1 AgentLoop turn pipeline；G2 Manager handler/service；G3 GUI API/Tauri Host；G4 GUI state/composables；G5 DTO contract 与清理。
  - 预期结果：AgentLoop 唯一 turn 入口和工具执行 seam 可定位；Manager handler 变薄；GUI domain/Host/local state 权威清晰；既有 API、schema 和用户旅程保持兼容。
  - 退出标准：`docs/dev/agent-loop-manager-gui-governance/13-acceptance-criteria.md` 全部运行时条目通过，并有真实 CLI/Manager/Tauri smoke 与性能回归证据。
  - 相关设计：`docs/dev/agent-loop-manager-gui-governance/`
  - 本次文档日志：`docs/logs/2026-07-current-design-governance/v0.0.1-agent-loop-manager-gui-plan/`

## Governance × Memory × Human-in-the-loop 核心重构排期

> 目标周期：10 周；建议配置为 2 名 Rust 核心开发、1 名前端/桌面开发、1 名 QA/安全兼职。若只有 1 名核心开发，按依赖顺序执行并将周期调整为 14–16 周。所有时间均为净开发周，不包含等待产品决策或外部安全评审的时间。

### 总体目标与强制边界

- [ ] **GMH-00：冻结“一条权威链”架构约束** 所有 Memory 写入、策略变更、外部副作用和高风险自治行为必须统一经历“提议 → 策略判定 → 必要时人工决策 → 执行 → 审计 → 可恢复”的闭环。
  - 不新建第二套 runtime、approval store、memory authority store 或 Manager 业务权威。
  - `agent-diva-core` 持有跨 crate 契约；`agent-diva-laputa` 持有已应用 Memory/Persona 权威；`agent-diva-sandbox` 持有执行策略；`agent-diva-agent` 只编排；`agent-diva-manager` 只提供服务/传输；GUI 只投影状态并提交人类决定。
  - 未识别的能力、风险类型、审批状态和 Memory 来源一律 fail-closed；待审提案不得进入默认 prompt。
  - 完成定义：ADR、术语表、能力/风险矩阵、数据所有权图和兼容性清单评审通过。

### Phase 0 — 基线、决策与测试护栏（第 1 周，必须串行）

- [ ] **GMH-01：现状行为刻画与权威清单（D1–D2）**
  - 盘点 `MemoryProvider`、`MemoryManager`、Laputa proposal/apply、Mentle feature lane、AutoDream、Plan approval、Sandbox Guardian、Manager API/SSE/Tauri、GUI Persona/Memory 页面。
  - 输出读路径、写路径、审批路径、事件路径、重启恢复路径；标出重复状态、绕过点、隐式副作用和缺失审计。
  - 建立 capability ledger：能力名称、风险等级、资源范围、幂等性、可撤销性、默认决策、所需证据、审批 TTL。
  - 验收：每个生产副作用入口均能映射到唯一 owner 和唯一 policy decision point。
- [ ] **GMH-02：产品决策冻结（D2–D3）**
  - 明确 Memory 分类：会话事实、长期事实、偏好、承诺、关系、身份、历史摘要、临时 recall；定义保留期、敏感级别和可遗忘语义。
  - 明确 HITL 决策：`approve_once`、`approve_session`、`approve_rule`、`edit_and_approve`、`reject`、`cancel`；定义谁可批、作用域、过期、撤销和拒绝后的行为。
  - 明确自治等级 L0–L4：只读建议、低风险自动执行、会话授权执行、逐次审批、高风险禁止；将 Memory 写入和工具调用分别映射。
  - 验收：无“实现时再决定”的 P0/P1 语义；未决项有 owner 和截止日。
- [ ] **GMH-03：characterization 与契约测试（D3–D5）**
  - 锁定当前 CLI/Manager/Tauri API、事件顺序、Laputa 已应用快照、pending proposal 排除、Plan revision-bound approval、Sandbox deny/approve/retry 行为。
  - 增加崩溃/重启、重复提交、过期审批、并发审批、撤销、旧配置迁移的测试设计。
  - Gate G0：测试能证明当前行为；失败用例先记录而非在本阶段顺手重构。

### Phase 1 — 统一治理契约与决策引擎（第 2–3 周）

- [ ] **GMH-10：核心治理领域模型（W2 D1–D3）**
  - 在 `agent-diva-core` 定义稳定的 `GovernanceSubject`、`Capability`、`ResourceScope`、`RiskClass`、`Decision`、`ApprovalRequest/Receipt`、`EvidenceRef`、`AuditCorrelation`。
  - 统一 Plan approval、Sandbox approval 与 Memory proposal 的公共信封，但保留各自领域 payload；禁止做“万能大枚举”耦合业务。
  - 所有请求包含 `request_id`、`turn_id/session_id`、actor、目标资源、内容摘要/哈希、策略版本和到期时间。
- [ ] **GMH-11：纯函数策略评估器（W2 D3–W3 D2）**
  - 输入主体、能力、资源、风险、上下文和已有授权；输出 allow/deny/require-human，附 reason code、约束和可审计证据。
  - 规则优先级：硬禁止 > 显式用户拒绝 > 资源/模式限制 > 有效授权 > 安全默认值；未知项拒绝。
  - 为 Plan、Memory、shell/filesystem/network/MCP/spawn/schedule 建矩阵和全笛卡尔/属性测试。
- [ ] **GMH-12：持久化审批账本与状态机（W3 D2–D5）**
  - 建立 append-only decision ledger；派生当前状态，禁止 Manager/GUI 维护第二份真相。
  - 实现 CAS/version、TTL、幂等键、内容哈希绑定、审批后内容变更失效、并发首胜、拒绝/撤销优先。
  - Gate G1：领域模型、策略矩阵、迁移与账本恢复测试通过；尚未接入生产执行。

### Phase 2 — Memory Framework 2.0（第 4–6 周）

- [ ] **GMH-20：发布当前基线 Memory Interfaces Spec（W4 D1–D2）**
  - 完成现有 backlog 中 `vrm-memory-test` 后续规格，将 `MemoryProvider` 生命周期拆清为 startup injection、prefetch/recall、turn sync、session end、proposal submission。
  - 规定 provider 读能力与 authority 写能力分离；Laputa applied sections 是长期权威，Mentle/索引只做检索层，不得反向覆盖权威。
- [ ] **GMH-21：规范化 Memory 记录与 provenance（W4 D2–W5 D1）**
  - 定义记录 ID、类型、内容、来源、证据、置信度、敏感级别、创建/有效/过期时间、supersedes/tombstone、租户/会话范围。
  - 兼容旧 `MEMORY.md`/`HISTORY.md` 和 Laputa JSON；设计双读校验、一次性迁移、回滚和数据完整性报告。
  - 对 prompt injection 内容做信任标注和转义；用户输入、工具结果、AutoDream 推断不得直接升级为 authority。
- [ ] **GMH-22：Recall 与上下文预算管线（W5 D1–D4）**
  - 分离候选召回、权限/敏感过滤、相关性排序、去重、时间衰减、token budget、最终渲染。
  - 每条注入内容可追踪到来源和选择理由；pending/rejected/expired/tombstoned 内容永不进入默认上下文。
  - 定义 degraded/fallback：检索失败时显式降级，不能静默使用陈旧或越权数据。
- [ ] **GMH-23：Memory 写入全部提案化（W5 D4–W6 D3）**
  - 会话同步、AutoDream、GUI 直接编辑、导入/迁移统一生成 proposal；按分类和风险决定自动应用或 HITL。
  - 高风险类别（身份、关系、承诺、敏感事实、批量删除）必须人工确认；低风险可在可配置策略下自动应用。
  - apply 必须原子化并生成 changelog/audit；支持 edit-and-approve、冲突检测、撤销/补偿和遗忘请求。
- [ ] **GMH-24：Memory 迁移与回归 Gate（W6 D3–D5）**
  - 影子读对比旧/新结果；建立召回准确性、错误注入率、重复率、延迟、token 成本、提案接受率基线。
  - Gate G2：fixture 迁移可回滚、authority 不丢失、旧配置兼容、Mentle/default lane 均通过；否则不切写路径。

### Phase 3 — Human-in-the-loop 端到端闭环（第 5–7 周，可与 GMH-22 前半并行）

- [ ] **GMH-30：统一审批协调器（W5 D1–W5 D5）**
  - 将 Plan、Sandbox 与 Memory 请求接入同一协调接口；领域执行器只消费有效 receipt，不直接询问 UI。
  - 支持 suspend/resume、进程重启恢复、取消传播、超时、重复响应、失联客户端和多客户端并发。
  - 审批 receipt 必须绑定请求哈希、策略版本、资源 scope 和执行次数；禁止用布尔值表达长期授权。
- [ ] **GMH-31：Manager API / SSE / Tauri 契约（W6 D1–D4）**
  - 提供 pending 列表、详情、approve/edit/reject/cancel、审计查询；所有 mutation 使用幂等键和版本前置条件。
  - 定义稳定 DTO、typed reason codes 和事件序列：requested → awaiting_human → decided → executing → succeeded/failed/compensated。
  - 权限校验在服务端完成；GUI 隐藏按钮不构成安全边界。
- [ ] **GMH-32：GUI 决策中心与就地审批（W6 D3–W7 D3）**
  - 展示动作、目标、风险、证据、diff、命令/路径/网络范围、授权持续时间和拒绝影响。
  - 支持 Memory diff 编辑后批准、一次/会话/规则授权、批量操作限制、倒计时、撤销与失败重试。
  - 无障碍、窄窗、离线/重连、重复事件去重、跨页面 pending badge 纳入测试。
- [ ] **GMH-33：CLI/headless 行为（W7 D2–D4）**
  - 交互 CLI 可审批；非交互环境按配置明确 fail/queue，绝不默认放行。
  - 给服务模式定义 webhook/外部审批扩展点，但本轮不绑定具体第三方平台。
  - Gate G3：真实 shell、Memory 高风险写入、Plan 执行各完成一条 approve/reject/timeout/restart E2E。

### Phase 4 — 接入 Agent Loop、自治治理与可观测性（第 7–8 周）

- [ ] **GMH-40：Agent Loop 单一副作用 seam（W7 D4–W8 D2）**
  - 工具组装、pre-call 和实际执行均引用同一治理快照；消除“已登记但可绕过”和“批准后 registry 未刷新”。
  - turn pipeline 明确 prepare → recall → deliberate → propose → decide → execute → sync → audit；每段可取消、可度量。
  - subagent、cron、heartbeat、AutoDream 继承父授权的方式必须显式，禁止权限放大。
- [ ] **GMH-41：自治预算与熔断（W8 D1–D3）**
  - 按 turn/session/day 限制工具次数、费用、写入量、审批数量和连续失败；拒绝风暴触发熔断。
  - 用户在场/离线作为上下文信号，不作为绕过审批的授权；高风险离线动作必须排队或拒绝。
- [ ] **GMH-42：治理可观测性与审计（W8 D2–D5）**
  - 指标：decision latency、人工等待、approve/reject、policy deny、stale receipt、Memory proposal/apply/rollback、recall quality。
  - 日志统一 correlation ID 并 redact 敏感内容；提供从用户决定到最终副作用的证据链。
  - Gate G4：故障注入证明执行失败、审计失败、UI 断线、存储冲突均不会绕过策略或丢失恢复线索。

### Phase 5 — 迁移、灰度、验收与收口（第 9–10 周）

- [ ] **GMH-50：兼容迁移与 feature flags（W9 D1–D3）**
  - 分开控制 unified governance、memory-v2 read、memory-v2 write、HITL UI；默认先 shadow，再 read cutover，最后 write cutover。
  - 每个 flag 有配置迁移、启动校验、降级路径和删除日期；禁止长期双写。
- [ ] **GMH-51：安全与数据恢复演练（W9 D3–D5）**
  - 覆盖恶意 Memory 注入、路径/命令混淆、scope 扩大、receipt 重放、审批竞态、数据库损坏、部分写入和时钟漂移。
  - 从备份恢复 authority/ledger，验证 pending/approved/executed 状态不重复执行。
- [ ] **GMH-52：全量验收（W10 D1–D3）**
  - 执行 `just fmt-check`、`just check`、`just test`、Mentle lane、GUI tests/build，以及 CLI/Manager/Tauri 最小真实路径 smoke。
  - 性能门槛：策略判定 p95、召回 p95、prompt token 增量、Manager 事件延迟不超过 Phase 0 约定预算。
  - 产品验收：用户能看懂“为什么问我、会改什么、授权多久、如何撤销”，并能从审计中心还原全过程。
- [ ] **GMH-53：灰度与清理（W10 D3–D5）**
  - 小样本开启 → 观察 → 扩大；出现越权、数据丢失、重复执行、不可恢复审批时立即回滚。
  - 删除旧审批布尔捷径、重复 store、废弃 DTO 与双写代码；更新运维手册、威胁模型、用户文档和 `TODOLIST.md`。
  - Gate G5：连续观察窗口内无 P0/P1，回滚演练成功，遗留项已分级并有 owner。

### 里程碑、依赖与并行建议

- [ ] **M0 / 第 1 周末：基线冻结** `GMH-01..03` 完成；没有 G0 不进入领域模型实现。
- [ ] **M1 / 第 3 周末：治理内核可用** `GMH-10..12` 完成；Memory/HITL 只能依赖该契约，不能各建策略引擎。
- [ ] **M2 / 第 6 周末：Memory v2 可影子运行** `GMH-20..24` 完成；旧权威仍可回退。
- [ ] **M3 / 第 7 周末：HITL 闭环可用** `GMH-30..33` 完成；批准、拒绝、超时、重启均有 E2E。
- [ ] **M4 / 第 8 周末：Agent Loop 接入** `GMH-40..42` 完成；所有生产副作用经过统一 seam。
- [ ] **M5 / 第 10 周末：灰度发布完成** `GMH-50..53` 完成。
- 并行规则：W4 后 Memory 数据模型与 HITL 传输/UI 可并行；共享 `agent-diva-core` 契约、Manager state、事件 DTO 时必须先拆文件级 story 并在 `LOCK.md` 声明；迁移、全 workspace 格式化和 schema 变更使用 `GLOBAL` 锁。

### 每个 Story 的统一完成定义（DoD）

- [ ] 设计/ADR 与 threat model 更新；API/schema/配置兼容性明确。
- [ ] 成功、拒绝、超时、并发、重启、降级和回滚路径均有确定性测试。
- [ ] 用户可见变更有 CLI/GUI/Channel 至少一条真实 smoke；GUI 变更另做 GUI smoke。
- [ ] `summary.md`、`verification.md`、`release.md`、`acceptance.md` 齐全；发现但未修问题回填本 `TODOLIST.md`。
- [ ] 每个 story 独立 Conventional Commit，只暂存本 story 文件；不推送，除非用户明确要求。

## Completed Archive

- 历史已完成项目已迁移至 [`docs/archive/todolist/completed-through-2026-07-29.md`](docs/archive/todolist/completed-through-2026-07-29.md)。
- 主 `TODOLIST.md` 只保留活动计划、未完成/延期事项，以及仍服务于开放验收工作的 review execution evidence。
