# P0-2 Subagent Security Suite Acceptance

## Acceptance Steps

1. Start the CLI or manager runtime with default config and trigger a task that uses `spawn`.
2. Confirm the child task can still perform workspace file or shell work.
3. Confirm the child task does not receive web search or MCP capability unless explicitly enabled in `tools.subagent`.
4. Confirm a second concurrent child beyond the configured limit is rejected with a clear reason.
5. Confirm a nested child beyond the configured depth is rejected with a clear reason.

## Expected User-Facing Outcome

- Subagents remain usable for delegated local work.
- Unsafe child capability inheritance is no longer the default.
- Runtime refusal messages distinguish concurrency exhaustion from depth overflow.
