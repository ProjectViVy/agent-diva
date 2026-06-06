# P0-2 Subagent Security Suite Release

## Release Method

No separate deployment step is required for this iteration.

The change ships through the normal Rust workspace build and runtime config load path:

- `agent-diva-core` exposes the new `tools.subagent` schema and defaults
- `agent-diva-cli` and `agent-diva-manager` map that config into `agent-diva-agent`
- `agent-diva-agent` enforces the runtime policy during subagent spawn and tool assembly

## Operator Notes

- Existing configs without `tools.subagent` continue to load because defaults are provided.
- Operators can explicitly opt subagents back into `web_fetch`, `web_search`, or `mcp`, but those capabilities are now off by default.
