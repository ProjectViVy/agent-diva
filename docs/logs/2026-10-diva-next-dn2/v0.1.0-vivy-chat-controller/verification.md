# Verification — DN-2 v0.1.0

Environment: linux/amd64, Node 22.20.0 (`~/toolchains/node/bin`),
pnpm, vue-tsc. No provider key — all transport-level tests use the
contract-verified fake `VivyTransport`.

## Commands run

- `npx vue-tsc --noEmit` → clean (exit 0).
- `npx vitest run` → 69 files, 519/519 tests pass.
  - `state/vivy-chat.test.ts` (19 tests): sessions list/history/switch;
    send → turn/start → run/subscribe → streaming text; terminal-only
    answer; reasoning + tool card; cancel → run/cancel + cancelled
    resolves typing; completion-wins cancel race; ambiguous send
    timeout (landed → echo dropped + adopted run; not landed → echo
    kept + isTyping false); approval list/approve/deny via
    review/respond; rejection displayed; double decision; timeout →
    outcomeUnknown; pending approval survives reopen (snapshot
    rebuild); expiry event refresh; question answer + cancel;
    plan pending → AwaitingApproval card; `plan/decide` params
    `{session_id, submission_id, action, expected_version}`; reject
    with no pending submission.
  - `state/vivy-session.test.ts` (10 tests): projection dedup,
    cursor/gap handling, snapshot authority, pending-approval rebuild.
- `pnpm build` → clean (dist emitted; pre-existing chunk-size warnings
  only).

## Intentionally not run

- Packaged live-model chain (DN-2 task 5) — needs provider key.
- UI-level browser exercise — no provider; `testing-vivy-ui` frozen-env
  trick remains available on request.
- cargo/just gates — governance docs still describe the deleted Rust
  workspace (GOVERNANCE-DOCS-STALE); the new Tauri shell's own crates
  were verified in DN-5 and untouched this wave.
