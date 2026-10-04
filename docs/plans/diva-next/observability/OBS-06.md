> **Historical under DN-W3 (2026-10-04).** This file preserves prior scope,
> contracts and evidence. Its Tauri/C ABI/Rust host execution steps and
> scheduling/readiness statements are inactive. Use the [current index](../index.md)
> and its W0–W7 plans. Domain behavior and explicitly deferred scope remain
> reference material; prior package evidence is not Wails acceptance.

# OBS-06 — Truthful token dashboard and host connection Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Show actual v2 usage coverage and current host connection without fabricated totals, prices or save success.
**Architecture:** Use one typed snapshot request and existing transport/controller metadata. Preserve OBS-06 token/connection ownership from the original plan.
**Tech Stack:** Vue/TypeScript, Vitest, stats/tokens v2
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

- Nil usage, partial usage and reported zero have different labels.
- Unknown model price and hidden retries cannot be called a provider invoice.
- Delayed response after session switch cannot replace current totals.
- Preview/disconnect cannot show successful mutation or live host metadata.
- Refresh/listener teardown must avoid duplicate requests and durable totals.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/api/tokenStats.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/tokenStats.test.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/api/vivy/contracts.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/components/console/TokenStatsPanel.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/ConsoleView.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/api/vivy/observability.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/vivy-observability.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/state/vivy-observability.test.ts`

### Interfaces

Consumes DN-0C actual stats DTOs plus the already implemented DN-2 controller/transport baseline. Produces `getTokenUsage(client: VivyClient,params: TokenUsageParams): Promise<VivyTokenUsageSnapshot>` and connection projection derived from initialize/ABI/transport, not a new health RPC.

Coverage uses the C2-1 source fields. Observed attempts partition into completed-with-usage/partial/missing/active; reported_calls is not an extra partition. `request_count` means usage reports; absent reasoning/cached buckets are unknown; `cost_known:false` never renders free. One in-flight refresh, stale-response/session fencing and visible stale/empty/unavailable states; no cache service or fabricated whole-config save.

### Ordered steps

- [ ] **Step 1:** Extend token/state tests with source fixtures for complete/partial/missing/active/legacy, cost_known false and out-of-order refresh. Assert unknown is not numeric zero/free and reported_calls is not double-counted.

- [ ] **Step 2:** Run focused tests red; implement typed stats/connection projections and update existing token DTOs verbatim from producer structs.

- [ ] **Step 3:** Coalesce only the current in-flight snapshot request. Finish/usage/reconnect invalidates it; stale request/session responses cannot win. Reuse connection metadata, with no gateway start/stop controls or healthz polling.

- [ ] **Step 4:** Mount honest coverage/connection/empty/error states in TokenStatsPanel/ConsoleView, add EN/ZH labels and remove false config/save/unsupported TPS claims only where present in touched flows.

- [ ] **Step 5:** Run full GUI checks and developer-host turn→totals→disconnect→reopen smoke. Commit `feat(console): expose usage coverage and host state`.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/api/tokenStats.test.ts src/state/vivy-observability.test.ts`; `just gui-test`, `just gui-build`. Expected: source-field coverage mapping, truthful unknown/stale connection and no duplicate refresh/listener behavior. Final native artifact comparison is OBS-09.

### Acceptance and handoff

Return stats fixture/source digests, mapping/assertion results and sanitized connection/coverage observations. OBS-09 compares these consumers to Journal on the new bundle; no all-process billing coverage is promised.

OBS-D1 provenance: these IDs retain their original outcomes from `docs/observability-migration-plan` at `364163e97c972cec9aef007f1be72005af60f25a`. DN-C2 source DTOs and closure scope supersede obsolete file/producer assumptions; this is the same OBS track, not new OBS IDs.

