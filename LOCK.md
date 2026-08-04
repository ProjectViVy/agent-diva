# LOCK

Codex/Cursor/manual parallel session mutex file.
Use this file to declare the current writer scope before mutating the workspace.

## Status

- Lock State: `RELEASED`
- Scope: `none`
- Owner: `none`
- Session/Task: `none`
- Branch/Worktree: `agent-diva-pro / C:\Users\Administrator\Desktop\morediva\agent-diva`
- Started At: `2026-08-05T00:00:00+08:00`
- Last Heartbeat: `2026-08-05T01:45:00+08:00`
- Expires At: `released`

## Lock Rules

1. Read this file before editing files, running worktree-mutating commands, or preparing a commit.
2. If `Lock State` is `HELD` and the heartbeat is still valid, do not modify files covered by `Scope`.
3. If work must proceed in parallel, prefer an isolated `git worktree` or separate branch and still record the new scope here.
4. `Scope` must name concrete files, directories, or modules.
5. Refresh `Last Heartbeat` at least every 30 minutes while holding the lock.
6. Record handoff notes before taking over a stale lock.
7. Use `GLOBAL` only for repo-wide migrations, bulk formatting, or similarly broad work.

## Active Lock

- No active lock.

## Handoff Notes

- `2026-08-05T01:45:00+08:00`: Released after fixing GUI sandbox settings save
  (snake_case enum unification). Manual desktop smoke deferred; see TODOLIST.md
  `SANDBOX-SAVE-FIX-DESKTOP-SMOKE`.
- `2026-08-03T20:08:00+08:00`: Released after adding `just make-diva` and verifying dual-window spawn.
