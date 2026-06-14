---
baseline_commit: 8a1114d
---

# Story 6.3: Add Direct-Write and Filesystem Tests

Status: ready-for-dev

## Story

As a maintainer,
I want tests that prove authority writes are constrained,
so that future work does not bypass Laputa accidentally.

## Acceptance Criteria

1. Given the test suite runs, when direct-write guard tests inspect EVO-DIVA flows, then unauthorized writes to durable authority paths fail the test.
2. Direct-read guard tests fail when runtime prompt assembly reads legacy authority files directly instead of the Laputa read boundary.
3. Filesystem tests cover atomic writes, lock timeout, stale lock recovery, staging recovery, and changelog/audit creation.
4. Tests run on Windows-compatible paths.

## Tasks / Subtasks

- [ ] Add repository-level direct-write guard tests or scripts that scan EVO-DIVA crates for raw writes to `.laputa`, legacy authority files, and prompt authority paths outside `agent-diva-laputa`. (AC: 1)
- [ ] Add direct-read guard tests for runtime prompt assembly, context assembly, Mentle integration, AutoDream, and Report System solidification boundaries. (AC: 2)
- [ ] Extend `agent-diva-laputa/tests/storage.rs` or add focused integration tests for atomic writes, lock timeout, stale lock recovery, staging recovery, and changelog/audit creation. (AC: 3)
- [ ] Include Windows-safe fixture paths and avoid assumptions about symlinks, `/tmp`, or Unix-only lock behavior. (AC: 4)
- [ ] Document any intentional allowed filesystem access with an allowlist that is narrow and reviewed in tests. (AC: 1, 2)
- [ ] Wire targeted tests into the Epic 6 release-gate story without requiring full GUI startup. (AC: 1-4)

## Dev Notes

### Architecture Context

- Durable subject-continuity writes must go through Laputa. This is an EVO-DIVA governance invariant, not only an implementation preference.
- Existing runtime prompt/context code lives under `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/agent_loop/*`, `agent-diva-agent/src/mentle_runtime.rs`, and compaction modules.
- AutoDream must write only `.agent-diva/autodream/*` and create proposals through Laputa APIs; it must not write authority files directly.
- Report System solidification belongs to proposal creation, not direct MEMORY/SOP/Skill writes.

### Current Code State

- `agent-diva-laputa/src/atomic.rs` and `agent-diva-laputa/src/lock.rs` already provide the primitives to test.
- Existing Laputa integration tests live in `agent-diva-laputa/tests/`.
- The workspace is currently dirty with unrelated changes; keep new tests scoped and do not rewrite existing test organization.

### Implementation Guardrails

- Prefer structured Rust tests for behavior and a small allowlist-based scanner for static direct-access rules.
- Do not make the scanner fail on references in docs, story files, or tests that intentionally mention paths.
- The allowlist should permit `agent-diva-laputa` storage internals and explicit test fixtures only.
- Tests should fail with actionable messages that name the offending file and forbidden access pattern.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-laputa`.
- Add the guard test to the smallest crate where it can inspect repo paths deterministically; if it lives outside a crate, expose a `just` or cargo test target in the release gate.
- Include lock timeout and stale lock recovery assertions that are not flaky under Windows path semantics.

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 6.3 requirements.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` — FR-201 direct-write detection and FR-203 read-boundary rule.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-106, FR-601, FR-602, FR-704.
- `agent-diva-laputa/src/atomic.rs` — atomic write primitive.
- `agent-diva-laputa/src/lock.rs` — lock timeout/stale recovery primitive.
- `agent-diva-agent/src/context.rs` — runtime context assembly boundary to guard.

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.

### File List
