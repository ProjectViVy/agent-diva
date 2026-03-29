---
name: agent-diva-workspace-validate
description: Validation and CI workflow for the agent-diva Rust workspace using just and cargo. Use after substantive code changes, before PRs, or when the user asks whether the tree is healthy. Covers just ci, fmt-check, clippy -D warnings, scoped crate tests, optional GUI smoke, RUST_LOG, config/env conventions, and parity with GitHub Actions.
---

# Workspace validation

## Default commands (repository root)

Run from the workspace root. On Windows the `justfile` targets PowerShell.

| Goal | Command |
|------|---------|
| Format | `just fmt` |
| Format check only | `just fmt-check` |
| Clippy (warnings denied) | `just check` |
| All tests | `just test` |
| Full CI gate | `just ci` |
| Run CLI | `just run -- <args>` (e.g. `just run -- gateway run`, `just run -- --help`) |

Without `just`:

```bash
cargo fmt --all
cargo clippy --all -- -D warnings
cargo test --all
```

## Scoped runs (faster feedback)

| Scope | Example |
|-------|---------|
| One crate | `cargo test -p agent-diva-manager` |
| Test name filter | `cargo test -p agent-diva-core message_bus` |
| Stdout from tests | `cargo test -- --nocapture` |

## When to run what

- **Any Rust change:** at minimum `just fmt-check && just check && just test` (or `just ci`).
- **CLI or user-visible behavior:** add smoke: `just run -- --help` or the exact subcommand you touched.
- **`agent-diva-gui` changes:** workspace checks **plus** GUI start / critical path smoke (project rule: GUI changes need GUI smoke).
- **Provider / HTTP wiring:** prefer tests that mock HTTP (`mockito`, `wiremock` per crate); avoid live network in unit tests.

## Observability while debugging

- `RUST_LOG=debug` (or `trace`) with `cargo run` / `just run -- ...` for verbose `tracing` output.

## Configuration reminders

- User config file: `~/.agent-diva/config.json`.
- Environment overrides: `AGENT_DIVA__...` (nested keys use double-underscore convention; document new keys and defaults when changing schema).

## CI parity

- Root pipeline: `.github/workflows/ci.yml` — align local gates with what CI runs; if you add a new mandatory step in CI, update this skill’s table or developer docs in the same change.

## Iteration logs (project process)

For tracked deliverables, iteration artifacts live under `docs/logs/<theme>/vX.Y.Z-slug/` with `summary.md`, `verification.md`, `release.md`, `acceptance.md` per `AGENTS.md`. Record **which commands** you ran and **pass/fail** in `verification.md` when that process applies.

Do not claim work is complete without the relevant checks unless the user explicitly scoped an exception.

## Related skills

- **`agent-diva-extend-integrations`**: checklist after provider/channel/tool changes.
- **`agent-diva-manager-gateway`**: focused tests/smoke for the HTTP gateway crate.
