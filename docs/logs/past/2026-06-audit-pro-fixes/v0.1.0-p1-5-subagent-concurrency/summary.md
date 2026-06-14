# P1-5 Subagent Concurrency Limit

## Summary

- Added a fixed `MAX_CONCURRENT_SUBAGENTS` limit for pro batch subagent execution.
- Changed `spawn_batch` to start only up to the concurrency limit, then enqueue another task each time one finishes.
- Preserved existing per-task timeout handling in `run_isolated_subagent`.

## Impact

- A large batch request no longer starts all provider/tool tasks at once.
- Result semantics remain unchanged: completed tasks return their `SubAgentResult`, panics are converted into `SubAgentStatus::Error`, and per-task timeout remains enforced.
