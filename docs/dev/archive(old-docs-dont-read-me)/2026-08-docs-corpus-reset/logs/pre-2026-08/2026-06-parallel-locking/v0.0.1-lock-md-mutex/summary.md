# Summary

- Added root `LOCK.md` as the canonical mutex document for parallel Codex/Cursor/manual sessions.
- Updated `AGENTS.md` with a dedicated parallel lock mechanism section and a mandatory `parallel-lock-file-required` rule.
- Recorded the change in `TODOLIST.md` so the process change is durable in the project backlog/history.

## Impact

- Reduces the chance of overlapping edits in a dirty shared working tree.
- Makes active ownership, file scope, heartbeat, and handoff state visible without requiring extra tooling.
- Complements the existing worktree-isolation rule instead of replacing it.
