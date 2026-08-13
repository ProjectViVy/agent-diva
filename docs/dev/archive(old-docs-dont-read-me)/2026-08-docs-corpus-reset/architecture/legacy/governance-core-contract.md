# GMH-10 Governance Core Contract

## Decision

Plan, Sandbox, and Memory governance share one domain-neutral envelope in
`agent-diva_core::governance`. Each domain retains its own payload and composes
it through `ApprovalRequest<P>`; the core does not define a universal business
payload enum.

The contract is additive. Existing Evolution, Plan, and Sandbox approval types
remain unchanged until the GMH-12 ledger migration introduces explicit
adapters. GMH-10 does not change persistence, transport, policy evaluation, or
runtime execution.

## Contract boundaries

- `GovernanceSubject`, `Capability`, `ResourceScope`, and `RiskClass` describe
  who wants to do what, against which bounded resource, at what risk.
- `ContentDigest` binds decisions to caller-computed immutable content without
  storing commands, file contents, prompts, or other sensitive payloads in the
  receipt.
- `AuditCorrelation` binds the request to request, turn, session, and optional
  trace identifiers.
- `ApprovalReceipt` binds the decision to the request ID, digest, policy
  version, capability, and resource scope.
- Existing `EvidenceRef` and `EvidenceSource` are re-exported rather than
  duplicated.

## Threat model and failure policy

The protected property is that an approval for one revision, policy, capability,
or resource cannot authorize another.

- Unknown subject, capability, resource, risk, digest algorithm, decision, or
  grant values deserialize into an explicit `Unknown` variant and fail
  validation.
- Empty stable identifiers and policy versions are invalid.
- Request and receipt expiry must be strictly later than creation or decision
  time.
- `validate_approve_once` requires an allow decision and a once grant, then
  compares the request ID, digest, policy version, capability, and full resource
  scope.
- Payload hashing remains the caller's responsibility. Callers must use a
  canonical representation and recompute the digest after every edit.
- Domain policy precedence, durable consumption, concurrency, revocation, and
  restart recovery are intentionally deferred to GMH-11 and GMH-12.

## Compatibility and migration

This module adds only new Rust exports and Serde contracts. No existing JSON
shape, database schema, configuration, HTTP route, SSE event, or GUI DTO changes.
Later adapters must translate existing Plan and Sandbox approvals explicitly;
they must not silently reinterpret legacy approval records as generic receipts.
