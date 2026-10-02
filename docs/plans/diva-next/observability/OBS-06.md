# OBS-06 — DIVA token dashboard and truthful console Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Replace dead token commands and fabricated console status/save success with authoritative stats and negotiated host state.
**Architecture:** One typed stats wrapper uses DN-1 client; DN-2 session/connection projection drives the mounted console.
**Tech stack:** Vue 3, TypeScript, Vitest/Vite, accepted Tauri shared-library transport.
**Epic / requirements:** OBS-C / O3,O6,O7
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-03: accepted stats v2 fixtures; DN-2: accepted native client/session/recovery path. Fixture preparation can start earlier; real predecessor acceptance cannot be mocked.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify DIVA `agent-diva-gui/src/api/{tokenStats.ts,tokenStats.test.ts,capabilities.ts}`, `src/components/{ConsoleView.vue,console/TokenStatsPanel.vue}`, existing/new tests and scoped `App.vue` wiring. Add proposed `src/api/vivy/observability.ts`, typed DTOs in DN-1's `src/api/vivy/contracts.ts`, and `src/state/vivy-observability.ts` / tests. No alternate Tauri business caller.

Proposed wrapper `getTokenUsage(client: VivyClient,params: TokenUsageParams): Promise<TokenUsageSnapshot>` calls `stats/tokens` once. Params and response keep VIVY snake_case; D3 owns the wire. Local `ObservabilityConnection` is `{state:'connecting'|'connected'|'disconnected'|'unavailable',generation_id?:string,protocol_version?:string,reason?:string}`; project it from accepted DN-1 transport + initialize/ABI metadata, not a new health RPC or /healthz probe. If DN-1 has not exposed this metadata/state, its acceptance output must be amended before this Story becomes Ready.

## Ordered tasks

- [ ] Add fixture-backed failures: one call supplies totals/models/providers/timeline/sessions; legacy seven-call Promise.all is gone; unknown price/missing usage/legacy scope do not render zero or invoice. Check timezone sign, period selection, request_count “usage reports” label and observed count.
- [ ] Implement the typed wrapper and replace tokenStats legacy callers/DTO transformations. Do not preserve unsupported realtime TPS/cache-write metrics as zero. Remove each obsolete command from the capability ledger only after mapping and test evidence.
- [ ] Make refresh use one coalesced in-flight snapshot request; usage/finish/terminal/reconnect invalidate it. Test overlapping refreshes, out-of-order responses, disconnected/empty/error state, stale data labeling and unmount cleanup.
- [ ] Replace preview stopped gateway with unavailable and native runtime connection metadata. Remove legacy gateway start/stop/process ownership semantics. Native Quit/restart behavior is owned by DN-5, not a stats button.
- [ ] Remove mock config loads and fabricated save timestamp. Show unavailable until DN-3's actual per-domain settings API is connected; do not create a whole-config compatibility alias. Tests assert unavailable previews never report successful mutations.
- [ ] Update console layout/mounting without touching unrelated settings/persona. Commit `feat(console): connect token statistics to vivy`.

## Verification and handoff

Focused: `pnpm --dir agent-diva-gui exec vitest run src/api/tokenStats.test.ts` plus the new named console/state test files. Broad: `pnpm --dir agent-diva-gui test`, `pnpm --dir agent-diva-gui build`. Expected red: fixtures expose deleted commands and false preview success. Expected green: mappings/render states and request/listener cleanup pass.

Then run the accepted DN-2 native package with a temp data root: model call changes dashboard totals, disconnect shows unavailable/stale, reopen refreshes persisted totals. Browser-only fixtures are not this acceptance. Return mapping row IDs, source commit, real core/artifact identity, tests and sanitized before/after snapshots to OBS-09. Review focus: no legacy command facade, scope/unknown-cost rendering and real connection authority.
