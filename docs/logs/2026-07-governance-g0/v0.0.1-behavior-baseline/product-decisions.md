# G0 Product Decisions

These decisions are the G1 compatibility contract. Implementations must fail closed
when a request cannot be classified.

## Memory classes

| Class | Retention and sensitivity default | Forget / supersede behavior | Write policy |
| --- | --- | --- | --- |
| Session fact | session-scoped unless promoted | expires with session; promotion creates a proposal | automatic session evidence only |
| Long-term fact | durable, sensitivity inherited from source | tombstone or superseding record; retain audit | proposal; human review when sensitive or disputed |
| Preference | durable and user-editable | latest approved value supersedes | explicit user edit or proposal |
| Commitment | durable, high impact | explicit cancellation/compensation; never silent overwrite | per-change human approval |
| Relationship | durable, sensitive | explicit correction/tombstone | per-change human approval |
| Identity | durable, highest sensitivity | explicit correction with audit | per-change human approval |
| Historical summary | bounded derived artifact | regenerate or supersede; source evidence retained | proposal or deterministic report pipeline |
| Temporary recall | turn/session scoped | discarded after budget window | read-only injection; never authority |

Pending, rejected, expired, tombstoned, or untrusted records are excluded from the
default prompt.

## Human decisions

| Decision | Scope | Expiry | Revocation / rejection behavior |
| --- | --- | --- | --- |
| `approve_once` | exact request hash, resource scope, policy version, one execution | consumed on use or request TTL | cancellation prevents execution; content change invalidates |
| `approve_session` | capability and bounded resource scope in one session | session end or configured TTL | revocable; restart does not widen scope |
| `approve_rule` | exact validated rule and resource constraints | persisted expiry or explicit disable/delete | revocable; rule revision invalidates prior match |
| `edit_and_approve` | edited content hash and one bounded request | request TTL | original and later mutations are unapproved |
| `reject` | exact pending request | terminal for that revision | executor receives typed denial; no side effect |
| `cancel` | pending or suspended request | immediate | cancellation propagates to waiter and audit |

Only an authenticated user controlling the owning workspace/session may decide.
Manager validates this boundary; hidden GUI controls are not authorization.

## Autonomy levels

| Level | Meaning | Memory writes | Tool side effects |
| --- | --- | --- | --- |
| L0 | read-only recommendation | none | inspect only |
| L1 | low-risk automatic action | low-risk proposal generation; no authority promotion | reversible, scoped action allowed by explicit policy |
| L2 | session-authorized action | only within approved class/scope | valid session receipt required |
| L3 | per-action approval | high-risk proposal requires exact receipt | exact once receipt required |
| L4 | prohibited | identity/authority mutation with missing evidence denied | destructive or unknown action denied |

User presence is context, not authorization. Offline high-risk work queues or fails;
it is never auto-approved.

## Frozen precedence and failure policy

Hard prohibition > explicit rejection > resource/mode restriction > valid scoped
authorization > safe default. Unknown capability, risk, state, source, receipt,
resource scope, or policy version is denied. Approval is bound to the request hash,
policy version, scope, expiry, and permitted execution count.

## P0/P1 decisions

All P0/P1 semantics required for G1 are resolved above. New ambiguity discovered
during implementation is owned by the current delivery owner, recorded in
`TODOLIST.md`, and blocks the affected production integration until an explicit
decision is added here. Default while blocked is fail-closed.
