# v0.0.2 P0-4 Frontend Reconciliation Acceptance

Recommended acceptance steps:

1. Start the GUI and open a session that has both backend history and stale `agent-diva-session-cache:*`; confirm the backend history is rendered.
2. Simulate backend unavailability and reopen a cached session; confirm the warning banner indicates cached/stale history.
3. Send a message and wait for completion; confirm the final message list matches backend canonical history rather than the optimistic placeholder stream.
4. Stop a running generation; confirm no empty assistant placeholder remains and the session re-syncs from backend history.
5. Reset the current session; confirm the old cached history does not reappear.
6. Delete a historical session and reload the history menu; confirm the deleted session does not resurrect from local cache.
