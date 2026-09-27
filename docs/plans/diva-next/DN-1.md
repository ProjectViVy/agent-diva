# DN-1 — Native VIVY protocol client and Vue state seam

- **Epic:** A · **Requirements:** R-2, R-3 · **Outcome:** browser client connects to a real pinned VIVY via `/rpc/bootstrap` + authenticated WebSocket JSON-RPC with `initialize`/capability discovery; Vue state projection exists.
- **Authoritative design:** issue #13 DN-P1 §4 (Connection/State/Submission rules) · **Baseline:** `0fd005a1` · **Status:** Planned — becomes Ready when DN-0's `backend-separation-contracts.md` lands · **Index:** [index.md](index.md)
- **In scope:** one connection lifecycle, state seam, dev proxy config, focused tests. **Out of scope:** migrating any business page (DN-2+); copying the React store. **Escalate:** missing transport contract in the pinned VIVY.

## Prerequisites / contracts

- Consumes from DN-0: pinned VIVY artifact/revision; bootstrap/RPC/replay contract summary; named unresolved contracts.
- Produces: `src/api/vivy/client.ts`, `src/api/vivy/contracts.ts`, `src/state/vivy-session.ts` (proposed paths); modified `src/api/capabilities.ts`, `agent-diva-gui/vite.config.ts`, `agent-diva-gui/package.json`.
- Rules that bind the implementation: VIVY session/run/interaction IDs authoritative; replay cursor + dedup; subscription IDs are connection-scoped; never auto-resend a mutating call after ambiguous disconnect; missing capability = explicitly unavailable.

## Tasks

- [ ] Implement bootstrap/connection lifecycle per the pinned contract: request correlation, connection errors, negotiated capabilities.
- [ ] Add Vue session/run state projection with replay cursor and deduplication; backend truth never in localStorage (theme/layout prefs exempt per issue §3).
- [ ] Configure the local dev proxy/origin deliberately in `vite.config.ts`; browser startup must reach actual VIVY.
- [ ] Coordinate shared protocol extraction upstream if needed; thin typed caller + Vue adapter only, not a fork of VIVY wire definitions.

## Verification

- Real VIVY `initialize` + session-read + replay smoke.
- Deterministic projection tests: invalid/expired auth, reconnect, duplicate/out-of-order delivery, incompatible capability, ambiguous mutation outcome.
- Commands: `pnpm --dir agent-diva-gui test`; `pnpm --dir agent-diva-gui build` (vue-tsc + Vite). Use the project-pinned package-manager version.

## Evidence to supervisor

Smoke output + test results; list of capabilities reported unavailable.

## Notes

Mocks alone do not pass this Story. A missing transport contract blocks it (Blocked, not improvised).
