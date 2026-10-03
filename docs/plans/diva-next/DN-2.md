> 2026-10-03 scope amendment: [DN-C2](p0-design.md) and [index.md](index.md)
> take precedence over historical P0-D1 host/speech/import/acceptance premises.
> Tasks/evidence below retain their original scope and artifact pins. New
> closure work is not proved by historical implementation or acceptance.

# DN-2 — Real desktop chat and approvals Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** Drive real sessions, streaming, approval/question decisions and cancellation from the existing Vue UI.
**Architecture:** Keep presentation, replace the orchestration authority with DN-1 and real VIVY operations.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Files and contracts

Modify existing `agent-diva-gui/src/App.vue`, `src/components/ChatView.vue`, `src/api/{approvals,planning}.ts`, `src/utils/streamingMessages.ts`, `src/components/planning/planExecutionState.ts` and associated tests; extend DN-1 state tests. Exact remaining approval consumers are enumerated in DN-0.

Consumes DN-1 client/projection and DN-0 mappings. Produces the user-visible core chain for DN-P and a verified domain mutation pattern for DN-3/DN-4. session/messages supplies history; turn/start returns the authoritative run; run/cancel requests cancellation. Planning reviews use their actual frozen work/action contract, not an assumed approval alias.

## Ordered tasks

- [ ] Add behavior tests for send/history/switch-session, complete-only text, reasoning/tool progress, cancellation racing completion, and an ambiguous send timeout. Use the redacted real protocol fixture for contract cases.
- [ ] Replace send_message, get_sessions, get_session_history, stop_generation and legacy core listeners with the frozen session/turn/run methods. Remove browser mock chat success and duplicate App.vue state orchestration for migrated flows.
- [ ] Add approval/question tests for approve/deny/expire, stale review revision, double decision and reopening while pending. Bind decisions to backend IDs and display backend rejections; resolve statuses through snapshots/events.
- [ ] Migrate planning/work surfaces only against verified schemas; leave any required missing capability recorded as blocked, never fake-complete. Delete the migrated legacy DTOs and stream startup commands.
- [ ] Run packaged chat with a real configured model and a policy-gated safe test tool in a temporary workspace; demonstrate the operation cannot happen before approval and is not executed after denial/cancellation.

## Verification

`pnpm --dir agent-diva-gui test`; `pnpm --dir agent-diva-gui build`; native live-model chain with safe test tool and Journal/session readback. Expected: text and persisted history agree, decisions govern actual execution, cancellation reaches an authoritative terminal, reopened window reconstructs the same run without resending it. Return transcript and removed-command ledger rows. No production credentials in evidence.
