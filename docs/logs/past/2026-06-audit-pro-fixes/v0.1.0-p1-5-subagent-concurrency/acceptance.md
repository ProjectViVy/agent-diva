# Acceptance

## Steps

1. Submit a batch with more than `MAX_CONCURRENT_SUBAGENTS` subagent tasks.
2. Confirm only the first window is spawned initially.
3. Confirm each completed task allows one queued task to start.
4. Confirm all tasks still return normal `SubAgentResult` values.
