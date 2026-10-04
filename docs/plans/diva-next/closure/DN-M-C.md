> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# DN-M-C — Audit scoped behavior and boundary closure Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Reconcile required behavior and exact native/backend boundaries against the integrated candidate.
**Architecture:** Reuse existing DN-M AST/dependency/manifest gates and current ledger/TODOLIST. Required live paths must exist; deferred/cancelled surfaces are visible and cannot masquerade as delivered parity.
**Tech Stack:** Existing Python/Node gates, owning test suites, native candidate
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-M / R-1, R-2, R-3, R-4, R-5, R-7, R-8, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Broad dormant exemptions must not hide newly reachable speech code.
- Static caller scans alone cannot prove real required UI/authority behavior.
- Fresh Next home must not import/live-read/dual-write/delete legacy homes.
- Unused controls cannot report successful unavailable operations.
- Unknown/pending checks and cancelled DN-7 cannot be labelled complete parity.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `scripts/ci/check_legacy_frontend_calls.mjs`
- **DIVA / modify existing:** `scripts/ci/check_vivy_backend_boundary.py`
- **DIVA / modify existing:** `docs/plans/diva-next/backend-separation-contracts.md`
- **DIVA / modify existing:** `TODOLIST.md`
- **DIVA / modify existing:** `docs/plans/diva-next/index.md`
- **DIVA / verify existing:** `agent-diva-gui/src-tauri/Cargo.lock`
- **DIVA / verify existing:** `agent-diva-gui/src-tauri/crates/vivy-bridge/Cargo.lock`

### Interfaces

Consumes OBS-09 integration/evidence plus its exact DN-P-C candidate. Produces R-1…R-9 scoped engineering matrix, boundary/command/dependency proof and residual dispositions. R-6 installed target completion remains DN-8C.

DN-6A/B/C amend native/AST gates in their own commits; this Story audits the complete reachable graph, not first permits speech at the end. Only the C2-4 exact commands and `speech:diagnostic` exist. Activated voice is outside dormant pet exemptions; generic invoke/dynamic imports/browser HTTP/old pet_* fail. Check the selected source/action inventory and current lockfile dependency audit. Issue #8's old vulnerabilities are not proof of the present locks.

### Ordered steps

- [ ] **Step 1:** Reconcile the current ledger's required/available/deferred/cancelled rows with actual implementations and evidence. Mark cancelled import separately from a successful importer; capture fresh-home no-legacy-read evidence.

- [ ] **Step 2:** Add only necessary negative fixtures for reachable browser speech/provider bypass, generic/dynamic native calls, unlisted native handlers and dormant-path imports. Run current gates before changing allowlists; required invalid fixtures must fail.

- [ ] **Step 3:** Audit dependency closure/current lockfiles and source/Generation inventory; run the available owning dependency-security command with its advisory date/result. Never claim vulnerability-free on unavailable audit feeds.

- [ ] **Step 4:** Exercise every touched operational setting/control against real RPC/native errors. Preserve deferred skill/rule/wipe/search/MCP/report/resource/pet surfaces in TODOLIST with honest unavailable UI; no unsolicited parity implementation.

- [ ] **Step 5:** Run scoped semantic/GUI/native integration checks for cognition/chat/voice/console/fresh-home. Record missing environment checks and immutable artifact pins in the existing iteration logs.

- [ ] **Step 6:** Commit ledger/index/TODO/gate closure evidence and pass exact remaining native acceptance rows to DN-8C. Governance instruction rewrites require separate explicit user authorization and are not a hidden product implementation workaround.

### Verification

`node scripts/ci/check_legacy_frontend_calls.mjs --selftest`; `python3 scripts/ci/check_vivy_backend_boundary.py`; `just ci`, affected `just shell-test`/`just shell-clippy` and exact owning semantic suites. Current native lock audit uses available audit tooling and records tool/advisory versions. Expected: invalid reachable paths rejected, required behavior evidenced, remaining dispositions explicit and no hidden import/authority fallback.

### Acceptance and handoff

Return requirement matrix, exact gate fixtures/inventories, current dependency-audit evidence and unchanged scope exclusions. DN-8C receives all pending native/owner rows. No rule-file rewrite, product release or final acceptance is authorized by this audit plan.

