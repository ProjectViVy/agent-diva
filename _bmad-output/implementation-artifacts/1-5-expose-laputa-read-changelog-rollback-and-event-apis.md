# Story 1.5: Expose Laputa Read, Changelog, Rollback, and Event APIs

Status: ready-for-dev

## Story

As a UI or service consumer,
I want stable Laputa operations through Rust, HTTP, and Tauri boundaries,
so that Evolution UI and runtime consumers can read authority without bypassing governance.

## Acceptance Criteria

1. Given Laputa is initialized, when callers request proposal CRUD, apply, snapshot, section, changelog list/detail, rollback, or events, then the Rust API, manager HTTP routes, and Tauri commands expose equivalent behavior.
2. Proposal, changelog, and error events are available through SSE with polling fallback.
3. Section reads for TBD sections return explicit TBD status instead of silently failing.

## Tasks / Subtasks

- [ ] Stabilize the `agent-diva-laputa` Rust service API for proposal, apply, snapshot, section, changelog, rollback, and events. (AC: 1)
- [ ] Add manager HTTP routes under `/api/laputa/*` that call the Rust API. (AC: 1, 2)
- [ ] Add Tauri commands that mirror the manager behavior for GUI consumers. (AC: 1)
- [ ] Implement event stream and polling fallback behavior. (AC: 2)
- [ ] Add tests for route behavior, Tauri command shape where practical, and explicit TBD section reads. (AC: 1, 2, 3)

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

TBD by dev agent.

### Debug Log References

### Completion Notes List

### File List
