# Agent Diva (Rust)

A lightweight, extensible personal AI assistant framework written in Rust.
This repository contains a multi-crate workspace that powers the agent core,
provider integrations, channel adapters, built-in tools, and the CLI.

[![CI](https://github.com/HKUDS/agent-diva/actions/workflows/ci.yml/badge.svg)](https://github.com/HKUDS/agent-diva/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Read this in other languages: [简体中文](README.zh-CN.md)

## Why Agent Diva

- Fast startup and low resource usage.
- Modular architecture (swap channels, providers, tools).
- First-class CLI for local workflows and automation.
- Durable memory and session management.
- Skills system for adding capabilities via Markdown.

## Workspace layout

```
agent-diva/
|-- agent-diva-core/       # Shared config, memory/session, cron, heartbeat, event bus
|-- agent-diva-agent/      # Agent loop, context assembly, skill/subagent flow
|-- agent-diva-providers/  # LLM/transcription provider abstractions and implementations
|-- agent-diva-channels/   # Channel adapters (Slack, Discord, Telegram, Email, QQ, etc.)
|-- agent-diva-tools/      # Built-in tools (filesystem, shell, web, cron, spawn)
|-- agent-diva-cli/        # User-facing CLI entrypoint
|-- agent-diva-migration/  # Migration utility from earlier versions
`-- agent-diva-gui/        # Optional GUI (if enabled in your build)
```

## Requirements

- Rust 1.70+ (install via rustup)
- Optional: `just` for convenient workspace commands

## Quick start

Clone and build:

```bash
git clone https://github.com/HKUDS/agent-diva.git
cd agent-diva
cargo build --all
```

Install the CLI locally:

```bash
cargo install --path agent-diva-cli
```

Initialize configuration:

```bash
agent-diva onboard
```

## Configuration

Default config file:

```
~/.agent-diva/config.json
```

Environment variable overrides are supported (both structured and aliases). For
example:

```
AGENT_DIVA__AGENTS__DEFAULTS__MODEL=...
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
```

## Usage

```bash
# Start the gateway (agents + enabled channels)
agent-diva gateway

# Send a single message
agent-diva agent --message "Hello, Agent Diva!"

# Launch interactive TUI
agent-diva tui

# Check status
agent-diva status
```

## Development

Common commands (prefer `just` when available):

```bash
# List available recipes
just

# Format, lint, and test
just ci

# Run all tests
just test
```

Without `just`:

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
```

## Documentation

- Architecture: `docs/architecture.md`
- Development: `docs/development.md`
- Migration: `docs/migration.md`

## Contributing

See `CONTRIBUTING.md` for guidelines. Please keep PRs focused and run `just ci`
before submitting.

## License

MIT. See `LICENSE`.

## Acknowledgements

This Rust workspace is a reimplementation of the original Agent Diva project.
