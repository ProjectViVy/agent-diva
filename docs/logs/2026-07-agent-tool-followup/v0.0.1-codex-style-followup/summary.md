# Codex-style tool follow-up (empty final reply)

## Problem

After tool-only turns (e.g. final `exec` verification while executing a plan), the UI showed:

```text
I've completed processing but have no response to give.
```

because `final_content` stayed `None` when the loop ended on tools without a text reply.

## Fix (aligned with Codex `needs_follow_up`)

In `agent-diva-agent/src/agent_loop/loop_turn.rs`:

1. After successful tool batches (except plan approval lifecycle stop), **continue sampling**.
2. If tools run at `max_iterations`, grant **one summary-only bonus pass** (no tools + system nudge).
3. Replace the English hard-coded fallback with Chinese resolution order:
   - plan approval barrier → 审批提示
   - tools ran → synthesize tool summary
   - else → generic 未生成可读回复
4. Blank model content is treated as missing so synthesis can run.

## Non-goals

- No GUI changes
- Default `max_tool_iterations` unchanged
- Provider usage-missing warnings unchanged
