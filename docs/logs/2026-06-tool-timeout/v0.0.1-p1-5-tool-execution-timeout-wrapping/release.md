# Release

No migration or special deployment steps are required.

- This change is internal runtime hardening for tool execution.
- Existing `tools.exec.timeout` settings remain valid, but now apply to all tool calls by default.
- Rollout follows the normal CLI/manager/GUI binary release path.
