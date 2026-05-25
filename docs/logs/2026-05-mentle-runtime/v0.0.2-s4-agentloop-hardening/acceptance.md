# Acceptance

## Criteria

- Sprint 4 entry audit records the S3 review package baseline and remaining
  hardening scope.
- Initial AgentLoop assembly consumes active `MentleRuntime` helper state
  without hand-written Mentle setup in the startup path.
- `build_agent_tools(...)` remains the single helper for built-in, MCP, cron,
  spawn, attachment, and custom tools.
- Startup cron rebuild and default-tool rebuild preserve main-agent
  `memtle_status` custom tools.
- `with_toolset()` disables Mentle prompt routing when the supplied registry does
  not contain `memtle_status`.
- Subagent registry assembly excludes Mentle custom tools and keeps
  `for_subagent().mentle == false`.

## Result

Accepted for Sprint 4 hardening scope. The remaining full-suite issue observed
is unrelated to AgentLoop/Mentle assembly and is recorded in `verification.md`.
