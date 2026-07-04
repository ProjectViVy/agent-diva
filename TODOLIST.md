# TODOLIST

项目级待办、延期项、评审计划与已完成事项记录。

## Open

_(No active open items. This pass closes what can be closed and explicitly defers everything else.)_

## Deferred

- [ ] **Production cron clock abstraction** Deferred. `runtime.rs:274` still uses a hardcoded clock path and needs abstraction for testability and portability.
  - Related files: `agent-diva-manager/src/runtime.rs`
- [ ] **Audit GUI i18n** Deferred. 3 Vue components still use hardcoded English strings without `useI18n`.
  - Related files: `agent-diva-gui/src/components/`
- [ ] **UX-DR-3/4/7** Deferred. UX gaps from sprint review remain postponed until a dedicated design pass.
  - Context: Sprint closure review items 3, 4, and 7

## Review Program

### Scope

- Range: `94baa4b..HEAD`
- Exclude: `docs:*`, research/archive-only changes, `cbab379`, `ba1d17b`
- Include: `feat:*`, `merge:*`, behavior-relevant `refactor:*`, validation-relevant `chore:*`, and `style:*` only for semantic-risk sampling

### Wave Plan

- [ ] **Wave A - 基础设施与 Harness 基线**
  - Commits: `e3cd30c`, `3322aac`, `1627ea3`, `0183c3c`
  - Focus: `ToolRegistry::execute()` 调用链迁移、`PokeEvent` 广播安全、Harness 与真实运行语义一致性、E2E 迁移后的断言与环境假设
- [ ] **Wave B - 安全 / 监督运行 / 预算治理**
  - Commits: `45b6aa6`, `9242579`, `ce70902`, `7ca1c92`, `f43ff96`, `0c1d1bd`, `8ecf041`
  - Focus: Token ledger 记账时机、subagent budget 强制生效、`UsageRecord`/`ContextBudgetPolicy`/`OverflowAction` 落点、安全收口默认行为、merge 带来的逻辑分叉
- [ ] **Wave C - 观测性 / Audit 主线**
  - Commits: `c58054c`, `28b00ce`, `41c829a`, `bd96dd2`, `de031fa`, `0851c4c`, `6ac4056`, `166fd16`, `b3fd4e8`
  - Focus: Audit schema 一致性、`GLOBAL_SINK` 初始化和线程安全、JSONL rolling 并发写、`/api/logs` 查询闭环、parser refactor 兼容性
- [ ] **Wave D - 上下文压缩 / 限流**
  - Commits: `0eb0cce`, `e2e2ad3`
  - Focus: TokenBucket refill/burst 边界、MetaCompactor 语义保持、触发顺序、失败路径稳定性
- [ ] **Wave E - Todo 数据面**
  - Commits: `909a573`, `84c5803`, `2445984`
  - Focus: API/CLI/store 状态机一致、archive/purge 误删风险、并发写入与重复 ID 边界
- [ ] **Wave F - 后台任务 / 子代理 / workspace CLI**
  - Commits: `c0f2712`, `c705538`, `b633b0d`
  - Focus: background task 生命周期、`SubagentRunHandler` 覆盖度、预算/权限/审计继承、workspace 路径隔离
- [ ] **Wave G - 横切补强**
  - Commits: `11728fa`, `9438b25`, `48dd875`, `e9336d9`, `e2941a8`
  - Focus: usage fallback 统计准确性、`ErrorCategory` 分类失真、timeout wrapper 语义变化、feature gate 漏检、rustfmt 大提交夹带逻辑改动

### Execution Order

- [ ] **Priority 1** Review `Wave B`
- [ ] **Priority 2** Review `Wave C`
- [ ] **Priority 3** Review `Wave F`
- [ ] **Priority 4** Review `Wave E`
- [ ] **Priority 5** Review `Wave D`
- [ ] **Priority 6** Review `Wave A`
- [ ] **Priority 7** Review `Wave G`

### Wave 1 Parallel Review

- [ ] **Wave 1 scope lock**
  - Coverage: `Wave A` and `Priority 1` (`Wave B`)
  - Goal: launch parallel review for the foundation path and the first release-risk path together
  - Output: one `Wave A` report, one `Wave B` report, and one cross-wave risk matrix
- [ ] **Lead-Agent**
  - Responsibilities: freeze commit map, assign tasks, deduplicate findings, arbitrate overlaps, publish final summary
  - Required outputs: commit-to-owner table, merged findings list, release/block recommendation
- [ ] **A1-Infrastructure**
  - Commits: `e3cd30c`, `3322aac`
  - Focus: `PokeEvent` fan-out, subscriber lifecycle, backpressure, `ToolRegistry::execute()` error propagation and caller migration completeness
- [ ] **A2-Harness**
  - Commits: `1627ea3`, `0183c3c`
  - Focus: Harness V2 entrypoints, fixture realism, CI baseline assumptions, E2E migration assertion drift and environment contract changes
- [ ] **B1-Budget**
  - Commits: `45b6aa6`, `9242579`, `7ca1c92`
  - Focus: token ledger write timing, budget enforcement on hot paths, subagent per-task budget coverage, policy definitions vs real execution
- [ ] **B2-SupervisedRun**
  - Commits: `ce70902`
  - Focus: supervised-run state transitions, persistence, cancellation ownership, budget/audit/security wiring
- [ ] **B3-SecurityMerge**
  - Commits: `f43ff96`, `0c1d1bd`, `8ecf041`
  - Focus: `SecurityDecision`, `check_security()`, audit emissions, default policy semantics, merge regression risk, high-risk drift sampling
- [ ] **Wave 1 execution flow**
  - Phase 0: `Lead-Agent` publishes commit map and review packet
  - Phase 1: `A1`, `A2`, `B1`, `B2`, `B3` review in parallel
  - Phase 2: `Lead-Agent` runs cross-checks for budget/security, harness/supervised-run, and error propagation gaps
  - Phase 3: `Lead-Agent` publishes `Wave A` and `Wave B` summaries plus one shared priority pool
- [ ] **Wave 1 report template**
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

- [ ] **Wave A / `e3cd30c`** Verify `PokeEvent` broadcast fan-out, subscriber lifecycle, and backpressure assumptions.
- [ ] **Wave A / `3322aac`** Verify every `ToolRegistry::execute()` caller correctly handles `Result<ToolError>`.
- [ ] **Wave A / `1627ea3`** Review Harness V2 entrypoints, fixture realism, and CI baseline assumptions.
- [ ] **Wave A / `0183c3c`** Review E2E migration for assertion drift and environment contract changes.
- [ ] **Wave B / `45b6aa6`** Verify token ledger writes exactly once per billable usage and enforces budgets on the real hot path.
- [ ] **Wave B / `9242579`** Verify per-task subagent budget is enforced, not merely logged.
- [ ] **Wave B / `ce70902`** Review supervised-run MVP state transitions, persistence, and cancellation ownership.
- [ ] **Wave B / `7ca1c92`** Verify usage source mapping and context budget policy integration points.
- [ ] **Wave B / `f43ff96`** Review `SecurityDecision`, `check_security()`, audit emissions, and default-deny/default-allow semantics.
- [ ] **Wave B / `0c1d1bd`** Audit merge integration for duplicate paths, stale gates, and branch-resolution regressions.
- [ ] **Wave B / `8ecf041`** Sample high-risk files for accidental behavior drift in the pre-merge catch-up commit.
- [ ] **Wave C / `c58054c`** Verify skill upload/delete/block audit events are emitted on all outcome paths.
- [ ] **Wave C / `28b00ce`** Verify cron start/completion/failure events do not double-fire or miss failures.
- [ ] **Wave C / `41c829a`** Review `ProviderTap` timing, streaming accumulation, and token accounting correctness.
- [ ] **Wave C / `bd96dd2`** Review `ToolExecutionTap` around success/error/timeout and nested tool calls.
- [ ] **Wave C / `de031fa`** Verify JSONL daily rolling, concurrent writes, and write-error swallowing behavior are intentional.
- [ ] **Wave C / `0851c4c`** Review `AuditSink` registration, `GLOBAL_SINK` initialization ordering, and no-op fallback behavior.
- [ ] **Wave C / `6ac4056`** Verify `/api/logs` query fields match persisted audit schema exactly.
- [ ] **Wave C / `166fd16`** Review parser refactor for GUI/manager compatibility and malformed-line handling.
- [ ] **Wave C / `b3fd4e8`** Verify health endpoint criteria and benchmark assumptions are stable and meaningful.
- [ ] **Wave D / `0eb0cce`** Review token bucket refill math, monotonic-time assumptions, and burst depletion edges.
- [ ] **Wave D / `e2e2ad3`** Review MetaCompactor summary fidelity, fallback behavior, and serialization compatibility.
- [ ] **Wave E / `909a573`** Verify todo CRUD routes, filters, 404 paths, and store integration.
- [ ] **Wave E / `84c5803`** Verify todo CLI behavior matches HTTP and store semantics.
- [ ] **Wave E / `2445984`** Review archive/purge for active-item safety, historical retention, and concurrent update behavior.
- [ ] **Wave F / `c0f2712`** Review enqueue background task lifetime, observability, cancellation, and error return path.
- [ ] **Wave F / `c705538`** Verify `RunKind::Subagent` dispatch covers every subagent execution path.
- [ ] **Wave F / `b633b0d`** Review workspace CLI path resolution, isolation, and failure output quality.
- [ ] **Wave G / `11728fa`** Verify usage fallback metrics/warnings neither double-report nor mask real provider usage.
- [ ] **Wave G / `9438b25`** Review `ErrorCategory` trait adoption and risk of over-generalized classification.
- [ ] **Wave G / `48dd875`** Verify timeout wrapper preserves cancellation, retry, and tool-specific error identity.
- [ ] **Wave G / `e9336d9`** Review feature-gate CI script for missing crate/feature combinations.
- [ ] **Wave G / `e2941a8`** Sample rustfmt-only commit for accidental semantic edits in touched modules.

### Deliverables

- [ ] **Wave reports** Each wave should end with scope, risk summary, findings, and release/block recommendation.
- [ ] **Cross-wave matrix** Aggregate findings by security, observability, persistence, concurrency, CLI/API consistency, and test gaps.
- [ ] **Priority pool** Classify review findings into `P0`/`P1`/`P2`/`P3`.

### Timebox

- [ ] **Day 1** `Wave B` + `Wave C`
- [ ] **Day 2** `Wave F` + `Wave E`
- [ ] **Day 3** `Wave D` + `Wave A` + `Wave G`
- [ ] **Day 4** Cross-wave regression pass, unified conclusion, and priority-pool cleanup

## Done

- [x] **Backlog normalization on 2026-07-04** Closed all already-resolved items and converted all remaining unfinished items to explicit deferred status.
- [x] **Wave 0 CI stabilization** Previous CI and workspace verification blockers were already cleared.
- [x] **Plan mode runtime wiring** Previously completed and validated in `docs/logs/2026-06-plan-mode-runtime/v0.0.1-plan-mode-runtime-wiring/verification.md`.
- [x] **Parallel lock mechanism** Repository-level `LOCK.md` workflow had already been introduced before this pass.
