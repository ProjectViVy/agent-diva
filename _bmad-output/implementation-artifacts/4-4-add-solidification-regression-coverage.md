---
baseline_commit: 63d1ea7
---

# Story 4.4: Add Solidification Regression Coverage

Status: review

## Story

作为维护者，
我希望测试证明旧的直接写入已经移除，
以便 Report 和 Notebook 不会在治理边界上回退。

## Acceptance Criteria

1. Given legacy commands such as `solidify_report_as_sop`, `solidify_report_as_skill`, and `update_memory_from_report` exist or are migrated, when tests exercise those flows, then they create proposals or return migration-compatible proposal responses.
2. Authority writes occur only after Laputa apply.
3. Direct-write grep guards fail on new EVO-DIVA direct writes outside Laputa.

## Tasks / Subtasks

- [x] Add regression tests for legacy Notebook command names and/or their replacement APIs, proving the observable result is proposal creation, not direct authority mutation. (AC: 1)
- [x] Add filesystem assertions around SOP, Skill, memory, identity, and legacy authority paths to prove no files change before `apply_proposal`. (AC: 2)
- [x] Add an apply-path control test proving the same proposed change only reaches authority after Laputa approval/apply. (AC: 2)
- [x] Add or extend grep/static guards for EVO-DIVA direct writes outside `agent-diva-laputa`, allowing explicit test fixtures and migration-only exceptions. (AC: 3)
- [x] Include GUI/Tauri bridge coverage if legacy command names remain registered for compatibility. (AC: 1)
- [x] Record any remaining allowed direct-write exceptions in `TODOLIST.md` only if they cannot be fixed in this story. (AC: 3)

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` test plan requires Report and Notebook tests for daily/weekly path consumption, monthly isolation, SOP proposal creation, Skill proposal creation, and memory proposal creation.
- The readiness checklist requires Notebook solidification to create proposals before the implementation can be considered ready.
- Epic 6 has broader direct-write filesystem tests, but this story owns the Report/Notebook-specific regression coverage for solidification flows.

### Current Code State

- `agent-diva-gui/src/components/NotebookView.vue` still references `solidify_report_as_sop`, `solidify_report_as_skill`, and `update_memory_from_report`.
- `rg` did not find registered backend implementations for those exact command names in `agent-diva-gui/src-tauri/src`, which means the current UI path is either broken, stubbed elsewhere, or awaiting migration.
- `agent-diva-gui/src-tauri/src/commands.rs` already registers Laputa proposal proxy commands; tests should target those paths after Story 4.2 migration.
- `agent-diva-laputa` is the only authority write boundary. Anything outside it should create proposals, read reports, or collect evidence only.

### Implementation Guardrails

- Do not loosen the governance boundary to make tests easy. If a test needs authority mutation, drive it through approve/apply.
- Grep guards must be precise enough to avoid false positives on tests/docs while still catching new production writes to authority paths.
- Keep compatibility behavior explicit: if old command names remain, their response should communicate proposal creation/migration, not direct success.
- Do not mark Epic 4 done from this story; completion requires implementation and review of all 4.x stories.

### Testing Requirements

- Required coverage:
  - SOP proposal creation from report;
  - Skill proposal creation from report;
  - memory proposal creation from report;
  - no pre-apply writes to authority files;
  - post-apply writes happen through Laputa;
  - grep/static guard for direct writes outside Laputa.
- Minimum validation:
  - targeted Rust tests for report solidification proposal flow
  - `cargo test -p agent-diva-laputa`
  - GUI/Tauri validation if command registration or `NotebookView.vue` changes
  - `just fmt-check` and `just check` if feasible for the final implementation pass

### References

- `_bmad-output/planning-artifacts/epics.md` - Epic 4 Story 4.4.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - sections 13.2, 13.5, readiness checklist.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` - GOV-FR801-GOV-FR805 and Report System governance requirements.
- `_bmad-output/implementation-artifacts/4-2-replace-notebook-direct-solidification-with-proposal-creation.md` - migrated solidification semantics.
- `agent-diva-gui/src/components/NotebookView.vue` - legacy command call sites.
- `agent-diva-gui/src-tauri/src/commands.rs` - Tauri Laputa proxy command surface.

## Previous Story Intelligence

- Stories 4.1-4.3 should establish report loading, proposal creation, and session evidence search. This story should test those flows together rather than creating new behavior.
- Story 6.3 will broaden direct-write and filesystem tests. Keep this story scoped to Report/Notebook solidification regressions and coordinate with the later Epic 6 guardrails.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-15: Story context prepared from Epic 4, architecture test plan, current Notebook legacy command references, and existing Laputa proxy APIs.
- 2026-06-16: Added Notebook UI regression coverage for preview/create proposal commands, explicit pre-apply authority path assertions in Notebook tests, and extended the Laputa direct-write static guard to scan Tauri notebook sources.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Verified Notebook proposal actions use `preview_notebook_report_proposal` and `create_notebook_report_proposal` rather than legacy direct-write command names.
- Added pre-apply filesystem assertions proving Notebook proposal preparation does not touch SOP, Skill, memory, identity, or Laputa authority files.
- Reused existing Laputa apply-path proof coverage and extended the static direct-write guard to include `agent-diva-gui/src-tauri/src`.
- No additional unresolved direct-write exceptions were discovered, so `TODOLIST.md` did not require a new entry for this story.

### File List

- `_bmad-output/implementation-artifacts/4-4-add-solidification-regression-coverage.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-gui/src/components/NotebookView.test.ts`
- `agent-diva-gui/src-tauri/src/notebook.rs`
- `agent-diva-laputa/tests/direct_write_guard.rs`
- `docs/logs/2026-06-solidification-regression-coverage/v0.0.1-solidification-regression-coverage/summary.md`
- `docs/logs/2026-06-solidification-regression-coverage/v0.0.1-solidification-regression-coverage/verification.md`
- `docs/logs/2026-06-solidification-regression-coverage/v0.0.1-solidification-regression-coverage/release.md`
- `docs/logs/2026-06-solidification-regression-coverage/v0.0.1-solidification-regression-coverage/acceptance.md`

### Change Log

- 2026-06-15: Created ready-for-dev story for Report/Notebook solidification regression coverage.
- 2026-06-16: Added Notebook/Tauri regression coverage, pre-apply filesystem assertions, and direct-write static guard coverage; moved story to review.
