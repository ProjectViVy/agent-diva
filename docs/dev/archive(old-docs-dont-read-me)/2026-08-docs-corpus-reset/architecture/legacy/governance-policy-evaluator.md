# Governance Policy Evaluator

## Scope

GMH-11 adds a deterministic policy evaluator to `agent-diva-core::governance`.
It evaluates an already-classified request and returns a typed decision, reason
code, constraints, and the request's bounded evidence references. It performs no
I/O, persistence, policy loading, payload hashing, authentication, or side
effect.

Runtime integration, policy configuration, approval consumption, revocation,
and the durable ledger remain GMH-12 or later work.

## Precedence

The evaluator applies the frozen order:

1. prohibited risk or L4 autonomy;
2. explicit user denial;
3. invalid context, capability/resource mismatch, or domain restriction;
4. valid scoped authorization;
5. autonomy-specific safe default;
6. human approval requirement.

Unknown subjects, capabilities, resources, risk classes, autonomy levels,
restriction kinds, receipt fields, and decisions fail closed. A malformed
request yields `deny/invalid_request`; malformed supplied authorization yields
`deny/invalid_authorization`.

## Capability and resource matrix

| Capability | Compatible resource |
| --- | --- |
| `inspect` | any known resource |
| `workspace_write` | `workspace`, `workspace_path` |
| `command_execute` | `command` |
| `network_access` | `network` |
| `mcp_invoke` | `mcp_server` |
| `spawn` | `agent` |
| `schedule` | `schedule` |
| `plan_mutate`, `plan_execute` | `plan` |
| `memory_propose`, `memory_apply` | `memory` |
| `policy_manage` | `policy` |

Every other pair is denied before an authorization is considered.

## Autonomy contract

- L0 allows inspect only; other capabilities are mode-restricted.
- L1 safely defaults only low-risk inspect, Plan drafting, and Memory proposal.
  Other actions require a human unless a valid authorization is supplied.
- L2 requires an unexpired session or validated-rule authorization.
- L3 requires an exact approve-once authorization.
- L4 denies all actions.
- Critical actions require an exact approve-once authorization at any
  authorization-capable level. Prohibited actions cannot be authorized.

An approve-once receipt matches the exact request, digest, policy version,
capability, and resource. Session authorization additionally requires the
request resource to be bound to the correlated session. Rule authorization is
bound to capability, resource, policy version, and the digest of its validated
constraints. Expired receipts do not authorize.

## Domain boundary

Plan, Sandbox, and Memory remain responsible for deriving risk, resource scope,
mode/resource restrictions, payload digest, and evidence. Manager remains
responsible for authenticating human decisions. The evaluator does not treat
user presence or a visible GUI control as authorization.
