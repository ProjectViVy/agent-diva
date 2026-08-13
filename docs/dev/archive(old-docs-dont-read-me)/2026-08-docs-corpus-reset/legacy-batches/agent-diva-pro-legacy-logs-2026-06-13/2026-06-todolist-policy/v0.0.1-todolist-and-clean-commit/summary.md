# v0.0.1 TODOLIST and Clean Commit Policy Summary

## What Changed

- Added the newly confirmed multimodal GUI paste boundary to root `TODOLIST.md`.
- Added a `TODOLIST Protocol` section to `AGENTS.md`.
- Added rulebook entries requiring discovered bugs and unfinished work to be recorded in `TODOLIST.md`.
- Added a cleanup-before-commit rule so temporary scripts, scratch data, archives, and other non-deliverable dirty-work files are removed or excluded before commits.
- Updated the no-self-commit rule to preserve the no-push/no-unrelated-change boundary while allowing commits when the repository rules or user instruction require them.

## Impact

This is a documentation and workflow-policy change. Runtime behavior is unchanged.

## Scope Note

Existing unrelated dirty files in the workspace are not cleaned or reverted by this iteration.
