# Wave C + Wave D Review Closure

## Scope

- Closed the remaining Wave C review checklist and `/api/health` benchmark CI gate.
- Hardened Wave D rate limiting retry semantics, compaction ordering, meta-compaction fallback, and session compaction compatibility coverage.

## What Changed

- `/api/logs` now counts malformed lines and unknown-shape events internally, emits warning visibility for schema drift, and has regression tests for malformed input, invalid cursors, and unknown event shapes.
- `/api/health` now has a benchmark-style CI gate test and `just` entrypoint via `health-benchmark-check`, wired into `just ci`.
- `RateLimiter` now treats invalid/non-positive refill rates as non-refilling buckets, returns `retry_after: None` when a wait cannot be determined, and keeps refill/retry math in pure helper functions with boundary tests.
- `ProviderTap` now preserves `Option<u64>` retry-after semantics instead of forcing unusable large values.
- `AgentLoop` now guarantees `budget check -> auto compaction -> persist -> build_messages -> provider call`, and overflow recovery guarantees `reactive compaction -> persist -> rebuild -> retry once`.
- `MetaCompactor` now preserves the newest summary under tiny budgets and has fixture coverage for critical fact retention.
- Session compaction serde coverage now explicitly tolerates unknown fields and future `schema_version` values.

## Impact

- Wave C review items are closed without changing `/api/logs` or `/api/health` response contracts.
- Wave D retains best-effort compaction behavior and public API shape while removing retry-after and ordering edge cases that could mislead callers or rebuild context from stale state.
