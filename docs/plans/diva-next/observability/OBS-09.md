# OBS-09 — Packaged observability acceptance and scope closure Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Prove readable logging, correct Journal statistics/trajectory and useful DIVA console together in the actual native package.
**Architecture:** Acceptance joins existing producers/consumers; DN-L/DN-P retain artifact and process ownership.
**Tech stack:** Accepted Windows/native matrix, sealed Generation, existing Go/GUI checks and real safe model/tool smoke.
**Epic / requirements:** OBS-D / O7,O8
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-06, OBS-07, OBS-08 accepted consumers; DN-P accepted native core chain. Backend Stories and DN-2 are transitive inputs, not redundant immediate predecessors.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Extend DIVA existing boundary/acceptance tests chosen by DN-P, `docs/plans/diva-next/backend-separation-contracts.md` ledger rows after DN-0 creates it, parent index/evidence and iteration `docs/logs/<execution-iteration>/...`. Add proposed fixture-backed GUI integration tests under `agent-diva-gui/src/api/vivy/` only for cross-component behavior not already tested. VIVY checks remain in its existing test/CI owners; do not create a second harness/product build path.

Consume exact fixture versions and producer/client commits; DN-L pack/inspect outputs include Generation ID, artifact hash, selected Recipe, C header, runtime dependencies and accepted platform. Observability-added RPCs must be present in this newly built artifact; an older accepted artifact is insufficient.

## Ordered tasks

- [ ] Reconcile O1–O8 against actual commits/tests and DN-0 command/data ledger. Required unresolved rows stay blockers; explicit retirement needs user scope authorization. Mark global audit, auxiliary all-process accounting and historical trajectory paging distinctly from delivered diagnostics/chat_runs/recent-run views.
- [ ] Run required VIVY CI for the integration revision, GUI full test/build and DN-P native bridge/package gates with exact commands. Record result/skip/blocker separately; do not claim cargo/pack acceptance from absent scaffolding or a Linux Go unit pass.
- [ ] Launch the sealed package with a new data root. Verify terminal vs redirected vs daily-file formatting; runtime and GUI secret sentinels remain redacted. Inspect artifact/capabilities to exclude retired Manager and alternate domain storage.
- [ ] Execute a real model turn plus safe tool follow-up, waiting approval, resume, child task and cancellation. Compare persisted model request/usage/finish attempts with trajectory and stats. Repeated snapshot/replay must not change totals; nil usage/unknown price tests stay visibly incomplete.
- [ ] Close/reopen the window during activity, then exercise notification gap/disconnection and application restart. Cursor recovery yields the same authoritative state; logger remains alive after window detach and closes only on explicit Quit. Exercise shutdown with active producer/poll/diagnostic writes.
- [ ] In browser preview, verify explicit unavailable state and absence of fabricated mutation success. Audit required placeholder rows: no mock/hidden feature is called accepted parity.
- [ ] Save sanitized screenshots/transcripts + exact artifact/core pins, reconcile parent DN-3 observability completion and commit `test(observability): verify packaged diva evidence flows`. Full DN-M/DN-8 release remains separate.

## Verification and acceptance oracle

Expected red: gate fails until every immediate predecessor has evidence. Deterministic fixtures prove replacement/unknown/status edge cases; real native smoke proves actual transport, persistence and lifecycle. Expected green: both fixture and real-package evidence pass every required row; no blocked row is silently accepted. Both are necessary. Compare usage subtotals/attempt coverage and per-run stable IDs with Journal replay, not provider invoice guesses or diagnostic files. Price estimates need the recorded catalog/model route, not a hard-coded dollar target.

Return a requirement-by-requirement acceptance matrix, artifact/hash/Generation/platform, commands/results, sanitized replay/screen evidence, remaining ledger blockers and scoped commit. Review focus: no mock predecessor, no stale artifact, no missed required placeholder and no whole-product completion claim. If a test cannot run, leave its gate blocked with the concrete missing producer/runner and next action.
