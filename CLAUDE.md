# CLAUDE.md

This file provides guidance to Claude Code when working in this repository. `AGENTS.md` remains the authoritative rule source; keep this file aligned with it.

## Communication Rules

- Reply to users in Chinese.
- Prefix every user-facing reply with `[I strictly follow the rules]`.

## Project Overview

Agent Diva is a Rust workspace for a modular AI assistant system spanning agent runtime, provider integrations, channels, tooling, local gateway, GUI, AutoDream, Laputa memory/proposal services, and sandbox policy enforcement.

## Workspace Structure

Primary crates:

- `agent-diva-core`: shared config, memory/session, cron, heartbeat, event bus foundations
- `agent-diva-agent`: agent loop, context assembly, skill/subagent flow
- `agent-diva-providers`: LLM/transcription provider abstractions and implementations
- `agent-diva-channels`: channel adapters
- `agent-diva-tools`: built-in tools
- `agent-diva-files`: file indexing and file-management helpers
- `agent-diva-tooling`: shared tooling abstractions/utilities
- `agent-diva-neuron`: supporting types/helpers used by the GUI
- `agent-diva-manager`: default local gateway and HTTP control plane for `agent-diva-cli`
- `agent-diva-autodream`: AutoDream manual-run, proposal, input, and output lifecycle support
- `agent-diva-laputa`: proposal, migration, recovery, and memory-provider services
- `agent-diva-sandbox`: sandbox policy, execution, platform adapters, approval/guardian support
- `agent-diva-cli`: user-facing CLI entrypoint
- `agent-diva-service`: Windows service wrapper
- `agent-diva-gui`: optional Tauri desktop app
- `agent-diva-migration`: migration utility from earlier versions

Nested/reference workspace:

- `agent-diva-nano`: lives under `.workspace/agent-diva-nano/`; it is not a root workspace member

Additional repository facts:

- Root workspace package version: `0.5.0`
- Rust MSRV: `1.80.0`
- Main branch in this checkout: `agent-diva-pro`
- `.workspace/` contains sibling reference projects such as `openfang`, `zeroclaw`, `nanobot`, `codex`, and `memtle`
- Mentle integration is intentionally pinned to published `memtle = 0.1.2`; do not replace it with path/git overrides in the main workspace

## Development Guidance

- Keep cross-cutting domain types in `agent-diva-core`.
- Keep provider/channel-specific logic isolated in dedicated crates.
- Prefer small modules and composable functions over large files.
- Avoid `unwrap`/`expect` in non-test code; propagate errors with context.
- Keep async boundaries explicit and avoid blocking in async paths.
- Preserve backward compatibility for public interfaces unless a breaking change is intentional and documented.
- When users reference `openclaw`, `nanobot`, `shannon`, or similar projects, inspect `.workspace/` first and adapt ideas to Agent Diva architecture.

## Build, Check, and Run

Prefer `just` from the workspace root:

```bash
just build
just build-release
just test
just check
just fmt
just fmt-check
just ci
just mentle-package-policy
just sprint5-default-check
just mentle-check
just sprint5-check
just run -- <args>
just migrate -- <args>
```

Useful direct cargo commands:

```bash
cargo test -p <crate>
cargo test <test_name>
cargo run -p agent-diva-cli -- <args>
```

Windows Mentle note:

- If `clang-cl.exe` exists under `C:\Program Files\LLVM\bin` but is not on `PATH`, prepend that directory before running `cargo check -p agent-diva-agent --features mentle`.

## Validation Rules

- Default post-change validation is `just fmt-check`, `just check`, `just test`.
- If a change does not justify one of those commands, explain why.
- For user-visible or executable behavior changes, run at least one smoke path in addition to tests.
- If modifying `agent-diva-gui`, include GUI-specific smoke validation.
- Record validation results in iteration logs under `docs/logs/.../verification.md`.

## Process Files

- `AGENTS.md`: authoritative repository rules
- `CLAUDE.md`: Claude-oriented mirror of operational guidance; keep aligned with `AGENTS.md`
- `TODOLIST.md`: canonical backlog for discovered bugs, gaps, and deferred work
- `LOCK.md`: canonical mutex file for parallel Codex/Cursor/manual sessions

## Parallel Work Rules

- If the user says the project is in a parallel state, do not continue development in the shared root working tree.
- Move work into an isolated branch/worktree or copied sibling workspace before editing.
- Before any write task or workspace-mutating command, read `LOCK.md`.
- If `LOCK.md` shows an active overlapping scope, do not edit in that scope until the lock is released, handed off, or you move to a non-overlapping isolated workspace.
- Every parallel write task must update `LOCK.md` with owner, session/task, branch/worktree, exact scope, heartbeat, and expiry.
- `GLOBAL` scope is reserved for migrations, bulk formatting, or broad refactors.

## Iteration Log Protocol

Each deliverable change must create a new versioned directory under `docs/logs/<theme>/v0.0.1-slug/` containing:

- `summary.md`
- `verification.md`
- `release.md`
- `acceptance.md`

Optional files such as `notes.md`, `prd.md`, or `rollback.md` may be added when needed.

## TODOLIST Protocol

- Add discovered bugs, unfinished work, known limitations, or deferred improvements to `TODOLIST.md` unless fixed in the same iteration.
- Entries should contain status, short title, reason/context, expected behavior, and related files/docs when available.
- Completed items should be moved or marked under `Done`, not silently deleted.

## Commit Rules

- Use English Conventional Commit prefixes such as `feat:`, `fix:`, `docs:`.
- Create one commit per completed, self-contained update.
- Do not push unless the user explicitly asks.
- Stage only files for the current focused update; do not include unrelated pre-existing workspace changes.
- Clean up scratch artifacts created during the task before committing.
- For non-trivial commits, include validation notes in the commit body or accompanying report.

## Command Mechanism

Agreed meta-commands:

- `/new-command`
- `/config-meta`
- `/check-meta`
- `/new-rule`
- `/commit`
- `/validate`
- `/init`

If commands are added or modified, update both the command index in `AGENTS.md` and `commands/commands.md` when that file exists or is first created.

## Provider Model-ID Safety Rule

When calling a provider's native OpenAI-compatible endpoint such as DeepSeek `https://api.deepseek.com/v1`:

- send the raw provider model ID such as `deepseek-chat`
- do not auto-rewrite it into LiteLLM form such as `deepseek/deepseek-chat`
- only apply `provider/model` rewriting for a true LiteLLM-style gateway or aggregator

Any provider-routing change should add or update tests that assert the final outbound `model` value.

## Testing Conventions

- Write focused unit tests near the code with `#[cfg(test)]`.
- Add integration tests under crate `tests/` directories for cross-module behavior.
- Cover success paths and representative failure paths.
- Validate serialization/config parsing for new config fields.
- Prefer deterministic tests; avoid external network access in tests.
- Use timeout-aware assertions for async behavior to avoid hanging CI.

## Safety and Observability

- Never commit real secrets or tokens.
- Redact sensitive fields in logs, errors, snapshots, and fixtures.
- Prefer least-privilege defaults for tools/channels that execute external actions.
- Use structured, actionable errors with preserved source context.
- Emit logs at appropriate levels and avoid noisy hot-path logging.
- Include useful identifiers without leaking private data.

## Notes for Claude

- The workspace may already be dirty from parallel stories; preserve unrelated changes.
- Do not revert user changes unless explicitly instructed.
- When updating process docs, also update `TODOLIST.md` and `docs/logs`.
- If `AGENTS.md` and `CLAUDE.md` diverge, align `CLAUDE.md` to `AGENTS.md` and state that `AGENTS.md` is authoritative.
