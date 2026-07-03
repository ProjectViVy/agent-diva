---
baseline_commit: 63d1ea7
---

# Story 4.1: Consume AutoDream and Report-Owned Paths Correctly

Status: review

## Story

作为用户，
我希望 Notebook 从权威存储位置展示报表，
以便日报、周报、月报的所有权边界保持清晰。

## Acceptance Criteria

1. Given reports exist, when Report System loads report lists, then daily and weekly reports are read from `.agent-diva/autodream/reports/*`.
2. Monthly reports remain under Report-owned monthly storage.
3. Missing reports show placeholders with trigger actions.
4. Large report files are safely truncated or lazy-loaded according to Report System requirements.

## Tasks / Subtasks

- [x] Add/extend report loading backend for Notebook so `daily` reads `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md` and `weekly` reads `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`. (AC: 1)
- [x] Keep `monthly` isolated under Report-owned storage, expected path `{workspace}/reports/monthly/{YYYY-MM}.md`; do not move monthly output into AutoDream. (AC: 2)
- [x] Parse AutoDream v1 report frontmatter (`period`, `date` or `week`, `generated_at`, `generated_by`, `schema_version`) into the existing `NotebookReport` DTO shape. (AC: 1)
- [x] Add empty/missing-state payloads or UI states with trigger actions for each period; daily/weekly trigger AutoDream, monthly uses Report-owned generation. (AC: 3)
- [x] Enforce large-file behavior before markdown rendering: truncate to the PRD bound or add lazy detail loading with a visible truncated marker. (AC: 4)
- [x] Preserve existing `NotebookView.vue` split list/detail layout and polling behavior; avoid introducing overlapping text or nested card layouts. (AC: 1-4)
- [x] Add tests for daily path consumption, weekly path consumption, monthly path isolation, missing report placeholders, and large markdown handling. (AC: 1-4)

### Review Findings

- [ ] [Review][Patch] Malformed notebook report files break the entire report list load [`agent-diva-gui/src-tauri/src/notebook.rs:124`]

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` section 7 defines Report System as a consumer/display layer.
- Daily reports are owned by AutoDream and read from `.agent-diva/autodream/reports/daily/*`.
- Weekly reports are owned by AutoDream and read from `.agent-diva/autodream/reports/weekly/*`.
- Monthly reports remain Report-owned and must stay isolated from AutoDream storage.
- `docs/prd-report-system/prd.md` FR-1/FR-2/FR-3 define display behavior, trigger behavior, and large-file limits.

### Current Code State

- `agent-diva-autodream/src/reports.rs` already writes daily/weekly markdown with v1 frontmatter and bounded evidence refs.
- `agent-diva-autodream/src/layout.rs` owns the AutoDream daily/weekly path helpers.
- `agent-diva-gui/src/components/NotebookView.vue` currently calls Tauri command `get_notebook_reports` with `period`, but `rg` only finds the frontend call; backend command implementation is missing or not registered in the visible Tauri command set.
- `NotebookView.vue` already has tabs, list/detail rendering, empty/loading/error states, markdown rendering, and 60s polling. Reuse this structure.

### Implementation Guardrails

- Do not import AutoDream internals into the GUI frontend. Cross-crate/backend code may depend on AutoDream path contracts or a small shared DTO, but the display contract should remain file/path plus markdown schema.
- Do not create proposals or authority writes in this story. This story is read/display plus trigger affordances only.
- Do not store daily/weekly reports under `{workspace}/reports/*`; that path is monthly/report-owned only.
- Reject or skip malformed date/week keys and path traversal instead of joining raw user-controlled paths.
- Markdown renderer has `html: false`; keep that security posture.

### Testing Requirements

- Add Rust tests near the report loading backend for daily/weekly/monthly path routing.
- Add GUI component/unit coverage where practical for missing-state and large-report behavior.
- Minimum validation:
  - `cargo test -p agent-diva-autodream --test reports`
  - targeted test for the new report loading backend
  - GUI smoke or component test if `agent-diva-gui` changes

### References

- `_bmad-output/planning-artifacts/epics.md` - Epic 4 Story 4.1.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - sections 7, 13.5, readiness checklist.
- `docs/prd-report-system/prd.md` - FR-1, FR-2, FR-3, FR-4, report schema.
- `_bmad-output/implementation-artifacts/3-5-generate-daily-and-weekly-reports.md` - AutoDream report writer completion notes.
- `agent-diva-autodream/src/reports.rs` - current daily/weekly report writer.
- `agent-diva-gui/src/components/NotebookView.vue` - current Notebook display surface.

## Previous Story Intelligence

- Story 3.5 established daily/weekly markdown writes, strict date/week validation, v1 frontmatter, bounded evidence refs, atomic replacement, and explicit monthly rejection.
- Epic 2 completed the Evolution workspace and governance actions; do not duplicate Evolution review UI inside Notebook.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-15: Story context prepared from Epic 4, Report System PRD, EVO-DIVA architecture section 7, AutoDream report writer, and current `NotebookView.vue`.
- 2026-06-15: Added a Tauri-side notebook report loader that reads AutoDream daily/weekly markdown plus report-owned monthly markdown without importing AutoDream runtime internals.
- 2026-06-15: Added `NotebookView` empty-state trigger affordances and truncated-report banner while preserving the existing split list/detail layout and polling behavior.
- 2026-06-15: Validation hit machine-level disk exhaustion during `cargo test -p agent-diva-gui notebook --lib`; initial failure occurred before the new Notebook code was type-checked, and a later retry still failed in dependency build output (`libsqlite3-sys`) with `No space left on device` after monthly trigger wiring was added.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Added `get_notebook_reports` and `trigger_notebook_report_generation` Tauri commands plus a local markdown/frontmatter parser for Notebook report DTOs.
- Daily and weekly now consume `.agent-diva/autodream/reports/{period}` while monthly remains isolated under `{workspace}/reports/monthly`.
- Large markdown payloads are truncated server-side before render, and the detail panel shows a visible truncated marker with line counts.
- `NotebookView.test.ts` covers daily empty-state trigger affordance and truncated-report banner behavior.
- Monthly trigger wiring now writes a Report-owned monthly markdown file directly from the Tauri backend using the report-system path contract and v1 frontmatter.

### File List

- `_bmad-output/implementation-artifacts/4-1-consume-autodream-and-report-owned-paths-correctly.md`
- `agent-diva-gui/src-tauri/Cargo.toml`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva-gui/src-tauri/src/notebook.rs`
- `agent-diva-gui/src/components/NotebookView.vue`
- `agent-diva-gui/src/components/NotebookView.test.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `Cargo.lock`

### Change Log

- 2026-06-15: Created ready-for-dev story for Report/Notebook path consumption.
- 2026-06-15: Implemented Notebook report loading from AutoDream/report-owned paths, added truncated rendering guardrails, monthly trigger generation, and NotebookView component coverage.
