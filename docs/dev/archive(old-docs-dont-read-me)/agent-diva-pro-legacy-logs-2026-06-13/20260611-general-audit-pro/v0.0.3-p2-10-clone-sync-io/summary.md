# P2-10 Clone Hotspots and Sync IO Summary

## Changed

- Cached tool definitions once per agent turn in `agent_loop/loop_turn.rs` and reused the cached normal or cron-filtered definitions across loop iterations.
- Changed `ToolCallFinished` event payloads to send a bounded result preview instead of cloning and broadcasting full large tool output.
- Moved default `MemoryManager::sync_turn` write paths for `MEMORY.md` and `HISTORY.md` into `tokio::task::spawn_blocking`.
- Added focused regression coverage for event-result truncation.

## Impact

- Reduces repeated tool schema reconstruction during multi-iteration turns.
- Keeps complete tool output available for LLM context handling while reducing event-stream payload size.
- Avoids running memory persistence writes directly on async runtime worker threads.
