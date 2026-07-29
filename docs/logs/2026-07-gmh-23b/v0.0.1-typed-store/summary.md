# GMH-23B Summary

Added an async, replaceable Embedded Laputa typed store at
`.laputa/memory.sqlite3`. It persists canonical Core `MemoryRecord` values with
schema/store/row revisions, exact workspace scope, capacity bounds,
supersedes/tombstones, FTS5 candidates, integrity reporting, and SQLite
backup/restore.

The store is not connected to the production provider, proposal apply,
AgentLoop, Manager, Tauri, or GUI.
