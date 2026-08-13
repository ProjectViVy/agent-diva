# Verification

## Automated checks

- `cargo test -p agent-diva-laputa` — passed; performance-only 10k tests remain
  explicitly ignored in the normal suite.
- `cargo test -p agent-diva-manager --lib` — 77 passed.
- `governance_snapshot_is_payload_free_and_tracks_latency_maxima` — passed.
- `health_response_uses_readiness_contract` — passed.
- `stale_governance_errors_increment_payload_free_metric` — focused gate.
- `cargo check -p agent-diva-manager --all-targets` — passed.
- `cargo fmt --all -- --check` — required before commit.

The serialization assertion permits only `_total` and `_max` metric keys.
Correlation completeness is reduced to counters; correlated identifiers remain
in their existing governed operation/audit records and are not added to health.
