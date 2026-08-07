# agent-diva-cli

## OVERVIEW

User-facing `agent-diva` binary and a reusable CLI support library for the GUI.
Binary entry point is `src/main.rs`; public helpers are exported through `src/lib.rs`.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| CLI command enum and top-level dispatch | `src/main.rs` (`Cli`, `Commands`) |
| Reusable command modules consumed by the GUI | `src/lib.rs` re-exports `approval_commands`, `chat_commands`, `cli_runtime`, `client`, `commands`, `provider_commands` |
| Config onboarding and config subcommands | `src/main.rs` (`run_onboard`, `run_config_*`) |
| Provider subcommands | `src/provider_commands.rs` |
| Chat/agent execution | `src/chat_commands.rs` |
| Approval review/decision | `src/approval_commands.rs` |
| Manager HTTP client for remote mode | `src/client.rs` (`ApiClient`) |
| Shared runtime helpers | `src/cli_runtime.rs` (`CliRuntime`) |
| TUI implementation | `src/main.rs` (`run_tui`, `TuiApp`) |
| Cron subcommands | `src/main.rs` (`run_cron_*`) |
| Channel subcommands | `src/main.rs` (`run_channel_*`) |
| Status and diagnostic reports | `src/main.rs` (`run_status`, `run_config_doctor`) |
| Windows service commands | `src/service.rs` |
| Workspace, todo, mask subcommands | `src/commands/workspace.rs`, `src/commands/todo.rs`, `src/commands/mask.rs` |

## CONVENTIONS

- Add new top-level commands to the `Commands` enum in `src/main.rs` and route them in `async_main`.
- Prefer `CliRuntime` for config loading, path resolution, and workspace selection.
- Support `--json` on commands that may be scripted; check `is_structured_output` before printing human-readable text.
- Global flags (`--config`, `--config-dir`, `--workspace`, `--remote`, `--api-url`) are parsed once in `Cli`; pass the resolved `CliRuntime` or `ApiClient` down.
- Keep terminal-only code in `main.rs`; expose reusable helpers through `src/lib.rs`.
- Use `agent_diva_cli::...` paths when the binary imports its own library modules.

## ANTI-PATTERNS

- Do not import private `main.rs` items from the GUI or other crates; use the public `agent-diva-cli` library surface.
- Do not hardcode `~/.agent-diva` paths; go through `CliRuntime` or the config loader.
- Do not add blocking I/O inside async command handlers.
- Do not implement provider/model/config logic directly in `main.rs` when `cli_runtime.rs` already has helpers.
- Do not add TUI rendering or raw-terminal handling outside `main.rs`.

## NOTES

Remote-mode commands use `ApiClient` and mostly skip local gateway setup; local commands build a `CliRuntime` and may start the gateway directly.
