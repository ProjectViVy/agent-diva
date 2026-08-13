# GMH-23C Shadow Recall Summary

Embedded Laputa now exposes a shadow-only Recall service backed by the GMH-23B
typed SQLite FTS5 store and the GMH-22 Recall v2 pipeline.

The candidate adapter bounds and quotes queries, returns workspace-global plus
exact-session records, normalizes BM25 deterministically, and degrades without
fallback. Core ranking now derives importance and query-matched persona signals
from canonical records, retains recency decay, and diversifies selection by
record kind.

The production MemoryProvider prefetch, AgentLoop prompt, Manager, Tauri, GUI,
schema, authority, proposal, and Mentle paths are unchanged.
