---
baseline_commit: 5d072f6
---

# Story 3.2: Collect Reflection Inputs with Limits

Status: review

## Story

As AutoDream,
I want to gather bounded evidence from sessions and project memory sources,
so that reflection has context without over-reading or mutating authority.

## Acceptance Criteria

1. Given a run starts, when input collection executes, then it reads recent sessions, Laputa snapshots/sections through the read API, and source capsules in configured priority order.
2. Missing inputs are recorded as non-fatal omissions.
3. Collected inputs are summarized in the run record.
4. Input collection never writes to Mentle and never directly writes Laputa authority files.

## Tasks / Subtasks

- [x] Add `agent-diva-autodream/src/inputs.rs` with a bounded input collection service. (AC: 1-4)
- [x] Read recent sessions through existing session storage APIs or stable session files; do not parse partially written temp files. (AC: 1, 2)
- [x] Read Laputa snapshot/sections through the Laputa service/API boundary, not by reading `.laputa/` files directly. (AC: 1, 4)
- [x] Add optional source capsule discovery under the existing compact/capsule locations when present, with omission records when absent. (AC: 1, 2)
- [x] Enforce configurable per-source count/size limits and total budget truncation in priority order. (AC: 1)
- [x] Persist an input summary into the current run record or run output metadata. (AC: 3)
- [x] Add tests for priority order, truncation, missing-source omissions, corrupt session omission, and direct-write/Mentle exclusion. (AC: 1-4)

## Dev Notes

### Architecture Context

- AutoDream input priority comes from `docs/prds/prd-autodream-2026-06-12/prd.md`: recent sessions first, then history/memory-compatible sources, then source capsules when available.
- EVO-DIVA architecture updates that older PRD boundary: authority state must be read through Laputa snapshot/section APIs, not legacy `MEMORY.md` authority reads.
- Context Compaction output can be secondary evidence only. A future proposal must not rely solely on compaction evidence.
- Mentle remains compatibility-only for EVO-DIVA v1. No input collector should write to Mentle or require Mentle availability.

### Current Code State

- `agent-diva-core/src/session/manager.rs` owns session history behavior.
- `agent-diva-laputa` exposes service APIs for snapshot and section reads from Epic 1.
- There is no `agent-diva-autodream` crate until Story 3.1 is implemented.

### Implementation Guardrails

- Input collection is read-only except writing AutoDream run metadata under `.agent-diva/autodream/runs/{run_id}/`.
- Missing sessions, missing Laputa sections, absent capsules, and parse failures should be captured as structured omissions, not hard failures unless all mandatory inputs fail.
- Do not create a second authority parser for `.laputa/state.json`; use the public Laputa boundary.
- Keep excerpts bounded. `EvidenceRef` should point to evidence and include optional short excerpts, not unbounded content.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-autodream inputs`.
- Include temp-dir tests for each input source and truncation rule.
- Include a negative test or grep-style guard proving the collector does not write `.laputa`, `MEMORY.md`, or Mentle paths.

## Previous Story Intelligence

- Story 3.1 should establish run IDs, storage layout, lifecycle state, and cancellation. This story should plug collection into an existing run rather than reimplementing lifecycle.
- If Story 3.1 records cancellation state, input collection must check cancellation before and after expensive reads.

## Completion Note

Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Loaded `bmad-dev-story` workflow, sprint status, Story 3.2 requirements, AutoDream PRD notes, and existing Story 3.1 lifecycle implementation.
- 2026-06-14: Added bounded AutoDream input collection across recent sessions, Laputa read APIs, and optional compact capsules with omission tracking and priority-budget truncation.
- 2026-06-14: Persisted input summaries back into AutoDream run records and added crate tests for omissions, truncation, read-only authority safety, and service integration.

### Completion Notes List

- Added `agent-diva-autodream/src/inputs.rs` with a read-only bounded collector that reads recent stable session files, Laputa sections through `LaputaService`, and optional compact capsules.
- Enforced per-source limits plus a total byte budget in priority order and recorded non-fatal omissions for missing/corrupt sessions, absent Laputa sections, and missing capsules.
- Extended `agent_diva_core::evolution::AutoDreamRunRecord` with typed `input_summary` metadata and wrote collection summaries back into the run record through `AutoDreamService::collect_inputs`.
- Added focused collector tests and a service integration test proving run record persistence, while preserving the no-direct-write boundary for `.laputa`, `MEMORY.md`, and Mentle paths.
- Restored the pre-existing `outputs` module implementation to keep `agent-diva-autodream` crate tests green while limiting Story 3.2 scope to input collection and run metadata.

### File List

- `agent-diva-autodream/Cargo.toml`
- `agent-diva-autodream/src/error.rs`
- `agent-diva-autodream/src/inputs.rs`
- `agent-diva-autodream/src/layout.rs`
- `agent-diva-autodream/src/lib.rs`
- `agent-diva-autodream/src/outputs.rs`
- `agent-diva-autodream/src/service.rs`
- `agent-diva-autodream/tests/outputs.rs`
- `agent-diva-autodream/tests/service.rs`
- `agent-diva-core/src/evolution/types.rs`
- `_bmad-output/implementation-artifacts/3-2-collect-reflection-inputs-with-limits.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-autodream-input-collection/v0.0.1-reflection-input-collection/summary.md`
- `docs/logs/2026-06-autodream-input-collection/v0.0.1-reflection-input-collection/verification.md`
- `docs/logs/2026-06-autodream-input-collection/v0.0.1-reflection-input-collection/release.md`
- `docs/logs/2026-06-autodream-input-collection/v0.0.1-reflection-input-collection/acceptance.md`

### Change Log

- 2026-06-14: Implemented bounded AutoDream reflection input collection and moved story to review.
