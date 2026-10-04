> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# OBS-09 — Verify observability in the new native candidate Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Demonstrate that packaged logs, trajectory and usage match actual persisted Agent work.
**Architecture:** Join accepted consumer evidence with the new sealed candidate and compare to Journal. Retain OBS-09 as a domain engineering gate; final whole-product acceptance remains the owner's.
**Tech Stack:** Native DIVA candidate, VIVY Journal/logging, actual model/tool smoke
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** OBS-D / R-6, R-8, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Replay/repeated snapshot cannot increase usage totals or duplicate trajectory rows.
- Approval wait/cancel/child activity must preserve actual call/run identities.
- Secret sentinels are redacted in GUI/native/runtime logs and displayed errors.
- Rotation/gap/reopen yields truthful incomplete/refetched data.
- A green mock consumer suite is not a native/real-producer acceptance result.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `docs/plans/diva-next/backend-separation-contracts.md`
- **DIVA / modify existing:** `docs/plans/diva-next/index.md`
- **DIVA / create proposed:** `docs/plans/diva-next/fixtures/closure-packaged-obs.json`
- **DIVA / verify existing:** `agent-diva-gui/src/state/vivy-chat.test.ts`

### Interfaces

Consumes DN-P-C immutable candidate, plus OBS-06/07/08 consumer matrices. These direct consumer edges deliberately retain a distinct input: candidate building consumes code commits but does not forward or replace accepted domain/replay/redaction evidence.

Produces pinned packaged capture/evidence for R-9. Compare actual run/call IDs, sequence/watermarks, usage evidence/coverage and diagnostic families against Journal; diagnostics do not become the usage authority. Parent/child/read links have only their proven current scope. No invoice/all-process billing/full-history claim. A newly rebuilt candidate invalidates this packaged evidence.

### Ordered steps

- [ ] **Step 1:** Verify all predecessor source/candidate/fixture hashes before starting. Run existing owning VIVY logging/runtime/RPC checks and accepted DIVA GUI/bridge/native recipes.

- [ ] **Step 2:** Launch fresh candidate state and compare terminal versus redirected versus daily logs. Write safe event plus secret sentinel; verify redacted runtime and GUI diagnostic readback.

- [ ] **Step 3:** Run a real configured model turn/safe tool follow-up, waiting approval/resume, child and cancellation. Compare trajectory/session v2 and token totals/coverage with persisted Journal IDs/evidence; repeat snapshot/replay and assert stable totals.

- [ ] **Step 4:** Exercise window hide/reopen, actual bridge gap/reconnect, app restart and log rotation. Prove one event owner, no append recursion, visible unconfirmed/dropped GUI records and bounded queries.

- [ ] **Step 5:** Save redacted `closure-packaged-obs.json` and evidence/screens with candidate hashes. Mark each R-9 row passed/pending/failed, update central index from actual results and commit scoped packaged OBS evidence.

### Verification

DIVA: `just ci`, `just shell-test`, `just shell-clippy`, AST selftest and native boundary gate on the candidate source. VIVY focused logging/runtime/RPC tests. Native scenarios above require actual model configuration; credentials stay local. Expected: Journal-matched stable projections and redaction/rotation/reopen behavior. Unavailable real-model/Windows access remains pending, not passed.

### Acceptance and handoff

Return R-9 evidence matrix with immutable candidate/core/source pins and actual outcomes. DN-M-C consumes it without converting a domain gate into whole-product final acceptance.

OBS-D1 provenance: these IDs retain their original outcomes from `docs/observability-migration-plan` at `364163e97c972cec9aef007f1be72005af60f25a`. DN-C2 source DTOs and closure scope supersede obsolete file/producer assumptions; this is the same OBS track, not new OBS IDs.

