# OBS-07 — DIVA trajectory and child activity view Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Mount a useful selected-session trajectory that survives live updates, cancellation and window reopen.
**Architecture:** Use the accepted VIVY trajectory snapshot and existing run/child notifications through one shared client.
**Tech stack:** Vue/TypeScript, Vitest, accepted native client/session projection.
**Epic / requirements:** OBS-C / O4
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-04: accepted trajectory v2/capability fixtures; DN-2: real session selection, run discovery, notification and recovery seam.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Add proposed DIVA `agent-diva-gui/src/components/console/TrajectoryPanel.vue` / test and `src/state/vivy-trajectory.ts` / test. Extend OBS-06's `src/api/vivy/observability.ts` and DN-1 contracts sequentially. Modify `src/components/SubAgentPanel.vue`, `src/composables/useSubAgents.ts` and existing tests only for verified child mappings; mount selected-session views in `ConsoleView.vue` / `App.vue` after the existing owner edits.

Wrapper `getSessionTrajectory(client: VivyClient,sessionId: string,limit?: number): Promise<TrajectorySession>` calls `trajectory/session`; defaults/bounds follow D4. Child wrappers use existing `child/list` DTOs frozen by DN-0, not guessed legacy SubAgent DTOs. Preserve tool/model typed IDs separately.

## Ordered tasks

- [ ] Add `vivy-trajectory.test.ts` cases for fixture render equivalence, stable row IDs after append, call_status active/cancelled/interrupted versus independent run_activity waiting state, nullable usage_evidence versus reported zero and recent-run-window label. No production path selects demo rows.
- [ ] Implement the typed snapshot wrapper and projection. Attach listener before initial read; merge/dedup by backend run/seq, keep only contiguous cursor, handle new runs via DN-2 discovery and gap via run/log/snapshot. Test duplicate/late/out-of-order/bridge-gap, reconnect and late response after session switch.
- [ ] Mount trajectory for the selected authoritative session. Show bounded context/tool details, sanitized errors, source/provider/model/timing/usage and parent/child/workflow references; do not render plaintext thinking or raw provider payloads.
- [ ] Wire child list/status/navigation after accepted DTO fixtures; unmounted SubAgentPanel must no longer rely on dead commands. Pending approval/cancel remains DN-2's interaction owner; the view does not infer a terminal or automatically decide an action.
- [ ] Test empty/unavailable/missing session, window detach/reopen, completed-only answer, listener cleanup and cancellation while an attempt is active. Do not declare full history pagination or workflow management complete.
- [ ] Commit `feat(trajectory): mount vivy activity in diva console`.

## Verification and handoff

`pnpm --dir agent-diva-gui exec vitest run src/state/vivy-trajectory.test.ts src/components/console/TrajectoryPanel.test.ts` plus affected SubAgent tests; full GUI `test` and `build`. Expected red: no mounted real trajectory, baseline child commands/identity mismatch. Expected green: snapshot/replay equivalence and all lifecycle/gap/cleanup assertions pass.

Real DN-2 package: run a safe tool follow-up, pending approval, child activity and cancellation; close/reopen the window and compare with backend Journal replay. If exact child DTO/notification acceptance is absent, keep that slice blocked and do not hide it as complete. Return row IDs, core/artifact pin, commit, tests and sanitized trajectory to OBS-09. Review focus: one notification owner, cursor recovery, child authority and honest statuses.
