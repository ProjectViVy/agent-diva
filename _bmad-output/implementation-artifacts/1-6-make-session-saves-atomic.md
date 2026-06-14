---
baseline_commit: 89e6711e9d761d82a056e62eaefb42ceb35ffcd9
---

# Story 1.6: Make Session Saves Atomic

Status: review

## Story

As AutoDream and Report System,
I want session history saves to be atomic,
so that reflection and search do not read partially written session files.

## Acceptance Criteria

1. Given `SessionManager::save` writes session history, when save is called, then it writes to a temporary file and renames it into place.
2. Failed writes leave the previous session file readable.
3. Tests cover successful save, interrupted save, and replacement behavior.

## Tasks / Subtasks

- [x] Replace direct session file writes in `SessionManager::save` with temp-file plus rename behavior. (AC: 1)
- [x] Ensure temporary files are created in the same directory as the target file. (AC: 1, 2)
- [x] Preserve existing JSONL format and load behavior. (AC: 2)
- [x] Add failure-path tests that prove old content survives failed replacement. (AC: 2, 3)
- [x] Add regression tests for successful save and replacement. (AC: 1, 3)

## Dev Notes

### Architecture Context

- This story is included in Epic 1 because AutoDream and Report System depend on reliable session history as evidence input.
- Current `agent-diva-core/src/session/manager.rs` uses `std::fs::write(&path, lines.join("\n"))?;`, which can expose partial writes to readers.
- This story is intentionally narrow: only session save atomicity.

### Required Behavior

- Write serialized JSONL content to a temporary sibling file.
- Flush/sync as appropriate for the project's current durability standard, then rename into the final path.
- If serialization or temp write fails, the previous session file must remain readable.
- If there is no previous file and save fails, no corrupt final file should appear.

### Implementation Guardrails

- Do not change the JSONL schema.
- Do not change `SessionManager::load` semantics except as required for tests.
- Do not move session storage paths.
- Do not implement AutoDream or Report System consumers in this story.

### Project Structure Notes

- Primary file: `agent-diva-core/src/session/manager.rs`.
- Tests can live inline near `SessionManager` or in a crate-level test if existing visibility requires it.
- Use `tempfile` from workspace dependencies for isolated tests.

### Testing Requirements

- Run at minimum: `cargo test -p agent-diva-core session`.
- Tests must cover:
  - New session save creates valid JSONL readable by `load`.
  - Replacing an existing session keeps final content valid.
  - Simulated failure before rename leaves previous file readable.
  - No temporary scratch artifacts remain in successful path where practical.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.6 requirements.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` — Report/System and AutoDream evidence reliability requirement.
- `agent-diva-core/src/session/manager.rs` — current `SessionManager::save` implementation.
- `docs/project-context.md` — Rust testing and error-handling conventions.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Loaded bmad config, project context, sprint status, Story 1.6, and `agent-diva-core/src/session/manager.rs` implementation context for atomic save changes.
- 2026-06-14: Replaced direct session writes with same-directory temp-file plus rename behavior, added a test hook for injected pre-rename failure, and added regression/failure-path tests.
- 2026-06-14: Validation passed: `cargo test -p agent-diva-core session`.
- 2026-06-14: Validation passed: `cargo test -p agent-diva-core`.

### Completion Notes List

- Updated `SessionManager::save` to serialize JSONL content into a sibling temporary file, sync it, and rename it into place so interrupted writes do not corrupt the session file.
- Preserved the existing JSONL format and `SessionManager::load` behavior while keeping the temporary file within the target session directory.
- Added tests covering successful replacement, failed replacement preserving the previous file, and cleanup of temporary scratch artifacts.

### File List

- `agent-diva-core/src/session/manager.rs`
- `_bmad-output/implementation-artifacts/1-6-make-session-saves-atomic.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-session-atomic-save/v0.0.1-session-atomic-save/summary.md`
- `docs/logs/2026-06-session-atomic-save/v0.0.1-session-atomic-save/verification.md`
- `docs/logs/2026-06-session-atomic-save/v0.0.1-session-atomic-save/release.md`
- `docs/logs/2026-06-session-atomic-save/v0.0.1-session-atomic-save/acceptance.md`

### Change Log

- 2026-06-14: Implemented Story 1.6 session atomic save behavior and moved story to review.
