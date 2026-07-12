# TODOLIST

项目级待办、延期项、评审计划与已完成事项记录。

## Active Plan

- [ ] **Sandbox command approval UI and persistent execution rules** Implement the design package in `docs/dev/sandbox-command-approval/`: route shell execution through the sandbox orchestrator, surface recoverable approval requests in the GUI, and persist only validated safe command-prefix allow rules globally.
  - Expected behavior: Plan mode remains strictly read-only; agent mode can approve a recoverable sandbox escalation once or per session; only safe prefixes can enter the global default execution list.
  - Related: `agent-diva-tools/src/shell.rs`, `agent-diva-sandbox/src/orchestrator.rs`, `agent-diva-manager/src/handlers.rs`, `agent-diva-gui/src/App.vue`

- [ ] **Plan Mode: make default execution-context Compact a durable summary boundary** The current default `Compact` policy only trims the first execution request to six local messages; it does not create a summary, advance a boundary, or prevent exploratory context from returning in later execution turns. `Clear` is also request-local rather than an execution-session policy.
  - Expected behavior: Compact persists a quality-checked pre-execution summary plus an execution boundary; Clear persists the boundary without a summary; both policies apply to every execution turn while retaining the transcript for audit. Summary failure must be explicit and must not silently retain context.
  - Related: `docs/architecture/plan-execution-context-compaction-fix.md`, `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-core/src/planning/report.rs`, `agent-diva-core/src/planning/report_store.rs`, `agent-diva-gui/src-tauri/src/commands.rs`, `agent-diva-gui/src/App.vue`
  - Suggested validation: deterministic approval-to-execution integration tests for Compact/Clear/Retain, restart persistence, and reactive generic compaction rebuild.

- [ ] **Plan/TODO architecture implementation** Execute the project-management plan in [09-project-management.md](docs/architecture/plan-todo/09-project-management.md), with the detailed architecture anchors in `docs/architecture/plan-todo/01-architecture-exploration.md` through `13-acceptance-criteria.md`. This is the only active stream.
  - Scope: P1 core state/capability policy → P2 revision-bound approval and optional TODO materialization → P3 agent-loop enforcement → P4 GUI projection → P5 regression and release validation.
  - Rule: before approval, file writes, shell execution, MCP, spawning, scheduling, and other external mutations remain denied by runtime policy.
  - Rule: TODO is optional and is materialized only after approval when selected by the user or plan.
  - Validation: `just fmt-check && just check && just test`, focused crate/GUI tests, and end-to-end denial/approval scenarios.

- [x] **P2/P3 approval boundary and runtime capability enforcement (2026-07-11)** Added `plan_submit`, revision-bound store approval, append-only submitted/approved events, revision invalidation on plan-content edits, materialized-TODO replacement protection, and phase-aware tool assembly/pre-call denial. GUI request-body adoption remains P4; manager test execution remains deferred by the local command time limit.
  - Related log: `docs/logs/2026-07-plan-todo-p2-p3/v0.0.1-approval-runtime-enforcement/`

### Plan/TODO P1 core policy — triple-review follow-ups (2026-07-11)

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

### Plan/TODO P2 approval materialization — triple-review follow-ups (2026-07-11)

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

### Plan/TODO P3 runtime capability gate — triple-review follow-ups (2026-07-11)

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

- [x] **Mentle: Windows STATUS_STACK_OVERFLOW on gateway startup** Fixed in `docs/logs/2026-07-10-mentle-windows-stack-overflow/v0.0.2-windows-native-open-isolation/`. Root cause was turso/simsimd stack pressure on default Windows stacks; fixed by process defaults (`LIMBO_DISABLE_FILE_LOCK`), large-stack assemble thread, CLI PE/worker stack 16 MiB. Gateway smoke reaches `Gateway ready` with `tool_count=32`.

- [ ] **Core: stabilize supervised executor no-handler failure test** `cargo test -p agent-diva-core --lib` intermittently/factually failed in `supervised::executor::tests::test_executor_fails_when_no_handler`: the run remained `Running` instead of becoming `Failed`. This is unrelated to the 30-day planning cleanup and leaves the full core library gate red.
  - Related files: `agent-diva-core/src/supervised/executor.rs`
  - Suggested fix: inspect the no-handler executor lifecycle and make the test await the terminal transition or correct the missing-handler failure path.

- [ ] **GUI: duplicate `mode` locale keys** Vite reports duplicate `mode` keys in `agent-diva-gui/src/locales/zh.ts` and `agent-diva-gui/src/locales/en.ts`; remove the duplicate definitions so locale builds are warning-free.

- [ ] **Agent: repair stale `compaction_real_test` integration harness** Running `cargo test -p agent-diva-agent <test-name>` still compiles `agent-diva-agent/tests/compaction_real_test.rs`, which currently targets removed compaction APIs such as `ContextCompactor::new(...)`, `compact_session(...)`, and `CompactTrigger::ProactiveThreshold`. This is unrelated to the image multimodal change but blocks clean package-scoped targeted test commands.
  - Related files: `agent-diva-agent/tests/compaction_real_test.rs`, `agent-diva-agent/src/compaction/compaction_exec.rs`, `agent-diva-core/src/session/`
  - Suggested fix: update the integration test to the current compaction entrypoints/trigger variants or gate it behind an explicit ignored/manual path until it reflects the live API.
- [ ] **Formatting: normalize pre-existing `agent-diva-e2e` rustfmt drift** `cargo fmt --check` is currently blocked by formatting diffs in `agent-diva-e2e/src/{collector,config,lib,report,runner,tracer,types}.rs`. This is outside the provider protocol split scope but prevents a clean workspace-wide fmt gate.
  - Related files: `agent-diva-e2e/src/collector.rs`, `agent-diva-e2e/src/config.rs`, `agent-diva-e2e/src/lib.rs`, `agent-diva-e2e/src/report.rs`, `agent-diva-e2e/src/runner.rs`, `agent-diva-e2e/src/tracer.rs`, `agent-diva-e2e/src/types.rs`
  - Suggested validation: run `cargo fmt -p agent-diva-e2e` in a focused formatting-only change, then rerun `cargo fmt --check`.
- [ ] **Provider: run StepFun real endpoint E2E for model pass-through** The runtime now keeps model IDs opaque and unit coverage verifies `provider_name = stepfun`, `api_base = https://api.stepfun.com/step_plan/v1`, and `model = step-3.7-flash` pass through unchanged. Real StepFun E2E could not be run in this checkout because `keys.txt` is absent and no StepFun API key is available in the environment.
  - Related files: `agent-diva-providers/src/openai_compatible.rs`, `agent-diva-e2e/src/config.rs`, `docs/logs/2026-07-provider-model-pass-through/v0.0.1-provider-model-pass-through/verification.md`
  - Suggested validation: set `E2E_PROVIDER_NAME=stepfun`, `E2E_API_BASE=https://api.stepfun.com/step_plan/v1`, `E2E_MODEL=step-3.7-flash`, and `E2E_API_KEY`/`DEEPSEEK_API_KEY` to a StepFun key, then run `just e2e-test`.
- [ ] **GUI: fix NormalMode.test.ts pre-existing `miku.svg` import failure** The vitest environment cannot resolve `/miku.svg` imported by `NormalMode.vue`, causing the whole `NormalMode.test.ts` suite to fail before any assertions run. This blocks regression testing of sidebar/navigation behavior and is unrelated to the pet overlay fix.
  - Related files: `agent-diva-gui/src/components/NormalMode.vue`, `agent-diva-gui/src/components/NormalMode.test.ts`, `agent-diva-gui/vitest.config.ts`
  - Suggested fix: add an SVG mock/ignore handler in `vitest.config.ts` (e.g., `assetsInclude` or a custom plugin) so static asset imports do not crash tests.
- [ ] **GUI: stabilize `embedded_gateway_serves_health_endpoint` test** `cargo test -p agent-diva-gui` currently fails in `embedded_server::tests::embedded_gateway_serves_health_endpoint` because the health probe returned HTTP 502 instead of the expected 200 during validation for the Tauri watcher fix. This blocks a clean GUI crate test pass and should be isolated from the watcher-only change.
  - Related files: `agent-diva-gui/src-tauri/src/embedded_server.rs`, `agent-diva-gui/src-tauri/tests/gateway_process_management_bugfix.rs`, `docs/logs/2026-07-gui-tauri-dev-exit-watch-loop/v0.0.1-tauri-dev-exit-watch-loop/verification.md`
  - Suggested fix: inspect the embedded gateway startup/readiness handshake in tests and make the health assertion wait for the backend to become ready before asserting `200`.
- [ ] **Plan GUI: remove the temporary visible approval-followup message after inline plan approval** The inline plan approval flow now calls the runtime approval API and then auto-sends a follow-up chat message (`"Plan approved. Execute the approved plan now..."`) to resume execution. This unblocks end-to-end behavior in the current session, but it is still a UX mismatch from the OpenAkita target, where approval continues execution without surfacing an extra synthetic user message.
  - Related files: `agent-diva-gui/src/App.vue`, `agent-diva-manager/src/handlers/planning.rs`, `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`
  - Suggested fix: add a dedicated runtime continuation command that resumes the approved plan in-session without injecting a visible chat turn from the GUI.

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

## Done

- [x] **LLM 归纳手动日报、周报、月报** Implemented fact-bundle collection, optional no-tool LLM curation, evidence validation, deterministic fallback, manager injection, and GUI generation-mode display. Default `reports.llm_curation.enabled=false`. Removed GUI duplicate monthly generator.
  - Related: `docs/plan/llm-curated-manual-reports.md`, `docs/logs/2026-07-llm-curated-manual-reports/v0.0.2-impl-llm-curated-manual-reports/`
  - Validation: core/providers/autodream/gui notebook targeted tests; manager check.
- [x] **GUI: fix pet immersive overlay sidebar navigation** Resolved the issue where clicking overlay sidebar items in pet immersive/fullscreen mode could not navigate back to the main page. The `.pet-immersive:not(.sidebar-expanded) .sidebar` rule applied `pointer-events: none` to the overlay sidebar as well, blocking all clicks. Excluded `.overlay-sidebar` from that rule.
  - Related files: `agent-diva-gui/src/styles.css`
  - Commit: `8c6fa68`
  - Validation: `pnpm build` in `agent-diva-gui` passes; `DivaPetView.test.ts` passes; `NormalMode.test.ts` still blocked by pre-existing `miku.svg` import failure (see Open).
- [x] **Backlog normalization on 2026-07-04** Closed all already-resolved items and converted all remaining unfinished items to explicit deferred status.
- [x] **Wave 0 CI stabilization** Previous CI and workspace verification blockers were already cleared.
- [x] **Plan mode runtime wiring** Previously completed and validated in `docs/logs/2026-06-plan-mode-runtime/v0.0.1-plan-mode-runtime-wiring/verification.md`.
- [x] **Parallel lock mechanism** Repository-level `LOCK.md` workflow had already been introduced before this pass.
- [x] **Wave 1 remediation on 2026-07-04** Closed the reviewed release blockers for E2E false-green behavior, token-budget enforcement, supervised-run cancellation, and security production wiring.
  - Related log: `docs/logs/2026-07-wave1-remediation/v0.0.1-wave1-remediation/`
- [x] **Wave 2 observability remediation on 2026-07-04** Closed the first Wave C runtime blockers around audit sink wiring, `/api/logs` path/cursor behavior, manager audit filtering, cron started-event ordering, and early tool denial audit emission.
  - Related log: `docs/logs/2026-07-wave2-observability/v0.0.1-wave2-observability-remediation/`
- [x] **Wave 3 review on 2026-07-04** Completed the next parallel review stage covering `Wave E` (Todo data plane) and `Wave F` (background task / subagent / workspace CLI), and recorded the resulting release blockers plus deferred follow-ups.
  - Related log: `docs/logs/2026-07-wave3-review/v0.0.1-wave3-summary/`
- [x] **Wave 3 workspace CLI hardening on 2026-07-04** Closed the `workspace` command path traversal, active-workspace delete bypass, and `list` write-side-effect findings, with focused CLI regression coverage.
  - Related log: `docs/logs/2026-07-wave3-remediation/v0.0.1-workspace-cli-hardening/`
- [x] **Wave C readiness and audit closure on 2026-07-05** Closed the direct CLI `ProviderTap` gap, stable skill rejection audit payloads, `JsonlAuditSink` read-after-write/rolling contracts, manager cron clock injection, `/api/health` readiness semantics, and audit-page i18n cleanup.
  - Related log: `docs/logs/2026-07-wavec-remediation/v0.0.1-wavec-remediation/`
- [x] **Wave C + Wave D review closure on 2026-07-05** Closed the remaining `/api/logs` malformed/schema-drift visibility tests, `/api/health` benchmark CI gate, rate limiter retry-after edge semantics, compaction ordering/retry-once guarantees, meta-compaction fact preservation, and session compaction serde compatibility coverage.
  - Related log: `docs/logs/2026-07-wavecd-remediation/v0.0.1-wavecd-review-closure/`
- [x] **Wave G review on 2026-07-05** Completed the parallel review of usage fallback metrics, ErrorCategory adoption, global timeout wiring, feature-gate CI coverage, and the rustfmt-only catch-up commit; review blockers were recorded as deferred residuals instead of being fixed in this read-only pass.
  - Related log: `docs/logs/2026-07-waveg-review/v0.0.1-waveg-summary/`
- [x] **Wave G remediation on 2026-07-05** Closed the OpenAI-compatible missing-usage false-zero fallback, tightened retry categorization and timeout retryability, wired `global_tool_timeout_secs` into production `ToolRegistry` assembly, fixed `logging.retention_days = 0`, and promoted the feature-gate check into a real cross-platform CI gate.
  - Related log: `docs/logs/2026-07-waveg-remediation/v0.0.1-waveg-remediation/`
