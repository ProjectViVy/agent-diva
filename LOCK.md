# LOCK

Codex/Cursor/manual parallel session mutex file.
Use this file to declare the current writer scope before mutating the workspace.

## Status

- Lock State: `FREE`
- Scope: `none`
- Owner: `none`
- Session/Task: `none`
- Branch/Worktree: `agent-diva-pro / C:\\Users\\Administrator\\Desktop\\morediva\\agent-diva`
- Started At: `2026-07-12T22:00:00+08:00`
- Last Heartbeat: `2026-07-12T00:15:00+08:00`
- Expires At: `none`

## Lock Rules

1. Read this file before editing files, running worktree-mutating commands, or preparing a commit.
2. If `Lock State` is `HELD` and the heartbeat is still valid, do not modify files covered by `Scope`.
3. If work must proceed in parallel, prefer an isolated `git worktree` or separate branch and still record the new scope here.
4. `Scope` must name concrete files, directories, or modules.
5. Refresh `Last Heartbeat` at least every 30 minutes while holding the lock.
6. Record handoff notes before taking over a stale lock.
7. Use `GLOBAL` only for repo-wide migrations, bulk formatting, or similarly broad work.

## Active Lock

- none

## Handoff Notes

- `2026-07-12`: Released GUI streamed-response truncation fix. The agent loop flushes the protocol guard's safe suffix and the GUI reconciles to the authoritative final response. Validation: agent protocol-guard unit tests and GUI production build passed; full agent integration tests are blocked by pre-existing provider-trait mock drift.
- `2026-07-12`: Released tool follow-up lock. Agent loop grants one summary-only pass after tools at max iterations and replaces English empty-final with Chinese synthesis. Unit tests passed.
- `2026-07-12`: Released streaming UTF-8 boundary fix. OpenAI-compatible, Anthropic, and Ollama now retain incomplete UTF-8 across HTTP chunks; `cargo test -p agent-diva-providers --lib` passed (108 tests).
- `2026-07-12`: Released tool protocol leak guard. Summary-only requests explicitly send `tool_choice: none`; DSML/XML-like internal tool protocols are blocked from streamed and final output. Commit: `50506e2`.
- `2026-07-12`: Released DeepSeek V4/DSML compatibility research document. Commit pending.
- `2026-07-12`: Released DSML guard evolution documentation update. Commit pending.
- `2026-07-12`: Released DeepSeek V4 DSML adapter. Adds explicit response protocol configuration, strict DSML decoding, stream isolation, and summary-only tool disabling. Validation: provider/agent checks and targeted unit tests passed; workspace fmt-check remains blocked by pre-existing unrelated formatting drift.
- `2026-07-12`: Released DeepSeek V4-Pro/V4-Flash default model prioritization. Registry unit test passed.
- `2026-07-12`: Released GUI provider model-selection persistence fix. Clicking a model now saves the active model and shortcut list immediately. Validation: GUI test suite (47 files, 400 tests) and production build passed.
