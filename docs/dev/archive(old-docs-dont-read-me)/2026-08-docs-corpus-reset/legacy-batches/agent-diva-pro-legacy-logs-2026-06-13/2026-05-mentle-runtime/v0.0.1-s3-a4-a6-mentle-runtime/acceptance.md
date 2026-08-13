# Acceptance

## Criteria

- `MentleRuntime` aggregates toolkit, memory provider, custom tools, and active flag.
- Runtime lifecycle and ownership are documented.
- AgentLoop assembly uses the runtime helper instead of direct Mentle setup.
- Custom tools remain reusable for initial assembly and cron/default tool rebuilds.
- Sprint 4 can use the frozen helper boundary without adding a second Mentle setup path.
