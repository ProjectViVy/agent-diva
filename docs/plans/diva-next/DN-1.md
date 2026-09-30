# DN-1 — VIVY client and Vue projection Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** Create a transport-neutral typed caller with authoritative replayable Vue state.
**Architecture:** The desktop transport uses the accepted thin-shell commands; state projects VIVY snapshots/events instead of legacy deltas.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Files and contracts

Proposed: `agent-diva-gui/src/api/vivy/{client,contracts,transport}.ts`, corresponding `*.test.ts`, `agent-diva-gui/src/state/vivy-session.ts` and its test. Modify existing `src/api/capabilities.ts`. package/lock edits only if required. No desktop WebSocket bootstrap or browser dev-proxy requirement remains.

Consumes DN-0 frozen wire schemas and DN-5 verified transport. Proposed client interface: `call<T>(method: string, params: unknown): Promise<T>`; `onEvent(handler: (event: VivyNotification) => void): () => void`; `close(): void` detaches frontend listeners only. The desktop transport handles request_id and envelope; method payloads remain VIVY-defined. Produces backend-ID-based session/run/interaction projection consumed by pages.

## Ordered tasks

- [ ] Add transport tests for correlation, RPC error preservation, incompatible ABI/capability, bridge close, and timeout after a mutation was accepted. Ensure the latter is never automatically retried.
- [ ] Implement the thin caller and typed contracts from the frozen fixture; isolate all business-facing Tauri imports in this transport. Native presentation calls use desktop-host.ts.
- [ ] Add reducer tests for repeated/late/out-of-order notifications, missing sequence, terminal-only answer, two windows and pending interaction snapshots. Implement listener-before-snapshot recovery and (run_id,seq) dedup from P0-D1. Never advance the contiguous cursor across a gap.
- [ ] Replace the static capabilities ledger's Manager authority labels with negotiated VIVY capability state plus DN-0 dispositions. Distinguish absent, disconnected, failed and available; no fake-success fallback or localStorage domain authority.
- [ ] Run a real native initialize/session-read/subscribe/log smoke through the DLL and shell. Tests may inject a transport, but the product path cannot select the old backend.

## Verification

`pnpm --dir agent-diva-gui test`; `pnpm --dir agent-diva-gui build`. Expected focused reducer/transport assertions and real native smoke both pass. Return core fixture version, recovery transcript and tests. Review focus is the five adverse cases in the shared design; state reducer owns ordering/gap and ambiguous mutation tests.
