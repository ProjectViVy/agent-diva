# v0.0.1-applied-laputa-memory-provider

## Summary

Story 5.1 implemented read-only applied Laputa authority consumption through the `MemoryProvider` boundary.

Changes:

- Added `agent_diva_laputa::LaputaMemoryProvider`, a read-only `MemoryProvider` adapter that renders applied Laputa sections as explicit authority prompt blocks.
- Wired `ContextBuilder`, `AgentLoop`, and manager runtime construction to prefer the Laputa adapter when `.laputa` exists, with safe fallback to `MemoryManager`.
- Removed default runtime prompt authority reads from legacy `SOUL.md`, `IDENTITY.md`, `USER.md`, `memory/MEMORY.md`, and `memory/HISTORY.md` paths.
- Changed subagent prompt assembly to consume applied Laputa authority instead of inherited legacy identity files.
- Added tests covering applied rendering, unapplied proposal exclusion, legacy non-authority behavior, and read-failure degradation.

## Impact

Default prompt authority now flows through applied Laputa state instead of direct legacy files. Legacy files remain available to migration and compatibility paths, but are not rendered as default authority.
