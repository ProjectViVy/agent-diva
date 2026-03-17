# Command Index

## CLI Commands

### `agent-diva config`
- Purpose: Manage config files, runtime paths, validation, and diagnostics for a single CLI instance.
- Input format:
  - `agent-diva [--config <config.json> | --config-dir <dir>] config path [--json]`
  - `agent-diva [--config <config.json> | --config-dir <dir>] config refresh`
  - `agent-diva [--config <config.json> | --config-dir <dir>] config validate [--json]`
  - `agent-diva [--config <config.json> | --config-dir <dir>] config doctor [--json]`
  - `agent-diva [--config <config.json> | --config-dir <dir>] config show --format <pretty|json>`
- Output / expected behavior:
  - `path`: print resolved config/runtime/workspace paths.
  - `refresh`: preserve existing values, fill defaults, sync workspace templates.
  - `validate`: run schema + semantic validation only.
  - `doctor`: run validation plus readiness checks, using exit code `1` for invalid config and `2` for warnings/readiness failures.
  - `show`: print effective config with secrets redacted.
- Examples:
  - `agent-diva config path`
  - `agent-diva --config ~/.agent-diva/config.json config doctor --json`
  - `agent-diva --config ~/.agent-diva/config.json config show --format json`
- Boundary conditions:
  - `--config` and `--config-dir` are mutually exclusive.
  - JSON output must not include ASCII logo or informational logs on stdout.

### `agent-diva onboard`
- Purpose: Initialize, refresh, or overwrite instance config and workspace templates.
- Input format:
  - `agent-diva onboard [--provider <name>] [--model <id>] [--api-key <key>] [--api-base <url>] [--workspace <dir>] [--refresh] [--force]`
- Output / expected behavior:
  - Creates or refreshes `config.json`.
  - Creates workspace directory and syncs templates without overwriting existing files.
  - Prints config path, runtime root, workspace path, and suggested next steps.
- Examples:
  - `agent-diva onboard`
  - `agent-diva --config ~/.agent-diva/config.json onboard --refresh`
  - `agent-diva --config ~/.agent-diva/dev.json onboard --provider openai --model openai/gpt-4o --api-key sk-...`
- Boundary conditions:
  - Existing config defaults to refresh-or-overwrite prompt unless `--refresh` or `--force` is supplied.
  - Provider names must exist in `ProviderRegistry`.

### `agent-diva provider`
- Purpose: Manage the active provider/model pair and inspect provider readiness from the CLI.
- Input format:
  - `agent-diva [--config <config.json> | --config-dir <dir>] provider list [--json]`
  - `agent-diva [--config <config.json> | --config-dir <dir>] provider status [--json]`
  - `agent-diva [--config <config.json> | --config-dir <dir>] provider set --provider <name> [--model <id>] [--api-key <key>] [--api-base <url>] [--json]`
  - `agent-diva [--config <config.json> | --config-dir <dir>] provider models --provider <name> [--static-fallback] [--json]`
  - `agent-diva provider login <provider> [--json]`
- Output / expected behavior:
  - `list`: print manageable providers from registry, including default model metadata and readiness.
  - `status`: print current default model, resolved provider, and readiness/missing fields.
  - `set`: update `agents.defaults.model` plus provider credentials through `ConfigLoader`.
  - `models`: query the provider's runtime model catalog when supported, optionally falling back to bundled static metadata.
  - `login`: stable placeholder interface for future OAuth/device login flows.
- Examples:
  - `agent-diva provider list`
  - `agent-diva --config ~/.agent-diva/config.json provider status --json`
  - `agent-diva --config ~/.agent-diva/config.json provider set --provider deepseek --api-key sk-...`
  - `agent-diva --config ~/.agent-diva/config.json provider models --provider openai --json`
- Boundary conditions:
  - `provider set` only supports providers that have config slots in the Rust config schema.
  - If registry metadata has no default model for a provider, `provider set` requires explicit `--model` unless the current configured model already belongs to that provider.
  - `provider models` is read-only and never mutates config.
  - Unsupported providers return structured `unsupported` or `static_fallback` responses instead of guessing undocumented endpoints.
  - JSON output must not include ASCII logo or extra stdout noise.

### `agent-diva chat`
- Purpose: Start a lightweight prompt-style chat loop without entering the TUI.
- Input format:
  - `agent-diva [--config <config.json> | --config-dir <dir>] [--workspace <dir>] chat [--model <id>] [--session <key>] [--markdown|--no-markdown] [--logs|--no-logs]`
- Output / expected behavior:
  - Opens a terminal prompt loop for direct local or remote chat.
  - Supports slash commands `/quit`, `/clear`, `/new`, `/stop`.
  - Reuses the same runtime/config resolution rules as `agent-diva agent`.
- Examples:
  - `agent-diva chat`
  - `agent-diva --config ~/.agent-diva/config.json --workspace ~/work chat --logs`
  - `agent-diva --remote chat --session cli:chat:remote`
- Boundary conditions:
  - `chat` does not replace `tui`; it is the lightweight interactive path.
  - `/stop` targets the current session key.
  - `--workspace` only overrides the current process runtime; it does not rewrite config unless another command saves it.

## Meta Commands

### `/new-command`
- Purpose: Define or extend a command contract in repository docs before or alongside implementation.
- Input format:
  - Triggered as a workflow/meta command in planning or agent interaction.
  - Required fields to collect: command name, purpose, input format, output/expected behavior, examples, boundary conditions.
- Output / expected behavior:
  - Update `commands/commands.md` with the new command contract.
  - Sync the command index in `AGENTS.md` when a new command is added or an existing command meaningfully changes.
  - If code implementation is part of the same iteration, keep the documented contract aligned with the shipped CLI/agent behavior.
- Examples:
  - `/new-command` for a future `/triage` slash command.
  - `/new-command` to formalize `agent-diva provider` command behavior before wiring GUI consumers.
- Boundary conditions:
  - Do not add undocumented commands.
  - Do not let code behavior drift from the documented input/output contract.
