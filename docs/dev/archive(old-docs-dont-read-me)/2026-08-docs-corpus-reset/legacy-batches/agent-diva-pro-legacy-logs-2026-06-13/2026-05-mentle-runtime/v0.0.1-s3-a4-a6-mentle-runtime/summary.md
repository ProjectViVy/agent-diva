# Summary

## Changes

- Moved Mentle runtime assembly into an internal `mentle_runtime` module.
- Froze `MentleRuntime` ownership over toolkit, memory provider, custom tools, and active flag.
- Updated AgentLoop assembly to consume runtime state through helper methods.
- Added Sprint 3 A4-A6 design record for runtime lifecycle and helper boundaries.

## Impact

- Mentle runtime setup now has one internal construction path.
- AgentLoop startup and later tool rebuild paths can reuse the same custom tools vector.
- Sprint 4 can attach AgentLoop and cron behavior without reopening runtime ownership decisions.
