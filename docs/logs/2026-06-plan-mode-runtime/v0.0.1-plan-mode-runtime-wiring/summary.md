# Plan Mode Runtime Wiring Summary

## Change

- Propagated GUI execution mode from ChatView through Tauri `send_message` into Manager chat requests.
- Added workspace-local planning runtime wiring for CLI, TUI, and Manager-created AgentLoop instances.
- Registered planning and TodoList tools through ToolAssembly, including Plan mode action restrictions.
- Injected active plan context into the main agent turn and added Plan mode fallback instructions.
- Registered GUI planning navigation and Tauri commands for active/list/detail plan queries.

## Impact

Plan mode now reaches the backend and changes runtime behavior instead of only changing local UI state. In Plan mode, implementation/external action tools are withheld or rejected; the agent can inspect files and use planning/TodoList tools to produce a plan, then stop for approval.

