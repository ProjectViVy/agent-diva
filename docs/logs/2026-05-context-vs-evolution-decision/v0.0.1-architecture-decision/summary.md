# Summary

## Change

Added `docs/dev/genericagent/context-compaction-vs-autonomous-evolution-decision.md`.

The document records the architecture decision that context compaction and autonomous evolution are separate tracks:

- context compaction is session-local runtime survival;
- autonomous evolution is cross-session subject continuity;
- the current main goal should start from autonomous evolution governance, not full context compaction.

## Impact

This clarifies implementation order for the next Agent-Diva architecture stage and prevents the AutoDream/rhythm memory work from being blocked by or confused with session-local prompt compaction.

