# Acceptance

1. Send a normal short request and confirm behavior is unchanged.
2. Trigger a long-history or large-tool-output turn and confirm the agent still answers instead of failing immediately.
3. Simulate or observe an overflow-like provider error and confirm:
   - the system retries once automatically with stronger compaction
   - if the retry still overflows, the user sees an explicit context-too-large message
4. Trigger a delegated subagent task and confirm it still completes under the same guardrail behavior.
