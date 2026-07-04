# LOCK

Codex/Cursor/manual parallel session mutex file.
Use this file to declare the current writer scope before mutating the workspace.

## Status

- Lock State: `FREE`
- Scope: `none`
- Owner: `none`
- Session/Task: `none`
- Branch/Worktree: `agent-diva-pro @ C:\Users\Administrator\Desktop\morediva\agent-diva`
- Started At: `2026-07-04T10:12:00+08:00`
- Last Heartbeat: `2026-07-04T11:08:00+08:00`
- Expires At: `2026-07-04T11:08:00+08:00`

## Lock Rules

1. Read this file before editing files, running worktree-mutating commands, or preparing a commit.
2. If `Lock State` is `HELD` and the heartbeat is still valid, do not modify files covered by `Scope`.
3. If work must proceed in parallel, prefer an isolated `git worktree` or separate branch and still record the new scope here.
4. `Scope` must name concrete files, directories, or modules.
5. Refresh `Last Heartbeat` at least every 30 minutes while holding the lock.
6. Record handoff notes before taking over a stale lock.
7. Use `GLOBAL` only for repo-wide migrations, bulk formatting, or similarly broad work.

## Acquisition Checklist

- Set `Lock State` to `HELD`
- Fill `Owner`, `Session/Task`, and `Branch/Worktree`
- Fill an exact `Scope`
- Record `Started At`, `Last Heartbeat`, and `Expires At`

## Release Checklist

- Confirm the work is complete, handed off, or explicitly paused
- Set `Lock State` to `FREE`
- Reset `Scope`, `Owner`, and `Session/Task` to `none`
- Record remaining risks or next steps in `Handoff Notes`

## Active Lock

- `none`

## Handoff Notes

- `2026-07-04`: Codex released the Wave 1 remediation lock after landing runtime fixes in `cb2ffda` and recording `docs/logs/2026-07-wave1-remediation/v0.0.1-wave1-remediation/`.
