---
baseline_commit: 50f1c7c143e366dda0c1fb7581f9141ef1ecfc51
---

# Story 1.2: Create Laputa File-First Authority Storage

Status: review

## Story

As a maintainer,
I want a dedicated `agent-diva-laputa` crate with file-first storage,
so that durable authority is owned by one implementation boundary.

## Acceptance Criteria

1. Given the workspace Cargo manifest is loaded, when `agent-diva-laputa` is added, then it builds as a workspace crate and owns `.laputa/state.json`, proposals, changelog, audit, rollback staging, migrations, locks, and legacy storage paths.
2. All writes use temp-file plus rename behavior.
3. File locking works on Windows, Linux, and macOS with timeout and stale recovery handling.

## Tasks / Subtasks

- [x] Add `agent-diva-laputa` to the workspace with normal Rust 2021 crate structure. (AC: 1)
- [x] Define file-first storage layout under a caller-provided workspace root. (AC: 1)
- [x] Implement atomic write helpers using temporary files and rename. (AC: 2)
- [x] Implement cross-platform lock abstraction with timeout and stale recovery behavior. (AC: 3)
- [x] Add unit tests with `tempfile` for layout creation, atomic writes, and lock timeout/stale cases. (AC: 1, 2, 3)

## Dev Notes

### Architecture Context

- Laputa is the only authority writer for `.laputa/` state.
- This story creates the storage boundary only. Proposal CRUD, apply gates, read APIs, events, and rollback behavior are Story 1.3 through Story 1.5.
- The storage must be file-first. Do not introduce sqlite, vector DB, graph DB, or external service dependencies.

### Required Storage Responsibilities

- Own these paths beneath the workspace root:
  - `.laputa/state.json`
  - `.laputa/proposals/`
  - `.laputa/changelog/`
  - `.laputa/audit/`
  - `.laputa/rollback/`
  - `.laputa/migrations/`
  - `.laputa/locks/`
  - legacy authority paths used for future migration staging
- Provide typed path helpers so later stories do not hard-code path strings across the codebase.
- Keep state initialization idempotent.

### Implementation Guardrails

- Do not implement proposal state transitions in this story; create only the storage substrate needed by later stories.
- Do not wire Laputa into `agent-diva-manager/src/runtime.rs` yet; runtime consumption is Epic 5.
- Do not add manager routes or Tauri commands; API exposure is Story 1.5.
- Avoid direct writes outside the storage module once the crate exists; centralize write helpers to make direct-write audits practical.

### Project Structure Notes

- Add a workspace member named `agent-diva-laputa`.
- Use `agent_diva_core::evolution` types from Story 1.1. If Story 1.1 is not complete, stop and implement it first.
- Follow existing crate conventions: `src/lib.rs`, small modules, `thiserror` for library errors, no `anyhow` in the library crate API.

### Testing Requirements

- Run at minimum: `cargo test -p agent-diva-laputa`.
- Also run: `cargo check -p agent-diva-laputa`.
- Tests must use temporary directories and no external network.
- Include failure-path tests for rename/write failure where practical and lock timeout behavior.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.2 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` — workspace shape and direct-write ban.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — Laputa file-first scope, write locks, and storage responsibilities.
- `Cargo.toml` — current workspace members.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

- 2026-06-14: Loaded bmad config, project context, sprint status, Story 1.2, Story 1.1 completion context, EVO-DIVA architecture references, Laputa PRD storage/locking requirements, and current workspace manifest.
- 2026-06-14: Implemented `agent-diva-laputa` crate with typed layout helpers, atomic write helpers, lock-file guard, and tempfile-based tests.
- 2026-06-14: Validation passed: `cargo fmt -p agent-diva-laputa -- --check`, `cargo check -p agent-diva-laputa`, `cargo test -p agent-diva-laputa`, and `cargo clippy -p agent-diva-laputa -- -D warnings`.
- 2026-06-14: Workspace-wide `cargo fmt --all -- --check` still fails on pre-existing unrelated formatting drift already captured in `TODOLIST.md`.

### Completion Notes List

- Added the `agent-diva-laputa` workspace crate with Rust 2021 library structure and public storage primitives.
- Added `LaputaPaths` and `LaputaStorage` for caller-rooted `.laputa/` ownership of `state.json`, proposals, changelog, audit, rollback, migrations, locks, legacy staging, and section paths.
- Added atomic persistence helpers that write to same-directory temporary files, fsync the temp file, and rename into place.
- Added `LaputaLock` with cross-platform lock-file creation, timeout polling, and stale file recovery.
- Added focused tests covering idempotent layout creation, atomic replacement, failure-path write errors, lock timeout, and stale lock recovery.

### File List

- `Cargo.toml`
- `agent-diva-laputa/Cargo.toml`
- `agent-diva-laputa/src/lib.rs`
- `agent-diva-laputa/src/error.rs`
- `agent-diva-laputa/src/atomic.rs`
- `agent-diva-laputa/src/layout.rs`
- `agent-diva-laputa/src/lock.rs`
- `agent-diva-laputa/tests/storage.rs`
- `_bmad-output/implementation-artifacts/1-2-create-laputa-file-first-authority-storage.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-laputa-storage/v0.0.1-laputa-file-first-storage/summary.md`
- `docs/logs/2026-06-laputa-storage/v0.0.1-laputa-file-first-storage/verification.md`
- `docs/logs/2026-06-laputa-storage/v0.0.1-laputa-file-first-storage/release.md`
- `docs/logs/2026-06-laputa-storage/v0.0.1-laputa-file-first-storage/acceptance.md`

### Change Log

- 2026-06-14: Implemented Story 1.2 Laputa file-first storage substrate and moved story to review.
