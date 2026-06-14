---
baseline_commit: 5d072f6
---

# Story 3.5: Generate Daily and Weekly Reports

Status: ready-for-dev

## Story

As a user,
I want AutoDream to generate daily and weekly reports,
so that Report System can display reflection summaries without owning those outputs.

## Acceptance Criteria

1. Given a manual report run succeeds, when daily or weekly report output is requested, then daily reports write to `.agent-diva/autodream/reports/daily/{date}.md`.
2. Weekly reports write to `.agent-diva/autodream/reports/weekly/{week}.md`.
3. Reports include evidence refs suitable for proposal creation.
4. Report System can read the files without creating authority writes.

## Tasks / Subtasks

- [ ] Add report generation/write APIs under `agent-diva-autodream/src/reports.rs` or `outputs.rs`. (AC: 1, 2)
- [ ] Implement daily report path writes: `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`. (AC: 1)
- [ ] Implement weekly report path writes: `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`. (AC: 2)
- [ ] Include v1 frontmatter with `period`, `date` or `week`, `generated_at`, `generated_by: agent-diva-autodream`, and `schema_version: 1`. (AC: 1, 2)
- [ ] Include bounded evidence references in the markdown body or metadata so later Notebook solidification can create proposals. (AC: 3)
- [ ] Write reports atomically using temp-file plus rename and parent-directory sync where supported. (AC: 1, 2)
- [ ] Add tests proving daily/weekly path writes, frontmatter schema, evidence refs, atomic replacement, and no monthly/report-system authority writes. (AC: 1-4)

## Dev Notes

### Architecture Context

- AutoDream owns daily/weekly report generation only. Report System owns display, monthly reports, solidification, and search.
- The path contract is fixed by `docs/architecture/autodream-architecture-2026-06-12.md` ADR-008 and `docs/architecture/evo-diva-architecture-2026-06-12.md` §7.
- Daily path: `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`.
- Weekly path: `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`.
- Monthly reports are explicitly out of AutoDream scope.

### Current Code State

- Report System/Notebook migration belongs to Epic 4. This story should write files that Report System can consume later, not modify Notebook display semantics.
- Existing session atomic-write behavior from Story 1.6 is the durability pattern to copy for report writes.

### Implementation Guardrails

- Do not modify Report System monthly storage.
- Do not directly create memory/SOP/Skill authority from report contents. Any later solidification must become proposals in Epic 4.
- Do not require NotebookView to import AutoDream internals; the contract is file paths and markdown schema.
- Validate report file names and dates; reject malformed week/date keys rather than writing unexpected paths.
- Keep report content bounded enough for GUI display and later evidence linking.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-autodream reports`.
- Include tests for daily and weekly frontmatter.
- Include negative tests for monthly period rejection and path traversal rejection.
- Add a lightweight readback smoke showing another component can read the markdown file without AutoDream internals.

## Previous Story Intelligence

- Story 3.4 should have established event and run-output artifact writing. This story may reuse artifact evidence refs but must keep report generation separate from proposal application.
- Epic 4 will consume these paths and migrate Notebook solidification to proposal creation.

## Completion Note

Ultimate context engine analysis completed - comprehensive developer guide created.
