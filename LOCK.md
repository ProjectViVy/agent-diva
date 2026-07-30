# LOCK

Codex/Cursor/manual parallel session mutex file.
Use this file to declare the current writer scope before mutating the workspace.

## Status

- Lock State: `FREE`
- Scope: —
- Owner: —
- Session/Task: —
- Branch/Worktree: —
- Started At: —
- Last Heartbeat: —
- Expires At: —

## Lock Rules

1. Read this file before editing files, running worktree-mutating commands, or preparing a commit.
2. If `Lock State` is `HELD` and the heartbeat is still valid, do not modify files covered by `Scope`.
3. If work must proceed in parallel, prefer an isolated `git worktree` or separate branch and still record the new scope here.
4. `Scope` must name concrete files, directories, or modules.
5. Refresh `Last Heartbeat` at least every 30 minutes while holding the lock.
6. Record handoff notes before taking over a stale lock.
7. Use `GLOBAL` only for repo-wide migrations, bulk formatting, or similarly broad work.

## Active Lock

- None.

## Handoff Notes

- `2026-07-30T23:25:00+08:00`: Released documentation-only Skill product
  boundary update. SOP is no longer a distinct type or planned product; future
  work is a deferred visual CRUD lifecycle for all Skills and is removed from
  the default B5 route. No runtime, config, or user-data changes.

- `2026-07-30T23:10:00+08:00`: Released documentation-only TODOLIST master
  execution planning. Added the B0-B8 dependency blueprint, Goal execution and
  human-pause protocol, project completion definition, Evolution re-baseline
  gate, and canonical workspace identity backlog item. No runtime, config, or
  user-data changes.

- `2026-07-30T22:45:00+08:00`: Released TODOLIST triage archive. Documentation
  only: slimmed `TODOLIST.md`, added
  `docs/archive/todolist/completed-through-2026-07-30.md`,
  `docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md`, and
  `docs/logs/2026-07-todolist-triage/v0.0.1-archive-and-residual/`. G2D real
  desktop acceptance remains the Active Plan product gate; no runtime code
  changed and no push occurred.

- `2026-07-30T22:38:39+08:00`: User-approved TODOLIST triage took over from the
  previous G2D GLOBAL lock (Codex / root, started 22:25). G2D real-desktop
  acceptance remains the product Active Plan; this slice only archives completed
  history and rewrites residual open items.

- `2026-07-30T21:32:08+08:00`: Released GMH-24A/B/C architecture closure.
  Commits `e0760897`, `36009ede`, and `af453d93` add offline typed import,
  shadow/typed read authority, governed typed writes and rollback, and remove
  the legacy runtime product/build surface.
