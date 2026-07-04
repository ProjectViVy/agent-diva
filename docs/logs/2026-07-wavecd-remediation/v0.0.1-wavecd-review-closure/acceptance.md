# Acceptance

1. Call `GET /api/logs` against an audit directory containing malformed JSON, valid audit events, and schema-drifted lines; confirm valid events are still returned and invalid cursor input reports an error payload without crashing the route.
2. Call `GET /api/health`; confirm readiness semantics are unchanged, and `just ci` now also runs the health benchmark gate.
3. Exhaust a zero-refill rate limiter bucket and confirm the surfaced provider error uses `retry_after: None` instead of an extreme numeric value.
4. Trigger auto compaction and reactive overflow compaction in tests; confirm provider calls observe the persisted post-compaction session snapshot and that overflow retries only once.
