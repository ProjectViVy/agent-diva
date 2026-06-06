# v0.0.1 P1-5 Tool Execution Timeout Wrapping

## Summary

This iteration closes `P1-5: Tool execution timeout wrapping`.

- Added a registry-level default tool timeout to `agent-diva-tooling::ToolRegistry`.
- Wrapped `ToolRegistry::execute()` with `tokio::time::timeout` so all tools get a global execution cap.
- Reused `tools.exec.timeout` as the default timeout for all tool calls instead of shell-only semantics.
- Wired `ToolAssembly` to propagate `exec_timeout` into the registry and added config validation for `tools.exec.timeout > 0`.

## Impact

- Filesystem, web, spawn, message, and other non-shell tools now have a global timeout guard.
- Existing shell and MCP internal timeouts remain in place as finer-grained protection.
- Main agent and subagent registries now share the same default tool timeout behavior.
