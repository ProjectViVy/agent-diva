# CTX-C3 summary

Implemented session-persistent tool-result artifacts and pressure-driven microcompaction.

- Added a workspace/session-bound artifact store under `.agent-diva/tool-artifacts/` with opaque UUID identifiers, atomic writes, SHA-256 verification, stable errors, quotas, TTL GC, and session deletion.
- Added complete sanitized `ToolExecutionOutput`, versioned `ToolResultRef`, and the CORE `read_tool_result` tool with a 12,000-character range limit.
- Results up to 12,000 characters remain inline. Larger successful results become a 3,000-character preview plus reference; artifact failure uses the shared 80,000-character safety fallback.
- Main-agent and supervised-subagent execution share the same result pipeline. Large MCP results are no longer truncated before persistence.
- When C2 reports the inline tool-result layer over its soft limit, completed results older than the current tool group and larger than 4,000 characters are replaced in place, oldest first. Cache observation receives `expected_deletion`.

The artifacts are operational session data, not BML/Laputa memory authority, and their content is not appended to Experience Journal evidence.
