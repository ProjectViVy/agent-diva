# GMH-32 Approval Center

## Outcome

- Added an application-shell approval badge and global drawer over the GMH-31 unified Manager projection.
- Added domain, status, and source-session filters with risk-first/expiry-second ordering and a bounded 1,000-record reconciliation window.
- Added one reusable approval card for the drawer and Chat inline surface. Unified Command/Plan cards suppress their legacy fallback card when the same request is present.
- Exposed explicit Once, five-minute Session, and backend-validated Rule grants for Command; Plan and Memory remain Once-only at the unified decision boundary.
- Added safe Plan summaries, Memory review diff, and optional Command rule suggestion to detail presentation without placing any payload in the ledger or event stream.
- Added stale refresh, outcome-unknown no-retry, durable reconnect deduplication, server-authoritative status, text labels, initial/focus-loop keyboard handling, and 44px action targets.

Memory edit/apply actions route to the owning Evolution proposal surface. Saving a proposal revision there continues to revoke the prior request and produce the new Pending fact through the existing governed domain path.

No provider, key, real profile, network service, administrator privilege, system security policy, push, or desktop process was used.
