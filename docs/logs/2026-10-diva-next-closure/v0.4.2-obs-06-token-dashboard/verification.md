# v0.4.2 — OBS-06 verification

Environment: Linux VM, Node v24.19.0, pnpm 10.33.2, happy-dom vitest.

- `pnpm exec vitest run src/api/tokenStats.test.ts
  src/state/vivy-observability.test.ts` — 21 tests pass: verbatim v2
  fixture coverage mapping (empty/complete/partial/legacy, partition
  sums, reported_calls not a partition, unknown buckets, hidden
  retries), costDisplay unknown-vs-zero, preview issues no RPC,
  initialize-derived connection + failure → unavailable, bridge
  gap/lost degradation, in-flight coalescing, param-switch and
  reconnect response fencing, failed refresh keeps stale totals,
  run/event staleness.
- `just gui-test` — 481 tests, 57 files, all pass.
- `just gui-build` — vue-tsc + vite build green.
- Producer DTOs verified verbatim against
  `agent-vivy/internal/rpc/tokenstats.go` and the DN-0C fixture
  (`closure-chat-obs.json` requests_responses[47], projection-v2
  assertion).

Environment limits: no Tauri runtime — the turn→totals→disconnect→
reopen smoke is recorded as owner acceptance; preview-shell behavior is
proven by unit test (zero RPCs, no host metadata).
