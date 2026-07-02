# Summary

## Changes
- Refactored `agent-diva-agent` agent loop internals into `loop_xxx` submodules while keeping public APIs unchanged.
- Added `agent-diva-agent/src/agent_loop/loop_tools.rs` for tool registration and runtime network tool refresh.
- Added `agent-diva-agent/src/agent_loop/loop_runtime_control.rs` for runtime control command handling, cancellation checks, and error event emission.
- Added `agent-diva-agent/src/agent_loop/loop_turn.rs` for single-turn processing flow and turn helper functions.
- Kept `agent-diva-agent/src/agent_loop.rs` as the orchestration shell (constructors, run entrypoint, direct processing wrappers, governance timing).
- Moved helper tests related to soul-file detection/notice formatting to `loop_turn` tests; kept existing AgentLoop creation/process tests.

## Impact
- No intentional behavior change in message processing, event emission, tool execution policy, or outward interfaces.
- Reduced file-level complexity in `agent_loop.rs` and improved maintainability by responsibility-based module boundaries.
