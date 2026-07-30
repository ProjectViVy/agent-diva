# Acceptance

GMH-42 is accepted when:

- a new allow/deny decision records latency and human wait once;
- decision retries do not double-count a completed decision;
- stale/expired/conflicting/consumed receipts increment a payload-free metric;
- successful typed apply and rollback record operation and correlation status;
- Manager health exposes only aggregate numeric fields;
- existing proposal, receipt, audit and GUI success DTOs remain compatible;
- Laputa and Manager suites pass.

All acceptance items are automated. Real desktop observation remains in the
deferred final G2D+ gate.
