# v0.0.1 P0-4 Backend Durability Acceptance

- When a provider/stream failure happens before a final assistant reply is saved, the user's inbound message still exists in session history.
- A normal successful turn stores exactly one user message for that turn.
- Session raw turn state is saved before consolidation advances `last_consolidated`.
- Session file write failures are no longer silent full-overwrite operations.
- Corrupt or unreadable session JSONL is treated as a load error instead of being silently replaced by a fresh empty session.
- The remaining GUI backend-first/cache reconciliation work is still tracked in `TODOLIST.md`.
