# Wave 3 Review Summary

## Iteration

- Date: `2026-07-04`
- Version: `v0.0.1-wave3-summary`
- Scope:
  - `Priority 3 / Wave F`
  - `Priority 4 / Wave E`
  - Commit range rooted in `94baa4b..HEAD` review program

## Overall Verdict

- `Wave E`: `Block release: yes`
- `Wave F`: `Block release: yes`
- Combined `Wave 3` verdict: `Do not ship`

## Top Risks

1. `JsonlTodoStore` can lose data under concurrent `create/update/archive` traffic.
2. The background-task tool path is not actually wired into production agent assembly.
3. Queued `RunKind::Subagent` work has no production worker bootstrap and can never execute end to end.
4. Workspace management commands can escape the managed root via unvalidated names and can delete arbitrary directories.

## Wave E Summary

- Commits reviewed: `909a573`, `84c5803`, `2445984`
- Key findings:
  - `P1`: `JsonlTodoStore` rewrites without locking, so concurrent `create/update/archive` can drop or erase todos.
  - `P2`: todo status vocabulary drifts across CLI/API/store (`open`, `pending`, `active`, `done`, `completed`).
  - `P2`: `GET /api/todos` accepts invalid status filters by silently returning all todos.
  - `P2`: `GET /api/todos` returns HTTP 200 on store failure while sibling todo endpoints return 5xx.
  - `P3`: store-level duplicate-ID and archive-after-query boundaries are not enforced or tested.

## Wave F Summary

- Commits reviewed: `c0f2712`, `c705538`, `b633b0d`
- Key findings:
  - `P1`: `enqueue_background_task` is dead code in production because no runtime injects a `RunStore` into `ToolAssembly`.
  - `P1`: `RunKind::Subagent` has no production worker bootstrap, so queued supervised runs do not execute.
  - `P1`: supervised runs complete as soon as `SubagentManager::spawn()` returns, not when the real subagent finishes.
  - `P2`: background-task metadata drops `chat_id/session/trace` context and does not inherit token-ledger semantics.
  - `P1`: workspace CLI accepts path-like names and can create/switch/delete outside `config_dir/workspaces`.
  - `P2`: workspace CLI model diverges from the broader runtime support for arbitrary workspace paths.

## Cross-Wave Matrix

- Persistence / data integrity:
  - todo store concurrent rewrite loses active data
  - archive flow can erase freshly appended todos
- Runtime wiring:
  - background-task tool not registered in real agent assembly
  - no supervised worker consumes queued subagent runs
- State machine / lifecycle:
  - supervised parent run reaches `completed` before child work is done
  - cancel/timeout semantics cannot cover detached subagent execution
- Security / isolation:
  - workspace name path traversal escapes managed root
  - workspace delete guard can be bypassed by global workspace override
- Contract consistency:
  - todo status terms differ across CLI/API/store
  - workspace management model conflicts with arbitrary-path runtime model
- Test gaps:
  - no concurrent todo-store tests
  - no end-to-end supervised-run worker test
  - no workspace traversal / override safety coverage

## Priority Pool

### P1

- `2445984` / `909a573` / `84c5803`: concurrent todo rewrite can lose data
- `c0f2712`: `enqueue_background_task` not wired into production assembly
- `c705538`: no production `RunKind::Subagent` worker bootstrap
- `c705538`: supervised run completes before the subagent actually finishes
- `b633b0d`: workspace CLI path traversal escapes the managed root

### P2

- `909a573` / `84c5803` / `2445984`: todo status contract drift across CLI/API/store
- `909a573`: invalid todo filter returns all rows instead of a request error
- `909a573`: todo list IO failures return HTTP 200
- `c0f2712` / `c705538`: background tasks lose reply-route, trace, and budget inheritance
- `b633b0d`: workspace CLI model drifts from arbitrary-path runtime behavior
- `b633b0d`: active-workspace delete guard can be bypassed via global `--workspace`

### P3

- `2445984`: duplicate-ID and archive/query boundary behavior is undefined and untested
- `b633b0d`: `workspace list` performs unexpected directory creation
- `b633b0d`: workspace command help and filesystem error output lack fidelity

## Recommended Fix Order

1. Close the release blockers that can corrupt data or break isolation:
   - todo store concurrency
   - workspace path traversal
2. Make the background-task lane real before extending it:
   - production `RunStore` injection
   - supervised worker bootstrap
3. Repair lifecycle and inheritance semantics:
   - parent/child run completion model
   - background task routing, trace, and token-ledger carry-over
4. Normalize user-facing contracts:
   - todo status parsing and HTTP error semantics
   - workspace registry vs arbitrary-path model
