# Agent Diva Architecture Guide

**Generated:** 2026-08-07
**Branch:** agent-diva-pro
**Purpose:** Exploratory architecture reference. For project rules and processes, see `AGENTS.md`.

## OVERVIEW

Agent Diva is a self-hosted Rust workspace gateway that connects chat apps (Telegram, Discord, Slack, etc.) to LLM providers. It is organized as a multi-crate workspace with dedicated crates for core types, agent loop, providers, channels, tools, memory authority (Laputa/BML), sandbox governance, gateway HTTP API, and a Tauri desktop GUI.

## STRUCTURE

Root workspace manifest: `Cargo.toml`. Members (17 Rust packages):

| Crate | Responsibility |
|---|---|
| `agent-diva-core` | Shared config, session, memory, cron, heartbeat, event bus, security. |
| `agent-diva-agent` | Agent loop, context assembly, skill/subagent flow, tool assembly. |
| `agent-diva-providers` | LLM/transcription provider abstractions and implementations. |
| `agent-diva-channels` | Channel adapters (Slack, Discord, Telegram, Email, QQ, Feishu, WhatsApp, DingTalk, Matrix, etc.). |
| `agent-diva-tools` | Built-in tools (filesystem, shell, web, cron, spawn) and MCP SDK integration. |
| `agent-diva-files` | File indexing, FileManager, storage backends shared by agent/tool flows. |
| `agent-diva-tooling` | Narrow `Tool`/`ToolRegistry`/`Module` trait contract shared by the workspace. |
| `agent-diva-neuron` | Single-turn LLM node types/helpers used heavily by the GUI. |
| `agent-diva-manager` | Local gateway runtime and HTTP control plane (Axum). |
| `agent-diva-autodream` | AutoDream manual-run, proposal, input, and output lifecycle. |
| `agent-diva-laputa` | Laputa/BML typed memory authority, proposals, migration, recovery. |
| `agent-diva-sandbox` | Sandbox policy, execution, platform adapters, approval/guardian. |
| `agent-diva-cli` | User-facing CLI entrypoint (`agent-diva` binary) + reusable CLI support library. |
| `agent-diva-service` | Windows service wrapper for `agent-diva gateway run`. |
| `agent-diva-migration` | Offline migration utility (`agent-diva-migrate` binary). |
| `agent-diva-gui/src-tauri` | Tauri desktop app Rust backend (nested crate manifest). |
| `agent-diva-e2e` | Real-LLM end-to-end test harness (not a production crate). |

Notable layout points:

- The only nested crate manifest in the root workspace is `agent-diva-gui/src-tauri/Cargo.toml`; the `agent-diva-gui` directory is a Tauri + Vue app, not a standard Rust crate root.
- The `.workspace/` directory referenced in older docs is not present at the repository root. There is a top-level `workspace/` directory that holds runtime mask data, not a Rust workspace.
- The top-level `-p/` directory is empty and appears to be an accidental artifact from a mistyped cargo command.
- `agent-diva-nano` is referenced as a nested workspace under `.workspace/agent-diva-nano/` in older docs, but it is not currently present in this checkout.

## WHERE TO LOOK

| Task | Location | Notes |
|---|---|---|
| Add a CLI command | `agent-diva-cli/src/main.rs`, `agent-diva-cli/src/lib.rs` | Reusable command modules go in `lib.rs` so the GUI can reuse them. |
| Change the agent loop | `agent-diva-agent/src/lib.rs`, `agent-diva-agent/src/agent_loop.rs` | Core loop, context assembly, skill/subagent flow. |
| Add an LLM provider | `agent-diva-providers/src/lib.rs`, `agent-diva-providers/src/*.rs` | Implement `LLMProvider`; update `ProviderRegistry`. |
| Add a chat channel | `agent-diva-channels/src/lib.rs`, `agent-diva-channels/src/*/` | Add handler under the channel-specific module. |
| Add a built-in tool | `agent-diva-tools/src/lib.rs`, `agent-diva-tools/src/*/` | Register in crate lib and tooling registry. |
| Change memory/governance | `agent-diva-laputa/src/lib.rs`, `agent-diva-laputa/src/*/` | Authority store, proposals, migration, recovery. |
| Change sandbox/approval | `agent-diva-sandbox/src/lib.rs`, `agent-diva-sandbox/src/*/` | Policy, orchestrator, guardian, platform adapters. |
| Change gateway HTTP API | `agent-diva-manager/src/lib.rs`, `agent-diva-manager/src/server.rs`, `agent-diva-manager/src/handlers/` | Axum server, routes, handlers. |
| Change desktop GUI backend | `agent-diva-gui/src-tauri/src/lib.rs`, `agent-diva-gui/src-tauri/src/*/` | Tauri commands and embedded gateway lifecycle. |
| Change desktop GUI frontend | `agent-diva-gui/src/` | Vue 3 + TypeScript components, stores, API clients. |
| Cross-cutting domain types | `agent-diva-core/src/lib.rs`, `agent-diva-core/src/*/` | Config, session, memory, event bus, cron, security. |
| End-to-end tests | `agent-diva-e2e/src/`, `agent-diva-e2e/tests/` | Scenario-driven harness; real LLM tests are env-gated. |
| Build/CI recipes | `justfile`, `.github/workflows/ci.yml`, `scripts/ci/` | Project-specific gates live in Python scripts. |

## CODE MAP

| Symbol / Crate | Type | Entry Point | Role |
|---|---|---|---|
| `agent-diva` | bin | `agent-diva-cli/src/main.rs` | User-facing CLI (onboard, gateway, agent, chat, tui, cron, config, etc.). |
| `agent-diva-cli` | lib | `agent-diva-cli/src/lib.rs` | Reusable CLI command modules consumed by the GUI. |
| `agent-diva-service` | bin | `agent-diva-service/src/main.rs` | Windows service wrapper for `agent-diva gateway run`. |
| `agent-diva-migrate` | bin | `agent-diva-migration/src/main.rs` | Offline migration utility. |
| `agent-diva-manager` | lib | `agent-diva-manager/src/lib.rs` | Local gateway runtime and HTTP control plane. |
| `agent-diva-core` | lib | `agent-diva-core/src/lib.rs` | Shared config, session, memory, cron, heartbeat, event bus, security. |
| `agent-diva-agent` | lib | `agent-diva-agent/src/lib.rs` | Agent loop, context assembly, skill/subagent flow, tool assembly. |
| `agent-diva-providers` | lib | `agent-diva-providers/src/lib.rs` | LLM/transcription provider abstractions and implementations. |
| `agent-diva-channels` | lib | `agent-diva-channels/src/lib.rs` | Channel adapters and ChannelManager. |
| `agent-diva-tools` | lib | `agent-diva-tools/src/lib.rs` | Built-in tools and MCP SDK integration. |
| `agent-diva-files` | lib | `agent-diva-files/src/lib.rs` | File indexing, FileManager, storage backends. |
| `agent-diva-tooling` | lib | `agent-diva-tooling/src/lib.rs` | Tool trait primitives (`Tool`, `ToolRegistry`, `Module`). |
| `agent-diva-neuron` | lib | `agent-diva-neuron/src/lib.rs` | Single-turn LLM node types for the GUI. |
| `agent-diva-autodream` | lib | `agent-diva-autodream/src/lib.rs` | AutoDream manual-run and proposal lifecycle. |
| `agent-diva-laputa` | lib | `agent-diva-laputa/src/lib.rs` | Laputa/BML typed memory authority and governance. |
| `agent-diva-sandbox` | lib | `agent-diva-sandbox/src/lib.rs` | Sandbox policy, execution, orchestrator, guardian, approvals. |
| `agent-diva-e2e` | lib | `agent-diva-e2e/src/lib.rs` | Real-LLM end-to-end test harness. |
| `agent-diva-gui` | lib/bin | `agent-diva-gui/src-tauri/src/lib.rs` | Tauri backend; `main.rs` is a tiny launcher. |

## Conventions

- **Rust toolchain:** edition 2021, MSRV 1.80.0 (`Cargo.toml`).
- **Formatting:** `rustfmt.toml` uses standard settings (max_width 100, 4 spaces, `reorder_imports`).
- **Linting:** `clippy.toml` pins MSRV; warnings are denied via `just check` (`cargo clippy --all -- -D warnings`).
- **Naming:** `snake_case` modules/functions/files, `PascalCase` structs/enums/traits, `SCREAMING_SNAKE_CASE` constants.
- **Error handling:** prefer `thiserror`/`anyhow::Context`; avoid `unwrap`/`expect` in non-test code.
- **Async:** use `#[tokio::test]` for async tests; avoid blocking calls inside async paths.
- **Tests:** unit tests live in `#[cfg(test)]` modules next to code; cross-module tests live in crate `tests/`.
- **Test utilities:** `tempfile` for isolated storage, `mockito` for HTTP mocks, `wiremock` in a few CLI unit tests. `tokio-test` is declared but not widely used.
- **No snapshot testing:** `insta`/`snapbox`/`expect-test` are not used.
- **Cross-cutting types:** keep domain types in `agent-diva-core`; keep provider/channel-specific code isolated.

## Anti-Patterns

- **Provider model-ID rewriting on native endpoints.** When calling a provider's native OpenAI-compatible endpoint (e.g., DeepSeek), send the raw model ID (`deepseek-chat`). Only apply `provider/model` prefixes for true gateways/aggregators.
- **`unwrap`/`expect` in non-test code.** Propagate or map errors with context instead.
- **Blocking calls inside async paths.** Keep async boundaries clear.
- **External network calls in unit/integration tests.** Use mocks (`mockito`, `wiremock`, hand-rolled TCP/WebSocket servers) or `MockProvider` implementations.
- **Direct Laputa authority writes outside `apply_proposal`.** Only `LaputaWrite::apply_proposal()` may write the authority store.
- **AutoDream writing MEMORY.md, identity, or authority directly.** AutoDream produces drafts/proposals; durable changes must go through Laputa governance.
- **Auto-allowing dangerous command prefixes.** The sandbox bans 44+ prefixes (python, bash, sudo, pwsh, node, etc.) from being auto-suggested as Allow rules (`agent-diva-sandbox/src/exec_policy.rs`).
- **GUI changes validated by Rust tests only.** GUI changes need at minimum `just gui-automated-check` or a real workflow smoke test.
- **Using Node-only validation as the default standard.** This is a Rust workspace; prefer `just`/`cargo` recipes.
- **Deleting or restoring mass-deleted Vue components/CSS during cleanup.** Mass deletion is a merge regression; stop and flag instead of "fixing" it.

## Commands

| Recipe | What it runs |
|---|---|
| `just build` / `just build-release` | `cargo build --all` (debug/release). |
| `just test` | `cargo test --all`. |
| `just check` | `cargo clippy --all -- -D warnings`. |
| `just fmt` / `just fmt-check` | Format or verify formatting. |
| `just ci` | `fmt-check` → `check` → `test` → `health-benchmark-check` → `feature-gate-check` → `laputa-clean-break-check`. |
| `just run -- <args>` | `cargo run --package agent-diva-cli -- <args>`. |
| `just migrate -- <args>` | `cargo run --package agent-diva-migration -- <args>`. |
| `just e2e-test` | `cargo test -p agent-diva-e2e -- --nocapture`. |
| `just memory-provider-check` | Focused agent-loop memory/provider assembly regressions. |
| `just laputa-clean-break-check` | Prove retired legacy memory names (`mentle`/`memtle`) have not re-entered code/manifests/CI/build recipes. |
| `just feature-gate-check` | Verify each non-default `agent-diva-sandbox` feature compiles individually. |
| `just health-benchmark-check` | Run the ignored `health_benchmark_ci_gate_stays_within_budget` manager test. |
| `just gui-automated-check` | GUI npm test/build + Tauri cargo check. |
| `just start` / `just diva-gate` / `just make-diva` | Local full-stack or gateway-only dev startup helpers. |

**CI notes:**
- The default GitHub push/PR job runs `just fmt-check`, `just check`, `just build`, `just sprint5-default-check`, and `just feature-gate-check`; it does **not** run `cargo test --all` unless triggered via `workflow_dispatch` with `run_tests=true`.
- The `just sprint5-default-check` recipe referenced in `.github/workflows/ci.yml` is currently missing from `justfile`; use `just memory-provider-check` / `just laputa-clean-break-check` / `just ci` for equivalent local coverage.
- The `release` job runs only on `v*.*.*` tags and consumes GUI artifacts from the `gui-build` job.

## Notes

- Embedded Laputa (SQLite + FTS5) is the sole production Memory authority. Legacy Memory files are offline import sources only. The umbrella term is **BML (Basic Memory Layer)**; the previous name "Mentle" is retired.
- `agent-diva-tooling` is intentionally small; it owns the `Tool`/`ToolRegistry`/`Module` contract shared by `agent-diva-tools` and other crates.
- `agent-diva-e2e` is a dedicated real-LLM test harness, not a production crate.
- Some legacy packaging scripts (`scripts/package-macos.sh`, `scripts/package-linux.sh`) still hard-code version `0.4.10`, while the workspace version is `0.5.0`.
- `agent-diva-gui/package.json` is also at version `0.4.10`, out of sync with the Rust workspace.
