# P1-8 SQLite WAL Acceptance

## Acceptance Steps

- Start a planning store with a file-backed SQLite database.
- Confirm `PRAGMA foreign_keys` returns `1`.
- Confirm `PRAGMA journal_mode` returns `wal`.
- Confirm `PRAGMA busy_timeout` returns `5000`.
- Create a plan with child steps, todos, events, and active-plan state, then delete the plan and confirm child rows are removed by cascade.
- Attempt a TodoList replacement that fails midway and confirm the prior todos and event log remain unchanged.

## Status

Accepted by automated coverage in `agent-diva-core` planning store tests.
