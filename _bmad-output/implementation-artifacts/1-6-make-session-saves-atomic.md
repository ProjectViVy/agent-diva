# Story 1.6: Make Session Saves Atomic

Status: ready-for-dev

## Story

As AutoDream and Report System,
I want session history saves to be atomic,
so that reflection and search do not read partially written session files.

## Acceptance Criteria

1. Given `SessionManager::save` writes session history, when save is called, then it writes to a temporary file and renames it into place.
2. Failed writes leave the previous session file readable.
3. Tests cover successful save, interrupted save, and replacement behavior.

## Tasks / Subtasks

- [ ] Replace direct session file writes in `SessionManager::save` with temp-file plus rename behavior. (AC: 1)
- [ ] Ensure temporary files are created in the same directory as the target file. (AC: 1, 2)
- [ ] Preserve existing JSONL format and load behavior. (AC: 2)
- [ ] Add failure-path tests that prove old content survives failed replacement. (AC: 2, 3)
- [ ] Add regression tests for successful save and replacement. (AC: 1, 3)

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

TBD by dev agent.

### Debug Log References

### Completion Notes List

### File List
