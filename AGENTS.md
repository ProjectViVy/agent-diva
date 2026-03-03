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

Use each crate's `src/` for code; add crate-level integration tests under `tests/` when needed.

**Common workspace conventions:**

- Keep cross-cutting domain types in `agent-diva-core`.
- Keep provider/channel-specific code isolated in their dedicated crates.
- Prefer adding small modules over large monolithic files.
- Place examples and fixtures under the owning crate when practical.

## Getting Started

- Install Rust stable (via `rustup`) and ensure `cargo`, `rustfmt`, and `clippy` are available.
- Install `just` and run commands from the workspace root.
- Copy and configure local environment files if required by a crate or channel.
- Verify toolchain and project health with `just fmt-check && just check && just test`.

## Development Guide
If users request references to projects such as openclaw, nanobot, or shannon, prioritize reviewing the contents under the .workspace directory. Analyze the architectures of these sibling projects and propose a development approach suitable for the agent-diva architecture.

## Build, Test, and Development Commands

Prefer `just` recipes from the workspace root:

- `just build` / `just build-release`: build all crates (debug/release).
- `just test`: run `cargo test --all`.
- `just check`: run clippy with warnings denied.
- `just fmt` and `just fmt-check`: format or verify formatting.
- `just ci`: run formatting, lint, and tests (CI-equivalent gate).
- `just run -- <args>`: run `agent-diva-cli`.
- `just migrate -- <args>`: run migration CLI.

**Useful direct cargo invocations:**

- `cargo test -p <crate>`: run tests for a single crate.
- `cargo test <test_name>`: run a specific test by name.
- `cargo run -p agent-diva-cli -- <args>`: run CLI without `just`.

## Coding Style & Naming Conventions

Use Rust 2021 conventions and keep `rustfmt` output authoritative (`rustfmt.toml` is checked in). Use:

- `snake_case` for modules/functions/files,
- `PascalCase` for structs/enums/traits,
- `SCREAMING_SNAKE_CASE` for constants.

Keep public APIs documented with `///`; use `//!` for module overviews when helpful. Run `cargo clippy --all -- -D warnings` before opening a PR.

**Additional style guidance:**

- Favor small, composable functions and explicit types at API boundaries.
- Avoid `unwrap`/`expect` in non-test code; propagate or map errors with context.
- Keep async boundaries clear; avoid blocking calls inside async paths.
- Preserve backward compatibility for public interfaces unless a breaking change is intentional and documented.

## Provider Model-ID Safety Rule (Critical)

When calling a provider's native OpenAI-compatible endpoint (e.g., DeepSeek `https://api.deepseek.com/v1`), always send the provider's raw model ID (e.g., `deepseek-chat`) — **do not auto-add LiteLLM prefixes** (e.g., do *not* rewrite to `deepseek/deepseek-chat`).  
Only apply `provider/model` prefix rewriting when routing through a true LiteLLM-style gateway or aggregator.

**Implementation checklist for provider routing changes:**

- Verify whether the endpoint is native-provider or LiteLLM-compatible gateway.
- Add/adjust tests that assert final outbound `model` value.
- Confirm config migration paths do not silently rewrite model IDs incorrectly.
- Document behavior in crate-level docs or README when introducing new providers.

## Testing Guidelines

Write focused unit tests near the code with `#[cfg(test)]`. Add integration tests in crate `tests/` folders for cross-module behavior. Run `cargo test --all` locally before pushing. Use workspace test utilities (`tokio-test`, `tempfile`, `wiremock`, `mockito`) where appropriate.

**Test quality expectations:**

- Cover both success paths and representative failure paths.
- Validate serialization/config parsing for new config fields.
- Prefer deterministic tests; avoid external network calls in unit/integration tests.
- For async behavior, include timeout-aware assertions to avoid hanging CI.

## Configuration, Secrets, and Safety

- Never commit real secrets or tokens; use environment variables or ignored local config files.
- Redact sensitive fields in logs, errors, snapshots, and test fixtures.
- When adding new config keys, document defaults and migration behavior.
- Prefer least-privilege defaults for channels/tools that execute external actions.

## Observability & Error Handling

- Use structured, actionable error messages with enough context for debugging.
- Preserve source errors (`thiserror`, `anyhow::Context`, or equivalent) rather than discarding them.
- Emit logs at appropriate levels (`trace`/`debug`/`info`/`warn`/`error`) and avoid noisy logs in hot paths.
- Include identifiers (session/channel/provider IDs) where useful, without leaking private data.

## Commit & Pull Request Guidelines

Recent history follows Conventional Commit prefixes (`feat:`, `fix:`, `docs:`); keep using that style with concise imperative summaries. Before PRs, run `just ci`, describe behavioral impact, link related issues, and update docs when interfaces/channels/providers change. Keep PRs focused to a single concern for easier review.

**Recommended PR checklist:**

- Scope is focused and commit history is readable.
- New behavior is covered by tests (or documented rationale if tests are not feasible).
- Backward compatibility and migration impact are addressed.
- User/operator-facing docs are updated when behavior or configuration changes.

## Iteration Protocol (docs/logs)

- Each iteration creates a new directory under `docs/logs` (suggested naming convention: date or theme, e.g., `2026-03-routing-refactor`).
- Each iteration directory contains subdirectories named `v0.0.1-version-slug` (semantic version + readable slug).
- Each version directory should include the following Markdown documents:
  - `summary.md`: Iteration completion summary (what was changed, impact range).
  - `verification.md`: Testing/Verification/validation method and result.
  - `release.md`: Release/deployment method (if not applicable, provide a reason).
  - `acceptance.md`: Acceptance steps from user/product perspective.
- Optional documentation: `prd.md`, `notes.md` (discussion records), `rollback.md` (rollback plan).

## Command Mechanism

- New commands are recorded in `commands/commands.md` and maintain an index in this section (repository does not have this file — create it when adding the first command).
- A meta-command agreement: input `/new-command` to trigger the creation of a new command process.
- Command document structure: name, purpose, input format, output/expected behavior, examples, boundary conditions.
- When adding or modifying commands, update `commands/commands.md` and the section index.

**Current agreed commands:**

- `/new-command`: Create a new command.
- `/config-meta`: Adjust or update the mechanism/metadata of `AGENTS.md`.
- `/check-meta`: Check for conflicts, expired items, or inconsistencies in `AGENTS.md`.
- `/new-rule`: Follow the Rulebook template for adding rules.
- `/commit`: Execute a commit (commit message in English).
- `/validate`: Run the project test, at minimum `just fmt-check`, `just check`, `just test`; if changes involve `agent-diva-gui`, add GUI-specific validation/smoke tests.

## Rulebook Mechanism

- Rules are maintained at the end of this file under the **Rulebook / Project Rulebook** section.
- A meta-command agreement: input `/new-rule` to trigger the creation of a new rule process.
- Rule entries must include: name (English kebab-case), constraints/range of applicability, examples, counterexamples, execution method, maintainer.

**Templates:**

- **<rule-name>**:
  - Constraints/Range of applicability:
  - Examples:
  - Counterexamples:
  - Execution Method:
  - Maintainer:

**Differentiate general rules and project rules:**

- **General rules**: Do not rely on project directory/tools chain/release process — can be migrated to most projects.
- **Project rules**: Depend on project paths, commands, components (like `just`, Rust workspace, `agent-diva-gui`).

By default, all rules are mandatory; if exceptions are needed, they must be explicitly stated within the rule entry.

---

## Rulebook

- **post-dev-stage-validation**:
  - Constraints/Range of applicability: Each development stage must validate, default to `just fmt-check`, `just check`, `just test`. If changes do not affect a certain item, explain the reasons.
  - Example: After modifying the provider, execute these three and record the results.
  - Counterexample: Make code changes and report completion without any validation.
  - Execution Method: Execute validation commands per stage and record commands and results in iteration logs.
  - Maintainer: Owner of the current delivery.

- **smoke-test-required-for-user-visible-change**:
  - Constraints/Range of applicability: For changes involving user-visible or executable behaviors, must have a minimum viable smoke test (CLI/GUI/Channel — at least one test).
  - Example: After changing the CLI parameter behavior, run `just run -- --help` or the corresponding real command to check output.
  - Counterexample: Claim functionality is ready after running only unit tests.
  - Execution Method: Choose the minimum real path for validation and record observation points.
  - Maintainer: Owner of the current delivery.

- **iteration-log-required**:
  - Constraints/Range of applicability: Each deliverable change must record corresponding version and core documents in `docs/logs`.
  - Example: Create `docs/logs/2026-03-provider-fix/v0.2.1-provider-model-id/summary.md`.
  - Counterexample: Finish the iteration with no logs or acceptance records.
  - Execution Method: Check if logs and required documents are complete before delivery.
  - Maintainer: Owner of the current delivery.

- **command-index-must-sync**:
  - Constraints/Range of applicability: When adding or modifying commands, `commands/commands.md` and AGENTS command index must be synchronized.
  - Example: After adding `/triage`, update the command document and this section listing.
  - Counterexample: Implement the command without updating the command index.
  - Execution Method: Include "update command index" in change list and acceptance items.
  - Maintainer: Current assistant.

- **no-self-commit-without-request**:
  - Constraints/Range of applicability: Do not commit or push code without user's explicit request.
  - Example: Commit only after the user explicitly says "help me commit."
  - Counterexample: Commit code without authorization.
  - Execution Method: Confirm explicit user instruction before committing.
  - Maintainer: Current assistant.

- **use-chinese-when-communicating**:
  - Constraints/Range of applicability: Use Chinese in communication with users.
  - Example: Use Chinese for demand clarification, scheme explanation, and feedback.
  - Counterexample: Use English directly to reply to users.
  - Execution Method: Use unison Chinese output.
  - Maintainer: Current assistant.

---

## Project Rulebook

- **rust-workspace-first-validation**:
  - Constraints/Range of applicability: This project focuses on Rust workspace, prioritize using `just`/`cargo` commands for build and validation.
  - Example: Execute `just ci` or `just fmt-check && just check && just test`.
  - Counterexample: Use Node-only validation (like enforce `tsc`) as default standard.
  - Execution Method: Validation scripts and processes are based on `justfile`, supplement crate-level commands as necessary.
  - Maintainer: Owner of the current deliverable.

- **provider-model-id-safety**:
  - Constraints/Range of applicability: When calling a provider's native OpenAI-compatible endpoint (e.g., DeepSeek `https://api.deepseek.com/v1`), use the provider's raw model ID — do not automatically add LiteLLM prefixes.
  - Example: Direct connection to DeepSeek uses `deepseek-chat`, do not rewrite to `deepseek/deepseek-chat`.
  - Counterexample: Uniformly apply `provider/model` prefix rewriting to native endpoints.
  - Execution Method: Supplement test cases when modifying provider routing and assert the final `model` outbound field.
  - Maintainer: Owner of the provider module.

- **gui-changes-need-gui-smoke**:
  - Constraints/Range of applicability: When modifying `agent-diva-gui`, supplement GUI start or key workflow smoke tests in addition to workspace validation.
  - Example: Execute `just start` or equivalent GUI start steps, and record key observation points.
  - Counterexample: Modify the GUI but run Rust tests only.
  - Execution Method: Record GUI smoke test commands and results in `verification.md`.
  - Maintainer: Committer of the GUI change.

---

## reply-prefix-required

- **Constraints/Scope**: All replies to users must begin with the prefix `[I strictly follow the rules]` (effective immediately, including this instruction).
- **Example**: `[I strictly follow the rules] Modification completed.`
- **Counterexample**: Replies without the prefix or only partially including the prefix.
- **Execution Method**: All outputs must include this prefix at the beginning.
- **Maintainer**: Current assistant.