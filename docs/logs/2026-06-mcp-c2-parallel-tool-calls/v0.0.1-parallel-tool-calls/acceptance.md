# Acceptance

1. Trigger two MCP tool executions that share the same server session.
2. Confirm both calls can make progress concurrently instead of waiting on a global write lock.
3. Confirm `cargo test -p agent-diva-tools` passes.
