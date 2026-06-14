# P2-10 Release

## Release Method

No special deployment step is required. This is an internal performance and runtime-safety change in the Rust workspace.

## Rollback

Revert the commit `fix(audit): P2-10 reduce clone hotspots + async IO` if event consumers require full tool results in `ToolCallFinished` events or if memory persistence behavior regresses.
