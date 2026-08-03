# Acceptance

## Automated stage acceptance

1. The global badge reports the exact loaded Pending count and opens a keyboard-addressable drawer.
2. Drawer filters cover domain, status, and source session; cards sort severe risk first and earliest expiry second.
3. Cards show textual domain/status/risk, capability, scope, version, TTL, evidence count, summary/diff, and explicit grants.
4. A stale response refreshes authoritative detail. An outcome-unknown mutation disables all repeat actions and exposes refresh-only recovery.
5. Reconnected or duplicate events do not duplicate cards, and events older than the known request version are ignored.
6. Chat consumes the unified projection and does not display a second legacy Command/Plan approval for the same active request.
7. Memory edit/apply navigation opens the owning Evolution proposal rather than accepting an arbitrary generic edit body.

## Deferred human acceptance

No manual action is requested now. The final M3 smoke must visually confirm the badge/drawer/inline surfaces, understandable copy, keyboard focus loop, non-color status, edit-and-approve revision sequence, disconnect/reconnect, and committed-but-refresh-failed presentation in both debug and release desktop paths.
