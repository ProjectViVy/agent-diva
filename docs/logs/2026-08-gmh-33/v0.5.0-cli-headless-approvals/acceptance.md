# Acceptance

Automated acceptance requires:

1. Interactive review displays domain, risk, scope, expiry, evidence count, and
   safe presentation before an explicit decision.
2. Empty/default input cannot allow; cancel/interrupt revokes the active request.
3. Non-interactive mode defaults to fail-closed and never executes a request that
   needs approval.
4. Queue is explicit, requires Manager, and returns a queryable Plan/Memory
   request while preserving non-success semantics.
5. Command queue never persists raw command payload and fails closed.
6. JSON reason codes and process status are stable on required/unavailable paths.

Human terminal observation is part of the one final M3 smoke, after all automated
release gates and completion audit pass. No intermediate operator action is needed.
