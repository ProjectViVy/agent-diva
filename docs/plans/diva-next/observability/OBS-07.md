> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# OBS-07 — Single-owner trajectory and child references Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Mount selected-session trajectory with correct call/run/usage state and stable replay recovery.
**Architecture:** Reuse the chat controller as subscription owner and project snapshots/events into a read-only trajectory consumer. Keep OBS-07 trajectory ownership.
**Tech Stack:** Vue/TypeScript, trajectory/session v2, Vitest
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** OBS-C / R-2, R-3, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- Duplicate/out-of-order/late events and initial snapshot race cannot duplicate rows.
- Gap/refetch must preserve contiguous per-run cursor and visible incompleteness.
- Waiting approval is run state, not a model-call spinner.
- Session switch/reopen rejects stale snapshots and leaves no listeners.
- Child links cannot imply child authorization, terminal state or approval decisions.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/state/vivy-chat.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/contracts.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/components/ConsoleView.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/App.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/state/vivy-trajectory.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/vivy-trajectory.test.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/components/console/TrajectoryPanel.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/components/console/TrajectoryPanel.test.ts`

### Interfaces

Consumes DN-0C trajectory/child fixtures and existing controller replay/discovery. Produces `getSessionTrajectory(client: VivyClient,sessionId: string,limit?: number): Promise<TrajectorySession>` in the shared proposed observability facade, and a snapshot/event projection with stable backend IDs.

C2-1 fields: projection_version 2, records/requests/run_activity/watermarks/has_older_runs; default 20/max 50 runs. One owner subscribes; console never subscribes independently. Preserve call_status versus run waiting/wait_kind, usage_evidence nullable, child/workflow/parent references only where verified. No restored generic child management or raw thinking/provider payload display.

### Ordered steps

- [ ] **Step 1:** Add `stableIdsOnSnapshotReplay`, `callFinishBeforeMessage`, `waitVsActiveCall`, `gapRefetchSingleOwner`, `lateSessionResponseIgnored` assertions using exact producer fixtures.

- [ ] **Step 2:** Run new projection/panel tests red; expose read-only event/projection notifications from the existing chat owner and implement trajectory snapshot/dedup/reconciliation without a second subscribeRun.

- [ ] **Step 3:** Mount bounded trajectory rows/activity/wait reasons/usage evidence and verified child/parent navigation. Use stable call/run/request/record IDs; array indices are not identities.

- [ ] **Step 4:** Show recent-run window/has_older_runs and unavailable/gap states honestly. No claim of full historical paging or additional workflow management; details are sanitized and bounded.

- [ ] **Step 5:** Run GUI checks and developer-native safe tool→approval wait→child activity→cancel→reopen comparison with Journal replay; commit `feat(trajectory): mount authoritative diva activity`.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/state/vivy-trajectory.test.ts src/components/console/TrajectoryPanel.test.ts src/state/vivy-chat.test.ts`; `just gui-test`, `just gui-build`. Expected: snapshot/replay equivalence, one listener owner, stable rows, correct wait/call/null-usage semantics. Missing child fixture/capability blocks that slice explicitly.

### Acceptance and handoff

Return trajectory/child DTO versions, listener/gap/replay evidence and view snapshots. Integrate contracts/client/App.vue serially with DN-2 and other OBS edits; file conflict is not an invented logical DAG dependency.

OBS-D1 provenance: these IDs retain their original outcomes from `docs/observability-migration-plan` at `364163e97c972cec9aef007f1be72005af60f25a`. DN-C2 source DTOs and closure scope supersede obsolete file/producer assumptions; this is the same OBS track, not new OBS IDs.

