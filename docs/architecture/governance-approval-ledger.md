# Governance Approval Ledger

## Decision

GMH-12 introduces `agent-diva-core::governance::SqliteGovernanceLedger` as the
future durable authority for cross-domain approvals. It is deliberately not
wired into production Plan, Sandbox, Manager, AgentLoop, or GUI paths in this
iteration.

Existing `planning.db` approvals, the process-local command approval
coordinator, and `execpolicy.toml` remain unchanged. Explicit Plan and Sandbox
adapters prove that legacy contracts can be wrapped without a flag-day
migration.

## Storage model

The `governance_ledger_events` table stores immutable JSON events with unique
`(request_id, version)` and `idempotency_key` constraints. SQLite triggers reject
updates and deletes. Current state is replayed from the ordered event stream;
there is no mutable current-state table.

The stored `ApprovalRecord` excludes the generic domain payload. Evidence is
reduced to ID, source, URI, hash, and timestamp; excerpts are removed. Commands,
Plan Markdown, Memory values, and other raw sensitive content remain in their
owning domain stores and are bound through `ContentDigest`.

## State machine

Allowed transitions:

- `requested → allowed | denied | revoked | expired`
- `allowed → consumed | revoked | expired`

Denied, revoked, consumed, and expired are terminal. An approve-once receipt can
be consumed exactly once and is revalidated against request ID, content digest,
capability, resource, and policy version before consumption.

Pending and allowed states derive expiry at query time from the request and
receipt TTL even when no explicit expired event has been appended. The explicit
expire operation materializes that derived terminal state for audit.

## Concurrency and idempotency

Every state-changing operation supplies an expected version and idempotency key.
The first insert for a request version wins. A retry with the same key and
identical operation returns the derived state; reuse with different content
fails with `idempotency_conflict`.

Concurrent allow/allow and allow/deny contenders can commit only one version.
Once a deny or revoke is committed, later allow and consume transitions fail.

## Threat model

- Raw payload persistence is forbidden; only an irreversible digest is stored.
- Unknown or invalid request/receipt fields fail before ledger append.
- Expired requests, expired receipts, future/out-of-window decisions, stale
  versions, altered digests, altered resources, and altered policy versions do
  not authorize.
- Database corruption that introduces missing versions, duplicate request
  events, or illegal transitions fails replay closed with a persistence error.
- Ledger persistence failure never returns an allowed state.
- Manager/GUI visibility and user presence are not authorization.

Revocation cannot undo an already consumed side effect. Compensation and
domain-specific rollback remain responsibilities of later runtime integration.

## GMH-30A coordination boundary

`agent_diva_core::governance::ApprovalCoordinator` is the shared domain-neutral
entry point over the policy evaluator and ledger. It returns `Allowed` or
`Denied` without manufacturing approval history, and appends a payload-free
`Pending` record only when policy requires a human decision. Plan, Sandbox, and
Memory keep ownership of their payloads and adapt them into the generic request
envelope.

The coordinator also exposes ledger-backed decision and state queries. Receipt
consumption, restart recovery, cancellation, and timeout orchestration remain
GMH-30B work; Manager transport contracts and GUI/CLI presentation remain
GMH-31 through GMH-33.
