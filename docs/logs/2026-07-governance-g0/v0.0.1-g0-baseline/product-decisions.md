# G0 Product Decisions

## Memory classes

| Class | Default | Forget/supersede | Write decision |
| --- | --- | --- | --- |
| Session fact | session-scoped | expires unless proposed | evidence only |
| Long-term fact | durable, source-sensitive | tombstone/supersede with audit | proposal; review if sensitive |
| Preference | durable, user-editable | latest approved value | explicit edit or proposal |
| Commitment | durable, high impact | cancellation/compensation | per-change approval |
| Relationship | durable, sensitive | correction/tombstone | per-change approval |
| Identity | durable, highest sensitivity | audited correction | per-change approval |
| Historical summary | bounded derived artifact | regenerate/supersede | proposal or deterministic report |
| Temporary recall | turn/session | discard after budget window | read-only, never authority |

Pending, rejected, expired, tombstoned, and untrusted records are excluded from the
default prompt.

## Human decisions

- `approve_once`: exact hash/scope/policy version, one use, request TTL; mutation invalidates.
- `approve_session`: bounded capability/resource until session end or TTL; revocable.
- `approve_rule`: exact validated rule/resource constraints; expiry, disable, delete, or revision invalidates.
- `edit_and_approve`: edited hash and one request; later mutation is unapproved.
- `reject`: terminal for the exact revision; no side effect.
- `cancel`: terminates pending/suspended work and propagates audit/cancellation.

Manager authenticates the deciding user and workspace/session ownership. Hidden GUI
controls are not authorization.

## Autonomy and precedence

- L0 inspect/recommend only.
- L1 reversible low-risk actions explicitly allowed; Memory only proposes.
- L2 valid session receipt required.
- L3 exact per-action receipt required.
- L4 prohibited or unclassified action denied.

Hard prohibition > rejection > resource/mode restriction > valid scoped
authorization > safe default. Presence is context, not permission. Unknown
capability, risk, state, source, scope, receipt, or policy version is fail-closed.
