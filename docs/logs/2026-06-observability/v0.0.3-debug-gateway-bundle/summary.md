# v0.0.3 Debug Gateway Bundle Summary

## Changed

- Added explicit foreground debug mode via `agent-diva gateway run --debug`.
- Added raw debug run files under `config_dir/debug-runs/<run-id>/`.
- Added raw `events.jsonl` and `raw.jsonl` debug event streams for gateway, provider, tool, and MCP boundaries.
- Added `agent-diva gateway bundle [--run-id <id>]` to create a local zip from one debug run.

## Impact

- Normal gateway logs and runtime JSONL logs keep their existing conservative redaction/truncation behavior.
- Debug mode intentionally records raw payloads and may include secrets, provider requests/responses, tool output, MCP I/O, and channel messages.
- No GUI, manager observability API, settings page, or clear-logs surface was added in this iteration.
- Deeper provider-native HTTP bytes and MCP SDK internal RPC frames remain a follow-up; this iteration records the full payloads visible at current agent/runtime boundaries.
