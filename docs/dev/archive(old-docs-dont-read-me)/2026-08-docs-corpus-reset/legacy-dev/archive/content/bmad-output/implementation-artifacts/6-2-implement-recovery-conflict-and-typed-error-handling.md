---
baseline_commit: 8a1114d
---

# Story 6.2: Implement Recovery, Conflict, and Typed Error Handling

Status: review

## Story

As a maintainer,
I want failures to be explicit and recoverable,
so that authority writes do not leave the system in an unknown state.

## Acceptance Criteria

1. Given a write, apply, migration, or rollback fails, when recovery logic runs, then staging data is used to restore the prior safe state where possible.
2. Unresolved merge conflicts mark proposals `needs_attention`.
3. Typed errors include schema incompatible, unauthorized, lock timeout, conflict unresolved, rollback expired, unknown layer, and IO error cases.
4. Errors emit diagnostics for UI and logs.

## Tasks / Subtasks

- [x] Add or extend recovery staging records for apply, migration, and rollback operations. (AC: 1)
- [x] Ensure rollback and apply recovery use the same section lock discipline as normal writes. (AC: 1)
- [x] Add proposal state transition support for unresolved conflicts to `ProposalState::NeedsAttention` where missing. (AC: 2)
- [x] Expand `LaputaError` with stable typed variants and API serialization codes for schema incompatible and unknown layer cases. (AC: 3)
- [x] Preserve existing variants for unauthorized target, lock timeout, conflict unresolved, rollback expired, and IO errors; do not collapse them into strings across HTTP/Tauri. (AC: 3)
- [x] Emit diagnostic events through existing Laputa event plumbing for recoverable failures, unresolved conflicts, and recovery failure. (AC: 4)
- [x] Add tests covering apply failure recovery, migration failure recovery, rollback failure recovery, unresolved conflict proposal state, typed API error shape, and diagnostic event emission. (AC: 1-4)

## Dev Notes

### Architecture Context

- `agent-diva-laputa/src/service.rs` already records proposal, changelog, and error events and writes to `.laputa/events.jsonl`.
- `agent-diva-laputa/src/error.rs` already has `LockTimeout`, `UnauthorizedTarget`, `UnresolvedConflict`, `RollbackExpired`, `RollbackConflict`, and `Io` variants.
- Manager and Tauri surfaces from Story 1.5 must preserve typed errors. Do not regress to plain string errors.
- Laputa PRD requires conflict detection with `needs_attention` event payload containing `conflict_reason`.

### Current Code State

- Rollback currently performs compensation if changelog/audit/proposal updates fail after section write. This story should generalize the recovery model instead of adding one-off fixes.
- `record_error_event` currently sets `status: "needs_attention"` and includes `error_type` and `conflict_reason`; reuse and strengthen that event schema.
- Event writes are locked through the `events` lock; keep that behavior.

### Implementation Guardrails

- Do not silently overwrite section content after conflict. If `expected_before` or `expected_current` does not match and merge cannot be resolved, mark the proposal `needs_attention`.
- Do not create a second error taxonomy in manager or GUI. The canonical source is `LaputaError`.
- Do not perform blocking or long-running recovery in async manager handlers; use Laputa service methods with bounded file operations.
- Recovery must leave previous safe state readable even if audit/changelog writing fails.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-laputa recovery`.
- Add route-level typed error coverage if HTTP serialization changes: `cargo test -p agent-diva-manager laputa`.
- Tests must assert the concrete error code/variant, not only `is_err()`.

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 6.2 requirements.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-105, FR-403, FR-705, FR-706.
- `agent-diva-laputa/src/error.rs` — current typed error enum.
- `agent-diva-laputa/src/service.rs` — apply/rollback/event behavior.
- `agent-diva-manager/src/handlers/laputa.rs` — HTTP typed error surface.
- `agent-diva-gui/src-tauri/src/commands.rs` — Tauri typed error proxy surface.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-15: Ran `cargo test -p agent-diva-laputa recovery`; 6 recovery-filtered tests passed.
- 2026-06-15: Ran `cargo test -p agent-diva-laputa`; all Laputa tests passed.
- 2026-06-15: Ran `cargo test -p agent-diva-manager laputa`; Laputa route tests passed.
- 2026-06-15: Ran `just fmt-check`; passed after rustfmt.
- 2026-06-15: Ran `cargo clippy -p agent-diva-laputa -- -D warnings`; passed.
- 2026-06-15: Ran `just check`; blocked by unrelated `agent-diva-sandbox` clippy errors recorded in `TODOLIST.md`.
- 2026-06-15: Ran `just test`; blocked by unrelated `agent-diva-gui` compile errors recorded in `TODOLIST.md`.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Added file recovery capture/restore for migration commit failures so prior section and state files remain readable.
- Preserved apply and rollback recovery through existing locked service/repository paths, and added service-level diagnostic error events for apply failures.
- Marked unresolved apply conflicts as `needs_attention` and emitted stable typed diagnostics through the Laputa event stream.
- Added canonical `LaputaError::code()` values, including `schema_incompatible`, `unknown_layer`, existing lock/conflict/rollback/IO cases, and manager HTTP serialization.
- Added targeted recovery, conflict, diagnostic, and typed API error tests.

### File List

- `agent-diva-laputa/src/error.rs`
- `agent-diva-laputa/src/memory_provider.rs`
- `agent-diva-laputa/src/migration.rs`
- `agent-diva-laputa/src/proposals.rs`
- `agent-diva-laputa/src/service.rs`
- `agent-diva-laputa/tests/apply.rs`
- `agent-diva-laputa/tests/migration.rs`
- `agent-diva-laputa/tests/service.rs`
- `agent-diva-manager/src/handlers/laputa.rs`
- `agent-diva-manager/src/server.rs`
- `TODOLIST.md`
- `_bmad-output/implementation-artifacts/6-2-implement-recovery-conflict-and-typed-error-handling.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-laputa-recovery-typed-errors/v0.0.1-recovery-conflict-typed-errors/acceptance.md`
- `docs/logs/2026-06-laputa-recovery-typed-errors/v0.0.1-recovery-conflict-typed-errors/release.md`
- `docs/logs/2026-06-laputa-recovery-typed-errors/v0.0.1-recovery-conflict-typed-errors/summary.md`
- `docs/logs/2026-06-laputa-recovery-typed-errors/v0.0.1-recovery-conflict-typed-errors/verification.md`

### Change Log

- 2026-06-15: Implemented recovery staging, conflict attention state, stable typed error codes, diagnostics, and targeted tests for Story 6.2.
