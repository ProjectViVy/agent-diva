# DN-M — P0-B semantic migration closure Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** Prove all required old frontend behavior has migrated with no reachable dead business seams.
**Architecture:** Reconcile the complete ledger with accepted core, operational, companion and optional-domain replacements.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Files and contracts

Consumes DN-3/DN-4/DN-6 accepted domain evidence; DN-0 ledger is separately required as the completeness contract. Modify `agent-diva-gui/src/api/desktop.ts`, leftover source consumers identified by the ledger, `src/api/capabilities.ts`, and `scripts/ci/check_vivy_backend_boundary.py`; proposed `scripts/ci/check_legacy_frontend_calls.mjs`. Final mapping/evidence stays in `backend-separation-contracts.md` and the acceptance log, not a second status tracker.

## Ordered tasks

- [ ] Reconcile every legacy source row against its accepted replacement and tests: behavior, request/response/error conversion, persistence authority, event ordering/recovery and user-visible failure states. Deletion without accepted disposition is a failure.
- [ ] Remove obsolete business exports/DTOs from desktop.ts and residual callers in App.vue, settings, voice, token statistics, notebooks, masks and composables. Keep valid native shell operations in desktop-host.ts and VIVY transport imports in api/vivy/transport.ts.
- [ ] Add a TypeScript-AST/import-boundary gate (reuse the existing TypeScript dependency): reject legacy business command strings, aliases/dynamic business invokes, Manager HTTP/SSE and direct provider credential bypasses; allow only the frozen native/transport entrypoints. Include negative fixtures showing aliased and wrapped legacy calls fail. Historical fixtures/docs are excluded deliberately.
- [ ] Run per-domain behavior/readback/restart evidence from DN-3/4/6 and core evidence from DN-2. Check missing/error/denied capability responses do not become success or stale local state.
- [ ] Close P0-B only when every required row has accepted evidence or explicit user-approved retirement. A blocked backend action or missing required speech/persona capability keeps this Story blocked; a green static scan cannot waive it.

## Verification

`node scripts/ci/check_legacy_frontend_calls.mjs` (proposed gate); `python scripts/ci/check_vivy_backend_boundary.py`; `pnpm --dir agent-diva-gui test`; `pnpm --dir agent-diva-gui build`; installed per-domain readback/restart scenarios. Expected zero unclassified production sites and zero reachable legacy business paths, not zero @tauri-apps imports. Review focus: wrappers evade scan, removed feature counted as migrated, legacy event drives new state, credentials bypass VIVY, required unavailable feature hidden. Return reconciled ledger and acceptance evidence; scope changes go to user review.
