# v0.0.3 P0-6 Context Overflow Guardrail

## Summary
- Added agent-level context budget defaults and validation under `agents.defaults`.
- Added heuristic request sizing, proactive context compaction, and a single overflow recovery retry in the main agent loop.
- Reused the same overflow guardrail in subagent execution so delegated tasks do not bypass the new runtime safety.

## Impact
- Main and subagent requests now shrink oversized tool outputs and drop oldest history before provider calls.
- Overflow-like provider errors now produce one stronger retry and then a clear user-visible error instead of silent degradation.
- Existing vision-specific provider rejection handling remains intact.
