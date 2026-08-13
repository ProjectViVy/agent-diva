---
baseline_commit: 5d072f6
---

# Story 3.1: Implement Manual Run Lifecycle

Status: review

## Story

As a user,
I want to trigger, inspect, and cancel AutoDream manually,
so that reflection is explicit before any automatic rhythm is enabled.

## Acceptance Criteria

1. Given no AutoDream lock exists, when a manual run is triggered, then AutoDream creates a run record, writes a lock file, returns a run ID, and reports status through API/Tauri commands.
2. Cancel requests stop the run and update status.
3. Stale locks older than the configured threshold recover safely.
4. Auto/session-threshold mode remains off by default.

## Tasks / Subtasks

- [x] Add `agent-diva-autodream` as a Rust 2021 workspace crate and wire it into the root `Cargo.toml`. (AC: 1)
- [x] Implement AutoDream storage layout helpers for `.agent-diva/autodream/lock`, `checkpoint`, `events.jsonl`, and `runs/{run_id}/`. (AC: 1, 3)
- [x] Define run lifecycle types around existing `agent_diva_core::evolution::AutoDreamRunRecord` without duplicating incompatible domain contracts. (AC: 1, 2)
- [x] Implement manual trigger, status lookup, cancel, run record persistence, lock acquisition, stale lock recovery, and default-off auto mode. (AC: 1-4)
- [x] Add manager HTTP routes: `POST /api/autodream/runs`, `GET /api/autodream/runs/{id}`, `POST /api/autodream/runs/{id}/cancel`, and `GET /api/autodream/runs`. (AC: 1, 2)
- [x] Add Tauri bridge commands: `trigger_autodream`, `get_autodream_run_status`, `cancel_autodream_run`, and `list_autodream_run_records`. (AC: 1, 2)
- [x] Add unit/integration tests for manual run creation, duplicate active lock rejection, cancellation, stale lock recovery, and default-off auto/session-threshold mode. (AC: 1-4)

## Dev Notes

### Architecture Context

- Epic 3 implements Phase 2 from `docs/architecture/evo-diva-architecture-2026-06-12.md`: create `agent-diva-autodream`, then add lock/checkpoint/run records before restricted inputs and outputs.
- AutoDream is a proposal and report producer, not an authority writer. It must never write `.laputa/state.json`, legacy `MEMORY.md`, report-system monthly paths, or Mentle databases.
- Existing authority types live in `agent-diva-core/src/evolution/types.rs`. Reuse `AutoDreamRunRecord` and `AutoDreamRunState`; extend only if needed through compatible fields or crate-local DTOs.
- Existing Laputa routes are registered in `agent-diva-manager/src/server.rs`; add an `autodream_routes()` group rather than mixing routes into unrelated handlers.
- Existing Tauri Manager API bridge patterns live in `agent-diva-gui/src-tauri/src/commands.rs` and registration in `agent-diva-gui/src-tauri/src/lib.rs`.

### Current Code State

- `agent-diva-autodream/` does not exist yet.
- Root `Cargo.toml` includes `agent-diva-laputa` but not AutoDream.
- Manager already exposes Laputa routes under `/api/laputa/*`; follow that route grouping and response-shape discipline for `/api/autodream/*`.
- Tauri currently registers Laputa commands but no AutoDream commands.

### Implementation Guardrails

- Manual runs first. Do not enable auto/session-threshold triggering by default in this story.
- Lock file path is `.agent-diva/autodream/lock`; checkpoint path is `.agent-diva/autodream/checkpoint`.
- Lock content should include run ID, PID where available, trigger, and started timestamp so stale recovery can be audited.
- Cancellation must produce a terminal run status and should not update success checkpoint.
- Do not implement reflection prompts, input collection, proposals, or reports here; later 3.x stories own those.
- Do not introduce broad shell execution or Mentle dependencies.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-autodream`.
- Also run `cargo check -p agent-diva-manager` and GUI Tauri command compile validation after route/command wiring.
- Tests must use temporary workspace roots and must not depend on real `.agent-diva` data.

## Previous Story Intelligence

- Epic 1 created and completed the governance/Laputa backend contract. AutoDream should create proposals through Laputa APIs only after Story 3.4; it must not bypass `agent-diva-laputa`.
- Story 1.6 made session saves atomic. Story 3.2 can rely on session files being safer to read, but should still treat corrupt or missing sessions as non-fatal input omissions.

## Completion Note

Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Loaded `bmad-dev-story` workflow, project context, sprint status, and Story 3.1 requirements.
- 2026-06-14: Implemented `agent-diva-autodream` with file-first storage, manual run trigger/status/cancel/list APIs, stale lock recovery, events, and default-off checkpoint flags.
- 2026-06-14: Wired manager HTTP routes and Tauri bridge commands for manual AutoDream lifecycle operations.
- 2026-06-14: `cargo test -p agent-diva-autodream` and manager AutoDream route smoke passed; `cargo fmt --check` and GUI Tauri compile validation are blocked by pre-existing unrelated rustfmt/sandbox issues recorded in `TODOLIST.md`.

### Completion Notes List

- Created the Rust 2021 `agent-diva-autodream` workspace crate and added it to workspace membership and manager dependencies.
- Reused `agent_diva_core::evolution::AutoDreamRunRecord` and extended `AutoDreamRunState` with `Cancelled` for explicit cancel terminal state.
- Implemented `.agent-diva/autodream/lock`, `checkpoint`, `events.jsonl`, and `runs/{run_id}/record.json` storage helpers.
- Added manual run creation, active lock rejection, status lookup, cancellation, stale lock recovery, run listing, event append, and default-off auto/session-threshold checkpoint behavior.
- Added manager routes under `/api/autodream/*` and Tauri bridge commands plus frontend API wrappers.
- Added integration tests covering manual creation, duplicate active lock rejection, cancellation, stale lock recovery, and default-off modes.

### File List

- `Cargo.lock`
- `Cargo.toml`
- `TODOLIST.md`
- `_bmad-output/implementation-artifacts/3-1-implement-manual-run-lifecycle.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-autodream/Cargo.toml`
- `agent-diva-autodream/src/atomic.rs`
- `agent-diva-autodream/src/error.rs`
- `agent-diva-autodream/src/layout.rs`
- `agent-diva-autodream/src/lib.rs`
- `agent-diva-autodream/src/service.rs`
- `agent-diva-autodream/tests/service.rs`
- `agent-diva-core/src/evolution/types.rs`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva-gui/src/api/desktop.ts`
- `agent-diva-manager/Cargo.toml`
- `agent-diva-manager/src/handlers.rs`
- `agent-diva-manager/src/handlers/autodream.rs`
- `agent-diva-manager/src/runtime/task_runtime.rs`
- `agent-diva-manager/src/server.rs`
- `agent-diva-manager/src/state.rs`
- `docs/logs/2026-06-autodream-manual-run-lifecycle/v0.0.1-manual-run-lifecycle/acceptance.md`
- `docs/logs/2026-06-autodream-manual-run-lifecycle/v0.0.1-manual-run-lifecycle/release.md`
- `docs/logs/2026-06-autodream-manual-run-lifecycle/v0.0.1-manual-run-lifecycle/summary.md`
- `docs/logs/2026-06-autodream-manual-run-lifecycle/v0.0.1-manual-run-lifecycle/verification.md`

### Change Log

- 2026-06-14: Implemented manual AutoDream run lifecycle and moved story to review.
