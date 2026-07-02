# P1-8 SQLite WAL Summary

## Changes

- Enabled SQLite `foreign_keys`, `journal_mode=WAL`, and `busy_timeout=5000` during planning store initialization.
- Wrapped planning schema creation in a transaction.
- Added transactional TodoList replacement for the multi-step delete/insert/event write path used by `todo_write`.
- Added tests covering PRAGMA configuration and `ON DELETE CASCADE` cleanup for planning child tables.

## Impact

- Planning store foreign-key constraints are now active for initialized connections.
- File-backed planning databases use WAL mode for improved read/write concurrency.
- Short lock contention waits up to 5000 ms before failing.
- TodoList replacement either fully completes with its revision event or rolls back to the prior list.
