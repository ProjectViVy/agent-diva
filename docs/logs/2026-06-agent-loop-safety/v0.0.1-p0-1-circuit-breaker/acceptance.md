# Acceptance

1. Trigger a tool-using task that repeatedly requests the same failing tool call with the same arguments.
2. Verify the agent stops after a bounded number of repeated failures instead of continuing indefinitely.
3. Verify the final user-visible response explains that execution was stopped because the same tool kept failing.
4. Trigger a similar task where the same tool is called with different arguments across iterations.
5. Verify the repeated-failure breaker does not fire for the argument-changing case and the loop instead falls back to its normal iteration budget.
6. Trigger the same repeated-failure scenario through a spawned subagent path and verify it also stops with a bounded failure result.
