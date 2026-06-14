---
baseline_commit: 77f9a9f61c76d4151eff8e9fcaefe4cf58858b6a
---

# Story 1.5: Expose Laputa Read, Changelog, Rollback, and Event APIs

Status: review

## Story

As a UI or service consumer,
I want stable Laputa operations through Rust, HTTP, and Tauri boundaries,
so that Evolution UI and runtime consumers can read authority without bypassing governance.

## Acceptance Criteria

1. Given Laputa is initialized, when callers request proposal CRUD, apply, snapshot, section, changelog list/detail, rollback, or events, then the Rust API, manager HTTP routes, and Tauri commands expose equivalent behavior.
2. Proposal, changelog, and error events are available through SSE with polling fallback.
3. Section reads for TBD sections return explicit TBD status instead of silently failing.

## Tasks / Subtasks

- [x] Stabilize the `agent-diva-laputa` Rust service API for proposal, apply, snapshot, section, changelog, rollback, and events. (AC: 1)
- [x] Add manager HTTP routes under `/api/laputa/*` that call the Rust API. (AC: 1, 2)
- [x] Add Tauri commands that mirror the manager behavior for GUI consumers. (AC: 1)
- [x] Implement event stream and polling fallback behavior. (AC: 2)
- [x] Add tests for route behavior, Tauri command shape where practical, and explicit TBD section reads. (AC: 1, 2, 3)

## Dev Notes

### Architecture Context

- This story exposes the backend boundary; it does not build the Evolution UI.
- Manager route patterns live in `agent-diva-manager/src/server.rs`; add a focused route group rather than mixing Laputa routes into unrelated provider/planning routes.
- Tauri commands live under `agent-diva-gui/src-tauri/src/`; follow existing command registration patterns.

### Required API Surface

- Proposal CRUD/list and apply.
- Snapshot read and per-section read.
- Changelog list/detail.
- Rollback.
- Proposal/changelog/error events.
- Polling fallback for proposal changes using a `since` style parameter.

### Implementation Guardrails

- Do not read `.laputa/` directly from manager or Tauri handlers; call `agent-diva-laputa`.
- Do not add UI components in this story; Epic 2 owns the Evolution workspace.
- Do not wire prompt/runtime memory injection here; Epic 5 owns runtime consumption.
- HTTP and Tauri surfaces must preserve typed errors rather than collapsing all failures into generic strings.

### Project Structure Notes

- Add `agent-diva-laputa` as a dependency where needed.
- Keep manager handler functions small and delegate to a Laputa service object.
- If manager `AppState` needs a Laputa handle, add it explicitly and keep initialization idempotent.

### Testing Requirements

- Run at minimum:
  - `cargo test -p agent-diva-laputa`
  - `cargo test -p agent-diva-manager laputa` or the closest route-focused test target.
  - `cargo check -p agent-diva-gui` if Tauri command compilation is included in that package.
- Tests must cover:
  - Snapshot includes all 14 sections.
  - TBD sections return explicit `tbd` status.
  - Changelog list/detail and rollback route errors are typed.
  - SSE or event source emits proposal/changelog/error events, with polling fallback available.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.5 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` — manager/Tauri-facing orchestration boundary.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-201 through FR-403 read/event/changelog/rollback contract.
- `agent-diva-manager/src/server.rs` — current HTTP route grouping pattern.
- `agent-diva-gui/src-tauri/src/commands.rs` — current Tauri command location.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Loaded bmad-dev-story workflow, project context, sprint status, Story 1.5, Laputa PRD FR-201 through FR-403, and existing manager/Tauri route patterns.
- 2026-06-14: Added `LaputaService` Rust API for proposals, apply, snapshot/section reads, changelog list/detail, rollback, event recording, SSE subscription, and polling fallback.
- 2026-06-14: Added manager `/api/laputa/*` routes and initialized `LaputaService` in `AppState` from the configured workspace root.
- 2026-06-14: Added Tauri proxy commands for Laputa snapshot, section, proposals, apply, changelog, rollback, and event polling.
- 2026-06-14: Validation passed: `cargo test -p agent-diva-laputa`, `cargo test -p agent-diva-manager build_router_exposes_laputa_routes`, `cargo check -p agent-diva-manager`.
- 2026-06-14: GUI compile validation attempted with `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml` and `cargo check -p agent-diva-gui --no-default-features`; both remain blocked by pre-existing `agent-diva-sandbox` compile errors already tracked in `TODOLIST.md`.

### Completion Notes List

- Added a stable `agent-diva-laputa::LaputaService` facade so Rust callers can use proposal CRUD/list, apply, snapshot, section, changelog list/detail, rollback, event subscription, and polling without touching `.laputa/` directly.
- Added explicit 14-section snapshot and per-section read behavior with TBD sections returning `status=tbd` and no silent null/404 for known TBD sections.
- Added event recording to `.laputa/events.jsonl`, in-process broadcast subscription for SSE, and polling fallback filtered by event kind and `since`.
- Added manager HTTP routes under `/api/laputa/*` for proposals, apply, snapshot, section, changelog, rollback, SSE events, and polling fallback, preserving typed Laputa error responses.
- Added Tauri commands that proxy the same Laputa manager APIs for GUI consumers without adding Evolution UI components.
- Added service and route tests covering all 14 snapshot sections, explicit TBD section reads, changelog list/detail, rollback, proposal/changelog event polling, and manager route exposure.

### File List

- `Cargo.lock`
- `agent-diva-core/src/evolution/types.rs`
- `agent-diva-laputa/Cargo.toml`
- `agent-diva-laputa/src/error.rs`
- `agent-diva-laputa/src/layout.rs`
- `agent-diva-laputa/src/lib.rs`
- `agent-diva-laputa/src/proposals.rs`
- `agent-diva-laputa/src/service.rs`
- `agent-diva-laputa/tests/service.rs`
- `agent-diva-manager/Cargo.toml`
- `agent-diva-manager/src/handlers.rs`
- `agent-diva-manager/src/handlers/laputa.rs`
- `agent-diva-manager/src/runtime.rs`
- `agent-diva-manager/src/runtime/bootstrap.rs`
- `agent-diva-manager/src/runtime/task_runtime.rs`
- `agent-diva-manager/src/server.rs`
- `agent-diva-manager/src/state.rs`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `_bmad-output/implementation-artifacts/1-5-expose-laputa-read-changelog-rollback-and-event-apis.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-laputa-api/v0.0.1-laputa-read-changelog-rollback-events/summary.md`
- `docs/logs/2026-06-laputa-api/v0.0.1-laputa-read-changelog-rollback-events/verification.md`
- `docs/logs/2026-06-laputa-api/v0.0.1-laputa-read-changelog-rollback-events/release.md`
- `docs/logs/2026-06-laputa-api/v0.0.1-laputa-read-changelog-rollback-events/acceptance.md`


### Change Log

- 2026-06-14: Implemented Story 1.5 Laputa Rust API, manager HTTP routes, Tauri commands, event/polling behavior, rollback/read APIs, and moved story to review.
