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

## 平台构建与打包指南（GUI + CLI）

下面是常用的构建 / 打包路径，方便在本机或 CI 上快速得到可分发产物。

### macOS：GUI dmg（内置 CLI，可由 GUI 一键启动网关）

- **一键脚本**：在 workspace 根目录执行：
  - `chmod +x scripts/build-macos-gui-bundle.sh`
  - `./scripts/build-macos-gui-bundle.sh`
- **脚本做的事**：
  - 在根目录执行：`cargo build --release -p agent-diva-cli`，生成 `target/release/agent-diva`；
  - 执行 `python3 scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os macos`：
    - 将 `agent-diva` 复制到 `agent-diva-gui/src-tauri/resources/bin/macos/agent-diva`；
    - 写入 `resources/manifests/gui-bundle-manifest.json`；
  - 进入 GUI 目录：`cd agent-diva-gui && pnpm install && pnpm tauri build`；
  - 最终在 `agent-diva-gui/src-tauri/target/release/bundle/dmg/` 下生成 `.dmg`。
- **运行效果**：
  - 安装后，`Agent Diva.app` 中已内置 `agent-diva` CLI；
  - GUI 通过 Tauri commands（如 `start_gateway`）从 `resources/bin/macos/agent-diva` 拉起 `agent-diva gateway run`，实现前后端一体的桌面体验。

### Windows：GUI 安装包 + 可选网关服务

- **GUI 安装包**（NSIS/MSI，由 Tauri 生成）：
  - 在 Windows 开发机或 CI 上：
    - 先构建 CLI：`cargo build --release -p agent-diva-cli`；
    - 准备 GUI 资源：`python scripts/ci/prepare_gui_bundle.py --gui-root agent-diva-gui --target-os windows`；
    - 构建 GUI：`cd agent-diva-gui && pnpm install && pnpm tauri build`；
  - 产物：
    - `.exe`/`.msi` 位于 `agent-diva-gui/src-tauri/target/release/bundle/nsis|msi/`；
    - 安装包会将 `agent-diva.exe` 打入 `resources/bin/windows/`。
- **可选 Windows 服务安装**：
  - 安装器通过 `agent-diva-gui/src-tauri/windows/hooks.nsh` 提供“安装并启动 Agent Diva Gateway 服务”选项；
  - 勾选时若找到 `resources/bin/windows/agent-diva.exe`，会调用：
    - `agent-diva.exe service install --auto-start`
    - `agent-diva.exe service start`
  - GUI 中的服务管理命令（如 `install_service` / `start_service`）通过 CLI `service` 子命令桥接到系统服务。

### Linux / 其他平台：CLI 构建与打包（无 GUI）

- **本地构建 CLI**：
  - `cargo build --release -p agent-diva-cli`
  - 产物：`target/release/agent-diva`。
- **已有打包脚本**：
  - Linux 打包脚本：`scripts/package-linux.sh`（如存在）或参考 `justfile` 中的 `package-linux` / `build-all-packages`：
    - 使用 `cargo build --release --package agent-diva-cli`；
    - 生成 zip / deb / rpm 等分发包（需要额外工具如 `cargo-deb`、`cargo-generate-rpm`）。
- **运行方式**：
  - 直接使用 CLI 运行：
    - `agent-diva gateway run`：启动本地网关；
    - `agent-diva tui`：启动终端 UI；
    - `agent-diva config` / `agent-diva provider` / `agent-diva chat` 等子命令见上文。

### 通用建议

- **先本地 smoke，再上 CI**：
  - 在本机先用上述命令完成一次完整构建，确认 `.dmg` / 安装包可正常安装并运行；
  - 再将构建脚本接入 CI，并在 `docs/logs/` 中记录对应版本的 `summary.md` / `verification.md`。
- **环境前提**：
  - Rust stable（`cargo`, `rustc`）；
  - Node.js 18+、`pnpm`（或 npm/yarn）；
  - 对应平台工具链：
    - macOS：Xcode Command Line Tools、Tauri CLI；
    - Windows：MSVC toolchain、NSIS/MSI 打包依赖由 Tauri 处理；
    - Linux：pkg-config、zlib/zstd 等基础 C 库（由依赖自动触发）。
