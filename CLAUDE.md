# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Agent Diva is a modular AI assistant framework written in Rust. It connects multiple chat platforms (Telegram, Discord, Slack, WhatsApp, Feishu, DingTalk, Email, QQ) to multiple LLM providers (OpenRouter, Anthropic, OpenAI, DeepSeek, Groq, Gemini, and others) with a built-in tool system.

## Build & Development Commands

The project uses `just` as a command runner (install via `cargo install just`). The justfile is configured for PowerShell on Windows.

```bash
just build            # Build all crates
just build-release    # Build in release mode
just test             # Run all tests
just check            # Run clippy linter
just fmt              # Format code
just ci               # Run fmt-check + clippy + tests (full CI pipeline)
just run <ARGS>       # Run the CLI (e.g., just run gateway)
just install          # Install CLI binary locally
```

Without just:
```bash
cargo build --all
cargo test --all
cargo clippy --all -- -D warnings
cargo fmt --all
```

Running specific tests:
```bash
cargo test test_name                    # Single test by name
cargo test --package agent-diva-core    # Tests in one crate
cargo test message_bus                  # Tests matching pattern
cargo test -- --nocapture               # With stdout output
```

Set log level: `RUST_LOG=debug cargo run`

## Architecture

This is a Cargo workspace with 8 crates:

- **agent-diva-core** — Foundation: message bus (dual-queue inbound/outbound), configuration loading, session management (JSONL persistence), memory system (MEMORY.md + HISTORY.md), error types
- **agent-diva-agent** — Agent loop, context builder (assembles LLM prompts), skill loader (Markdown-based skills), subagent manager
- **agent-diva-providers** — LLM provider trait + implementations; uses LiteLLM-compatible HTTP API pattern with a provider registry
- **agent-diva-channels** — Channel handler trait + channel manager + platform-specific handlers
- **agent-diva-tools** — Tool trait + registry + implementations (filesystem, shell, web, message, spawn, cron, MCP)
- **agent-diva-cli** — Entry point binary; commands: `onboard`, `gateway`, `agent`, `tui`, `status`, `channels`, `cron`
- **agent-diva-migration** — Migrates config/sessions from the older Python version
- **agent-diva-gui** — Optional Tauri + Vue.js desktop GUI (in `agent-diva-gui/src-tauri`)
- **agent-diva-manager** — API server for remote management

### Data Flow

Incoming messages flow: Channel Handler → Message Bus (inbound) → Agent Loop → Context Builder → LLM Provider → Tool Execution (if needed) → Message Bus (outbound) → Channel Handler (response).

Sessions persist to JSONL files via the Session Manager. Long-term memory uses MEMORY.md and append-only HISTORY.md files.

### Key Traits

- `Provider` trait in agent-diva-providers — implement to add a new LLM provider
- `ChannelHandler` trait in agent-diva-channels — implement to add a new chat platform
- `Tool` trait in agent-diva-tools — implement to add a new tool

## Configuration

Config file: `~/.agent-diva/config.json`

Precedence: Environment variables (`AGENT_DIVA__*`) > config file > defaults.

## Code Conventions

- Dependencies are declared in workspace `Cargo.toml` under `[workspace.dependencies]` and referenced with `{ workspace = true }` in crate-level Cargo.toml files
- Error handling: `thiserror` for library crate errors, `anyhow` for application-level (CLI)
- Async runtime: Tokio multi-threaded; use `tokio::sync::mpsc` for message passing, `tokio::spawn` for concurrency
- Logging: `tracing` crate (`info!`, `debug!`, `warn!`, `error!`)
- Tests: `#[cfg(test)]` in-module, `#[tokio::test]` for async tests, mock external services with mockito/wiremock

## Provider Model-ID Safety Rule (Critical)

- For native provider OpenAI-compatible endpoints (example: DeepSeek `https://api.deepseek.com/v1`), keep raw model IDs unchanged (example: `deepseek-chat`).
- Do **not** auto-prefix raw IDs into LiteLLM form (example: avoid rewriting to `deepseek/deepseek-chat`) when not using a gateway.
- Apply `provider/model` prefix rewriting only for real LiteLLM-style gateways/aggregators.
