# Wave 3 Workspace CLI Hardening

## Scope

- Hardened `agent-diva-cli` managed workspace commands against path-like workspace names.
- Removed the `workspace list` directory-creation side effect.
- Fixed deletion protection to honor the persisted active workspace even when `--workspace` overrides the runtime view.

## Changes

- Added workspace-name validation so `workspace create/switch/delete` only accept a single managed directory segment.
- Stopped `workspace list` from creating `config_dir/workspaces` when no managed workspaces exist.
- Switched delete protection from `effective_workspace()` to the persisted config workspace path.
- Added regression coverage for traversal rejection, list side effects, and delete-guard bypass attempts.

## Impact

- Blocks managed-workspace escape via traversal or absolute-like names in CLI commands.
- Prevents accidental deletion of the configured active managed workspace under override-driven execution.
- Keeps the broader managed-vs-arbitrary workspace model question explicitly deferred in `TODOLIST.md`.
