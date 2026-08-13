# Release

This is a backward-compatible diagnostic DTO extension:
`memory.governance_metrics` is added to Manager health responses. Existing
health and proposal success fields are unchanged.

The metrics are process-local operational aggregates. Durable evidence remains
the governance ledger, typed apply journal, proposal, changelog, audit and
rollback records.

No push, external API call, desktop key read or production deployment is part
of this slice.
