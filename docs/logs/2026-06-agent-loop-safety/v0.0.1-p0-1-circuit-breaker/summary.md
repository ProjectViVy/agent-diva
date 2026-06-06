# v0.0.1 P0-1 Circuit Breaker

## Summary

This iteration closes `P0-1: Infinite loop / circuit breaker`.

- Added `agent-diva-agent::loop_guard` as a shared internal guard for both the main agent loop and subagent execution.
- Reused existing iteration budgets while adding loop-level wall-clock timeout protection.
- Added stable tool-call fingerprinting based on tool name plus canonicalized JSON arguments.
- Stopped repeated identical failing tool calls after a bounded threshold and surfaced a clear stop reason to the user.

## Impact

- Main agent turns no longer keep retrying the same failing tool call indefinitely.
- Subagents now use the same breaker model instead of relying on an isolated bare loop.
- Successful repeated tool calls remain allowed; only repeated identical failures trip the breaker.
