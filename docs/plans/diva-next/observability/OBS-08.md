> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# OBS-08 — Bounded diagnostics and honest GUI persistence Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Show bounded runtime/GUI logs and visible append loss without recursion or blind write retries.
**Architecture:** Use VIVY-owned diagnostic families via the shared client, plus a bounded in-memory GUI recorder projection. Keep OBS-08 diagnostics ownership.
**Tech Stack:** Vue/TypeScript, diagnostic RPCs, Vitest
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** OBS-C / R-2, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Diagnostic errors cannot feed their own recorder recursively.
- Rotation/truncation produces visible gap and authoritative reread.
- Partial/unknown append never increments confirmed persisted count.
- Queue overflow/disconnect shows loss without blocking chat.
- Speech diagnostics exclude secret/text/audio/body/full URL before recording.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/components/ConsoleView.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/contracts.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/api/vivy/observability.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/gui-diagnostics.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/gui-diagnostics.test.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/components/console/DiagnosticsPanel.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/components/console/DiagnosticsPanel.test.ts`

### Interfaces

Consumes DN-0C diagnostics/append error fixtures and the implemented shared client baseline. Extend the single observability facade from OBS-06/07. Produces `readDiagnostics(client,q: DiagnosticQuery): Promise<DiagnosticPage>`, `appendGuiLogs(client,batch: GuiLogBatch): Promise<{accepted:number}>` and local `recordGuiDiagnostic(record): void` + visible queued/persisted/dropped/unconfirmed state for DN-6C.

Producer limits: 500 rows, 8 KiB each, 4 MiB scan/batch; client batches ≤50/256 KiB leave ABI headroom. Queue ≤1000 records/1 MiB serialized, drop oldest with visible count; these are proposed implementation guards, not performance claims. Do not parse accepted-prefix counts from error text or blindly retry ambiguous writes. `source/date/after/limit/level/query` and gap/has_more/truncated remain verbatim. No browser path input or frontend durable log DB.

### Ordered steps

- [ ] **Step 1:** Add `partialAppendUnknownNoRetry`, `readerErrorNoRecursion`, `rotationGapVisible`, `queueBoundsAndDropCount`, `speechSentinelRedacted` tests; assert at most 50 records/256 KiB per call and no retry after ambiguous prefix write.

- [ ] **Step 2:** Run proposed recorder/panel suites red; implement bounded typed read/append wrappers, sanitized allowlisted records and recorder accounting. Success ack counts only accepted rows; uncertainty remains unconfirmed.

- [ ] **Step 3:** Mount runtime/GUI source/date/filter/next-page/manual refresh with gap/truncated states. Use ConsoleView; the old AuditPage/guiLogger files in OBS-D1 are absent and are not assumed existing.

- [ ] **Step 4:** Expose the bounded recorder entry point for main-window speech diagnostics. Its own transport/query/error paths are excluded from capture; no full transcript or raw console/provider payload is forwarded.

- [ ] **Step 5:** Run GUI checks and disposable native log write/secret sentinel/rotation/error/reopen smoke; commit `feat(diagnostics): show logs and persistence loss`.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/state/gui-diagnostics.test.ts src/components/console/DiagnosticsPanel.test.ts`; `just gui-test`, `just gui-build`. Expected: exact page semantics, bounded queue/frame, visible loss/unknown prefix, no recursion or secrets. Real filesystem rotation proof belongs to developer smoke and OBS-09.

### Acceptance and handoff

Return recorder entry point, queue/ack/redaction fixtures and native diagnostic evidence. DN-6C consumes this queue. This delivers diagnostics, not a generic audit system or reliable frontend log spool.

OBS-D1 provenance: these IDs retain their original outcomes from `docs/observability-migration-plan` at `364163e97c972cec9aef007f1be72005af60f25a`. DN-C2 source DTOs and closure scope supersede obsolete file/producer assumptions; this is the same OBS track, not new OBS IDs.

