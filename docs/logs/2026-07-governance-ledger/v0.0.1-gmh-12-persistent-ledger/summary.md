# GMH-12 Summary

Added the future canonical SQLite approval ledger and state machine to Core.

- Persisted payload-free, excerpt-free append-only governance events.
- Derived pending, allowed, denied, revoked, consumed, and expired states by
  replay.
- Enforced version CAS, durable idempotency, TTL, receipt binding, terminal
  denial/revocation, and one-time consumption.
- Added explicit, non-production Plan and Sandbox compatibility adapters.
- Kept existing Plan, Sandbox, Manager, AgentLoop, GUI, and command-rule
  persistence behavior unchanged.

GMH-12 is complete in `TODOLIST.md`; production integration begins only in later
domain stories.
