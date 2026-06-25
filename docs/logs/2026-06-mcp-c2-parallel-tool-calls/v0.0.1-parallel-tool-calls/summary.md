# Summary

- Fixed `agent-diva-tools/src/mcp_sdk.rs` so `McpSdkTool::execute()` no longer holds a write lock across the full MCP request.
- Switched the shared MCP session handle to a clonable trait object behind `RwLock<Option<...>>`, allowing concurrent tool calls on the same MCP server.
- Added a regression test that runs two tool calls concurrently against the same shared client and asserts they overlap.
