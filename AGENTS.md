# Repository Guidelines

## Project Structure & Module Organization
This repository is a Rust workspace. Crates are organized by responsibility:
- `agent-diva-core`: shared config, memory/session, cron, heartbeat, and event bus foundations.
- `agent-diva-agent`: agent loop, context assembly, skill/subagent flow.
- `agent-diva-providers`: LLM/transcription provider abstractions and implementations.
- `agent-diva-channels`: channel adapters (Slack, Discord, Telegram, Email, QQ, etc.).
- `agent-diva-tools`: built-in tools (filesystem, shell, web, cron, spawn).
- `agent-diva-cli`: user-facing CLI entrypoint.
- `agent-diva-migration`: migration utility from earlier versions.
Use each crate��s `src/` for code; add crate-level integration tests under `tests/` when needed.

## Build, Test, and Development Commands
Prefer `just` recipes from the workspace root:
- `just build` / `just build-release`: build all crates (debug/release).
- `just test`: run `cargo test --all`.
- `just check`: run clippy with warnings denied.
- `just fmt` and `just fmt-check`: format or verify formatting.
- `just ci`: run formatting, lint, and tests (CI-equivalent gate).
- `just run -- <args>`: run `agent-diva-cli`.
- `just migrate -- <args>`: run migration CLI.

## Coding Style & Naming Conventions
Use Rust 2021 conventions and keep `rustfmt` output authoritative (`rustfmt.toml` is checked in). Use `snake_case` for modules/functions/files, `PascalCase` for structs/enums/traits, and `SCREAMING_SNAKE_CASE` for constants. Keep public APIs documented with `///`; use `//!` for module overviews when helpful. Run `cargo clippy --all -- -D warnings` before opening a PR.

## Testing Guidelines
Write focused unit tests near the code with `#[cfg(test)]`. Add integration tests in crate `tests/` folders for cross-module behavior. Run `cargo test --all` locally before pushing. Use workspace test utilities (`tokio-test`, `tempfile`, `wiremock`, `mockito`) where appropriate.

## Commit & Pull Request Guidelines
Recent history follows Conventional Commit prefixes (`feat:`, `fix:`, `docs:`); keep using that style with concise imperative summaries. Before PRs, run `just ci`, describe behavioral impact, link related issues, and update docs when interfaces/channels/providers change. Keep PRs focused to a single concern for easier review.
