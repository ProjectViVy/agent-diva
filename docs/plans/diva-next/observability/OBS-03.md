# OBS-03 — Usage coverage, replacement and VIVY dashboard Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Make token totals explain their evidence and price completeness while preserving the current stats/tokens contract.
**Architecture:** Canonical Journal event fold feeds both SQL readers, RPC aggregates and the existing VIVY UI.
**Tech stack:** Go, SQLite/PostgreSQL, JSON-RPC, existing React/Vitest UI.
**Epic / requirements:** OBS-B / O3,O8
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** OBS-02: accepted schemas and persisted-event fixtures. No DIVA ABI dependency for backend/VIVY UI verification.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify VIVY `internal/storage/contracts.go`, `internal/storage/{sqlite,postgres}/token_usage.go`, `internal/rpc/tokenstats.go` and existing/proposed tests; add proposed shared `internal/storage/usage_projection.go` / `usage_projection_test.go` for a pure fold. Update `ui/src/lib/api.ts`, `ui/src/components/demo/TokenStatsPanel.tsx` and focused tests. The “demo” directory name is not evidence of a mock backend.

Preserve `TokenUsageStore.ListModelUsage(ctx,sinceUnixMilli)` and `SessionTokenUsageStore.ListSessionModelUsage(ctx,sessionID)` signatures, extending UsageRow additively with call identity, state and known-bucket evidence. Wire `stats/tokens` adds projection_version/coverage exactly from [D3](architecture.md#d3--honest-usage-projection-and-wire-compatibility). No new ledger or DDL.

## Ordered tasks

- [ ] Add `TestUsageProjectionLatestSamplePerAttempt`: two cumulative samples 10 then 15 contribute 15; another attempt contributes separately; duplicate replay is unchanged. Legacy samples retain (run,seq) behavior, never retroactive fake IDs.
- [ ] Test observed attempts with nil usage, real zero usage, active streams, failed/cancelled partial usage, interrupted finish, optional unknown buckets, inconsistent/negative samples, normalization_partial evidence and unsafe addition. Assert coverage partitions and row/global known flags.
- [ ] Implement one pure fold; SQL readers use it with equivalent canonical event selection. Select new calls by start time and load replacement samples through the snapshot watermark. Test midnight/timezone boundaries and a usage sample arriving after the period boundary; keep legacy sample-time compatibility.
- [ ] Add SQLite and PostgreSQL parity cases for main/child/summary route attribution and reopened stores. Unknown summary provider/model is not priced as main. If an index becomes necessary, stop and produce paired append-only migration evidence before continuing.
- [ ] Extend RPC snapshots/aggregates and tests: preserve period union and timezone sign, distinguish usage reports from observed attempts, reason/cache subsets, estimated pricing and incomplete known subtotal. Unknown cost must remain cost_known=false even if existing numeric compatibility fields are zero.
- [ ] Update VIVY UI wire types/rendering: scope/coverage labels, “—” for unknown price/buckets, active loading/error/empty states. Coalesce invalidations from usage/finish/terminal and reconnect; do not add a second permanent event subscription owner.
- [ ] Commit `feat(stats): expose token usage coverage and attempt totals`.

## Verification and handoff

`go test ./internal/storage/... ./internal/rpc`; run `go test -count=1 -v ./internal/storage/postgres` with `VIVY_POSTGRES_TEST_DSN` set to a disposable local database by the runner (never print its secret value). The backend conformance tests currently skip without that variable; skipped integration is not parity evidence. VIVY UI: `pnpm --dir ui test`, `pnpm --dir ui typecheck`, `pnpm --dir ui build`; stage-UI prerequisites must succeed rather than be bypassed. Finish required `just ci`.

Expected red: duplicate/partial/absent samples and period-boundary tests contradict baseline totals/coverage. Expected green: both stores and RPC agree with the fixture oracle; UI cannot turn missing into zero or estimate into invoice. Hand off exact stats v2 fixtures (complete/partial/legacy/empty/unknown-price), source commit and commands to OBS-06. Review focus: period selection, known-bucket propagation, price attribution and compatibility.
