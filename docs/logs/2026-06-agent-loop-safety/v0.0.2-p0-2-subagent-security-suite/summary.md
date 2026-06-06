# P0-2 Subagent Security Suite Summary

## Scope

This iteration closes `P0-2` by making subagent delegation least-privilege by default and by enforcing runtime safety limits on nested delegation.

## Delivered

- Added `tools.subagent` config defaults in `agent-diva-core`:
  - `max_concurrent = 2`
  - `max_depth = 1`
  - `allow_shell = true`
  - `allow_filesystem = true`
  - `allow_web_fetch = false`
  - `allow_web_search = false`
  - `allow_mcp = false`
- Added `SubagentPolicy` runtime type in `agent-diva-agent` for shared policy evaluation.
- Reworked `SubagentManager` into the policy owner:
  - rejects new subagents when concurrency is full
  - rejects nested delegation beyond configured depth
  - trims child network and MCP config before execution
- Reworked subagent tool assembly to rebuild from a policy whitelist instead of inheriting the parent set and only blacklisting a few tools.

## Impact

- Subagents now default to filesystem + shell only.
- Subagents no longer inherit web search credentials by default.
- Subagents no longer inherit MCP servers by default.
- Existing hard bans on `spawn`, `cron`, and `attachment` remain in place.
- Main-agent tool availability is unchanged.

## Closure Notes

- Workspace validation is now fully green after fixing several pre-existing or environment-sensitive tests exposed during the `P0-2` closeout.
- The follow-up fixes were test/environment stabilization only; they did not expand `P0-2` scope beyond the subagent least-privilege and runtime-limit work.
