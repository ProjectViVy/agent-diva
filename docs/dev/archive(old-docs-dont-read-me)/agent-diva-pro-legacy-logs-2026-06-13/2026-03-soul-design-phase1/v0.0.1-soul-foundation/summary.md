# Iteration Summary

## Scope
- Implemented Soul design phase-1 for `agent-diva` with default-on, fallback-compatible behavior.
- Added workspace templates, context injection, bootstrap state support, transparency notices, and subagent identity inheritance.

## Key Changes
- Added templates in workspace sync: `SOUL.md`, `IDENTITY.md`, `USER.md`, `BOOTSTRAP.md` while preserving existing `PROFILE.md`.
- Added `agents.soul` configuration with defaults:
  - `enabled: true`
  - `max_chars: 4000`
  - `notify_on_change: true`
  - `bootstrap_once: true`
- Added soul lifecycle state persistence at `workspace/.agent-diva/soul-state.json`.
- Upgraded system prompt assembly to inject soul-related markdown sections in order with graceful fallback.
- Added transparent final-response notice when soul identity files are changed via tools.
- Added subagent prompt inheritance from `SOUL.md` and `IDENTITY.md` summary.
- Updated migration conversion for new config field and clamped migrated Brave web search max results to valid range.

## Impact
- Existing workspaces remain compatible with fallback behavior.
- New workspaces get soul files automatically.
- Agent behavior is now identity-file driven and can evolve transparently across sessions.
