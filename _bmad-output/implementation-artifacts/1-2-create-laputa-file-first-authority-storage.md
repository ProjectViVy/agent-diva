# Story 1.2: Create Laputa File-First Authority Storage

Status: ready-for-dev

## Story

As a maintainer,
I want a dedicated `agent-diva-laputa` crate with file-first storage,
so that durable authority is owned by one implementation boundary.

## Acceptance Criteria

1. Given the workspace Cargo manifest is loaded, when `agent-diva-laputa` is added, then it builds as a workspace crate and owns `.laputa/state.json`, proposals, changelog, audit, rollback staging, migrations, locks, and legacy storage paths.
2. All writes use temp-file plus rename behavior.
3. File locking works on Windows, Linux, and macOS with timeout and stale recovery handling.

## Tasks / Subtasks

- [ ] Add `agent-diva-laputa` to the workspace with normal Rust 2021 crate structure. (AC: 1)
- [ ] Define file-first storage layout under a caller-provided workspace root. (AC: 1)
- [ ] Implement atomic write helpers using temporary files and rename. (AC: 2)
- [ ] Implement cross-platform lock abstraction with timeout and stale recovery behavior. (AC: 3)
- [ ] Add unit tests with `tempfile` for layout creation, atomic writes, and lock timeout/stale cases. (AC: 1, 2, 3)

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

### Completion Notes List

### File List
