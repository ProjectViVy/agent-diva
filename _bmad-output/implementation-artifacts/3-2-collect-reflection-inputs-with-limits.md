---
baseline_commit: 5d072f6
---

# Story 3.2: Collect Reflection Inputs with Limits

Status: ready-for-dev

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

- [ ] Add `agent-diva-autodream/src/inputs.rs` with a bounded input collection service. (AC: 1-4)
- [ ] Read recent sessions through existing session storage APIs or stable session files; do not parse partially written temp files. (AC: 1, 2)
- [ ] Read Laputa snapshot/sections through the Laputa service/API boundary, not by reading `.laputa/` files directly. (AC: 1, 4)
- [ ] Add optional source capsule discovery under the existing compact/capsule locations when present, with omission records when absent. (AC: 1, 2)
- [ ] Enforce configurable per-source count/size limits and total budget truncation in priority order. (AC: 1)
- [ ] Persist an input summary into the current run record or run output metadata. (AC: 3)
- [ ] Add tests for priority order, truncation, missing-source omissions, corrupt session omission, and direct-write/Mentle exclusion. (AC: 1-4)

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
