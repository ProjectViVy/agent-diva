# agent-diva-tools

## OVERVIEW

Built-in tool implementations and MCP SDK integration. Concrete tools live here; the trait contract lives in `agent-diva-tooling`.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Tool trait re-exports | `src/lib.rs` (`Tool`, `ToolError`, `ToolRegistry`, `Result`) |
| Filesystem tools | `src/filesystem.rs` (`ReadFileTool`, `WriteFileTool`, `EditFileTool`, `ListDirTool`) |
| Shell execution | `src/shell.rs` (`ExecTool`) |
| Web fetch/search | `src/web.rs` (`WebFetchTool`, `WebSearchTool`) |
| Cron management | `src/cron.rs` (`CronTool`) |
| Spawn external processes | `src/spawn.rs` (`SpawnTool`) |
| Ask-user / attachment | `src/ask_user.rs`, `src/attachment.rs` |
| Memory tools | `src/memory_*.rs` (add/get/list/remove/search/update/distill) |
| Planning / working checkpoint | `src/update_plan.rs`, `src/update_working_checkpoint.rs` |
| Persona / skill / message | `src/persona.rs`, `src/skill.rs`, `src/message.rs` |
| MCP SDK integration | `src/mcp_sdk.rs` (load/probe/wrap discovered tools) |
| Tool discovery / registry | `src/tool_discovery.rs`, `src/registry.rs` |
| Sanitization helpers | `src/sanitize.rs` |

## CONVENTIONS

- Add a new tool as a struct implementing `Tool`, then re-export it from `src/lib.rs`.
- Register tools in `agent-diva-agent`'s `ToolAssembly`/`BuiltInToolsConfig`, not here.
- Keep MCP SDK wrapper types separate from built-in tool logic.
- Prefer `agent-diva-files` for file persistence rather than direct IO when deduplication/ref-counting matters.

## ANTI-PATTERNS

- Do not define new `Tool` traits here; use `agent-diva-tooling`.
- Do not bypass the sandbox/shell approval layer when invoking `ExecTool`/`SpawnTool`.
- Do not hardcode `~/.agent-diva` paths; accept paths from `CliRuntime` or config.

## NOTES

- `ActmemTool`/`ActmemEditWorkTool`/`ActmemCompleteTool`/`ActmemDropTool` are actmem workflow tools.
- `WtfTool` prints the ASCII logo and is mainly for easter-egg/diagnostics.
