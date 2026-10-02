# OBS-04 — Trajectory projection and VIVY live view Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Expose real attempts and run activity with stable IDs, useful states and replay recovery in VIVY's existing trajectory.
**Architecture:** Read Journal through the existing trajectory/session projection and share the existing client event owner.
**Tech stack:** Go projection/RPC, existing React trajectory components, JSON-RPC.
**Epic / requirements:** OBS-B / O4,O8
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-02: accepted call identity/lifecycle fixtures. Shares schema/API files with OBS-03; serialize edits without inventing a semantic dependency.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify VIVY `internal/runtime/{trajectory.go,trajectory_test.go}`, `internal/rpc/control.go` and its tests, `ui/src/lib/api.ts`, `ui/src/components/trajectory/{trajectory-session.ts,trajectory-types.ts,TrajectoryPanel.tsx,TrajectoryTimeline.tsx,TrajectoryLedger.tsx,TrajectoryDetailPanel.tsx}` and focused tests as needed. Production imports must not use `trajectory-demo-data.ts`.

Keep `Service.SessionTrajectory(ctx,sessionID,limit) (TrajectorySession,error)`; RPC stays `trajectory/session({session_id,limit?})`. Add v2 call_status/request_id/usage_evidence/run_activity/usage_state/watermark fields as [D4](architecture.md#d4--trajectory-as-a-live-journal-view), and advertise availability in capabilities. No historical cursor/page API is introduced in this round.

## Ordered tasks

- [ ] Add `TestTrajectoryMultipleCallsStableIdentity`: tool follow-up calls produce separate rows, latest usage replaces its own attempt, IDs remain unchanged after another event/rebuild and legacy replay remains meaningful.
- [ ] Add `TestTrajectoryActiveWaitCancelAndInterrupted`: active call is not failure/completed; run_activity shows waiting approvals/questions independently of completed call_status. Cancellation, failed setup and recovered open calls are distinct. Preserve legacy status/usage fields; v2 clients use nullable usage_evidence. A call finish cannot terminalize the run. Nil usage differs from reported zero.
- [ ] Fold child/parent/workflow references from existing durable events/run metadata and retain tool-call details/error flags. Clip detail at the current 8KiB boundary and do not expose plaintext reasoning or unbounded provider payloads.
- [ ] Add watermarks from the same read prefix used by the snapshot and has_older_runs from the existing window. Preserve default 20/max 50 behavior; test empty/missing session, truncation, older runs and cancellation. Avoid a watermark that claims events the projection did not see.
- [ ] Implement listener-before-snapshot merge through the existing RPC owner: dedup (run_id,seq), advance contiguous sequences only, gap → replay/snapshot, reconnect → reconcile. Test duplicate/late/gap notifications, new run discovery, completed-only answers and selection/unmount listener cleanup.
- [ ] Update VIVY statuses, unknown usage and recent-run window labels; remove demo imports from production wiring. Commit `feat(trajectory): project durable call activity and live state`.

## Verification and handoff

`go test ./internal/runtime ./internal/rpc`; VIVY UI `pnpm --dir ui test`, `typecheck`, `build` with the explicit `pnpm --dir ui` prefix on each command; then required `just ci`. Expected red: baseline positional IDs and active/error assumptions fail new tests. Expected green: fixture projection and live UI reconcile to identical state after replay/reconnect, with no leaked listeners.

Return actual RPC/capability fixtures, stable-ID and watermark rules, source commit, tests and a sanitized waiting/cancel/child trajectory transcript to OBS-07. Review focus: truthful lifecycle, no full-history claim, stable identity, snapshot watermark and event-gap recovery.
