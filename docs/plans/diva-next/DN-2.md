# DN-2 — First usable Rust-free chat loop

- **Epic:** A · **Requirements:** R-2, R-3 · **Outcome:** browser → real VIVY → configured model → visible/persisted answer; approval gates affect real execution; no Rust Manager path in process/network inspection.
- **Authoritative design:** issue #13 DN-P1 §4 · **Baseline:** `0fd005a1` · **Status:** Planned · **Predecessor:** DN-1 client + session projection · **Index:** [index.md](index.md)
- **Files:** `agent-diva-gui/src/App.vue`, `src/components/ChatView.vue`, approval components, `src/api/approvals.ts`, `src/api/planning.ts`, `src/utils/streamingMessages.ts`, DN-1 state module. **Escalate:** approval/interaction semantics absent from the pinned VIVY contract.

## Prerequisites / contracts

- Consumes DN-1 client, contracts, and session projection (accepted evidence required).
- VIVY run/interaction IDs bind every decision; denial/timeout/pending-approval recovery per the pinned contract.

## Tasks

- [ ] Replace send/history/session/stop and reasoning/tool event handling with native VIVY turn/run/session operations.
- [ ] Move affected orchestration out of `App.vue` into the DN-1 state seam; preserve presentation; no unrelated refactoring.
- [ ] Bind approval/question decisions to exact VIVY interaction IDs and supported review revisions; map errors visibly.
- [ ] Remove the browser mock chat path and old business listeners for migrated flows.
- [ ] Verify completed-only output, cancellation, refresh/reconnect, pending approval recovery, denial/timeout, follow-up turns.

## Verification

- `pnpm --dir agent-diva-gui test` and `build` pass.
- Manual smoke: real model answer visible and persisted; approval gate blocks/releases real execution; `ps`/network inspection shows no Rust Manager/SSE path.
- A deterministic provider fixture is acceptable development evidence but does not substitute the live-model gate.

## Evidence to supervisor

Smoke recording/log, test results, confirmation that no legacy executor is reachable for migrated flows.
