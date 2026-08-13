# Plan Mode Runtime Wiring Acceptance

## Acceptance Steps

1. Start the app normally.
2. Select Plan mode in the chat composer.
3. Send a planning request.
4. Confirm the request reaches Manager with `mode = plan`.
5. Confirm the agent can use planning/TodoList tools and read-only inspection tools.
6. Confirm implementation/external action tools such as shell, write/edit file, web, MCP, spawn, cron, and custom tools are unavailable or rejected.
7. Open the Planning navigation item and confirm active/list/detail plan queries load through the registered Tauri commands.

