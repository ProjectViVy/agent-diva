# Session-Scoped Ephemeral PLAN Runtime

PLAN drafts, approvals, execution context, and execution TODOs now live only in the backend process and are keyed by session.

The startup path removes obsolete `planning.db`, `planning.db-wal`, and `planning.db-shm` files. Session reset and deletion discard their corresponding PLAN runtime state.
