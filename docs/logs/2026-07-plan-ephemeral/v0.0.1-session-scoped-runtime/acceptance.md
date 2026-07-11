# Acceptance

1. A PLAN draft generated for session A is unavailable to session B.
2. Approval requires the same session key, plan id, revision, and markdown hash.
3. Restarting the backend loses all PLAN state; legacy SQLite PLAN data is deleted at startup.
4. Session reset or deletion discards the session's draft/execution state.
5. Chat history no longer receives PLAN snapshot messages.
