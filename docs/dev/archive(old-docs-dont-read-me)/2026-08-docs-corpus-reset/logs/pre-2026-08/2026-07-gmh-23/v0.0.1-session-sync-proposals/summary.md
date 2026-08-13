# GMH-23 Stage 1 Summary

## Outcome

Laputa-backed post-turn synchronization now persists consolidation candidates as
governed proposals instead of silently doing nothing or writing applied authority.

- Non-empty memory updates create a medium-risk `memory_patch`.
- Non-empty history entries create a low-risk `history_patch` containing the
  applied history plus the candidate entry.
- Both proposals begin in `pending_review` with bounded session evidence.
- Empty synchronization remains a deterministic `Noop`.
- Applied `memory_md` and `history_md` files are unchanged until a later
  governance decision applies a proposal.

## Compatibility

The `MemoryProvider` interface, AgentLoop call path, session format, Manager/CLI/
Tauri contracts, and legacy Markdown provider behavior are unchanged. AutoDream
already used the canonical Laputa proposal API and was not modified.

GMH-23 remains open for GUI/import write paths, risk-policy auto-apply/HITL, and
end-to-end edit, conflict, compensation, and forgetting flows.
