# TODOLIST

项目级待办、延期项、评审计划与已完成事项记录。

## Active Plan

- [ ] **Plan/TODO architecture implementation** Execute the project-management plan in [09-project-management.md](docs/architecture/plan-todo/09-project-management.md), with the detailed architecture anchors in `docs/architecture/plan-todo/01-architecture-exploration.md` through `13-acceptance-criteria.md`. This is the only active stream.
  - Scope: P1 core state/capability policy → P2 revision-bound approval and optional TODO materialization → P3 agent-loop enforcement → P4 GUI projection → P5 regression and release validation.
  - Rule: before approval, file writes, shell execution, MCP, spawning, scheduling, and other external mutations remain denied by runtime policy.
  - Rule: TODO is optional and is materialized only after approval when selected by the user or plan.
  - Validation: `just fmt-check && just check && just test`, focused crate/GUI tests, and end-to-end denial/approval scenarios.

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
