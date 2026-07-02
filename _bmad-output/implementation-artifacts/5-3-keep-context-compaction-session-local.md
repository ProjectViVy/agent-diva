---
baseline_commit: 8a1114d
---

# Story 5.3: Keep Context Compaction Session-Local

Status: review

## Story

As Diva runtime,
I want Context Compaction to summarize only the active session,
so that compaction does not become hidden durable memory.

## Acceptance Criteria

1. Given Context Compaction creates a session summary, when EVO-DIVA evidence is collected, then compaction output can be referenced only as secondary evidence.
2. Compaction output is not written to Laputa, Mentle, MEMORY, SOP, skill, identity, relationship, commitment, preference, or policy stores.
3. Durable changes based on compaction must still create proposals.

## Tasks / Subtasks

- [x] Audit current compaction storage and prompt injection paths to confirm summaries remain in session-local data only. (AC: 1, 2)
- [x] Add guardrails so AutoDream input collection and proposal generation mark compaction summaries as secondary evidence, never sole authority evidence. (AC: 1, 3)
- [x] Ensure no compaction code writes `.laputa`, Mentle storage, `MEMORY.md`, SOP, skill, identity, relationship, commitment, preference, or policy paths. (AC: 2)
- [x] Add proposal validation behavior that blocks or marks `needs_attention` when evidence contains only compaction summaries. (AC: 1, 3)
- [x] Preserve existing `ContextBuilder::build_messages` compaction boundary behavior for live prompt survival. (AC: 1)
- [x] Add tests for session-local serialization, prompt injection boundaries, no durable writes, and compaction-only evidence rejection. (AC: 1-3)

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` section 10 defines Context Compaction as session-local prompt survival.
- It may summarize current-session context under context pressure and may be cited as secondary evidence.
- It must not write Laputa, Mentle, MEMORY, identity, relationship, preferences, commitments, reports, SOPs, or skills.
- It must not be the sole evidence for a durable proposal.

### PRD Context

- Governance PRD FR-701 requires compaction to preserve only the current session under context pressure.
- Governance PRD FR-702 allows compaction outputs only as secondary evidence and requires compaction-only proposals to be blocked or marked `needs_attention`.

### Current Code State

- `agent-diva-core/src/session/store.rs` defines `CompactSummary` and `Session.compaction_history`.
- `agent-diva-core/src/session/manager.rs` loads and saves compaction history inside session data.
- `agent-diva-agent/src/context.rs` injects compaction summaries into prompt messages with boundary markers.
- `agent-diva-agent/tests/compaction_integration.rs` already covers multi-compaction chains, backward compatibility, and message injection.

### Implementation Guardrails

- Do not move compaction summaries into Laputa authority state.
- Do not convert compaction summaries directly into MEMORY/SOP/skill/identity edits.
- Do not introduce Mentle indexing or report sync for compaction outputs.
- Keep compaction prompt text clearly bounded so downstream logic can distinguish it from durable authority.
- Proposal creation from compaction-derived insights must require non-compaction evidence or a `needs_attention` state.

### Testing Requirements

- Extend compaction integration tests or add focused governance tests for:
  - compaction summaries staying inside session records;
  - prompt messages containing boundary markers;
  - no durable authority writes from compaction code;
  - compaction-only evidence rejected or marked `needs_attention`;
  - compaction plus primary evidence allowed as secondary support.
- Run targeted validation:
  - `cargo test -p agent-diva-agent compaction`
  - `cargo test -p agent-diva-core session`
  - `cargo test -p agent-diva-autodream inputs` if proposal/evidence collection is touched.

### References

- `_bmad-output/planning-artifacts/epics.md` - Story 5.3 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - Context Compaction boundary and test plan.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` - FR-701 and FR-702.
- `agent-diva-core/src/session/store.rs` - `CompactSummary` and session-local compaction history.
- `agent-diva-core/src/session/manager.rs` - session persistence.
- `agent-diva-agent/src/context.rs` - compaction prompt injection.
- `agent-diva-agent/tests/compaction_integration.rs` - existing compaction regression coverage.

## Previous Story Intelligence

- Story 5.1 changes authority prompt consumption. Preserve compaction as a separate session-local prompt survival mechanism, not a replacement authority source.
- Story 5.2 keeps Mentle out of governance. Compaction must not become a backdoor Mentle indexing feed.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Story context prepared from Epic 5, EVO-DIVA architecture section 10, Governance PRD FR-7xx, and existing session/compaction implementation.
- 2026-06-15: Audited session compaction storage (`Session.compaction_history`), prompt injection (`ContextBuilder::build_messages`), AutoDream input capsules, and proposal output emission.
- 2026-06-15: Added core governance evidence validation so `ContextCompaction` evidence is secondary only and cannot be the sole evidence for durable proposals.
- 2026-06-15: Validation notes: `just fmt-check`, `just check`, and `just test` remain blocked by unrelated existing workspace issues recorded in `TODOLIST.md`; targeted 5.3 checks passed.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Added `validate_governance_evidence` and `is_primary_governance_evidence` helpers in the shared evolution domain contract.
- Marked AutoDream compaction capsule excerpts with a secondary-evidence-only boundary before they enter proposal evidence.
- Blocked AutoDream output emission before persistence when artifact or proposal candidate evidence is compaction-only.
- Preserved existing session-local compaction serialization and prompt boundary behavior; no compaction path was moved into durable authority stores.

### File List

- `_bmad-output/implementation-artifacts/5-3-keep-context-compaction-session-local.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-core/src/evolution/types.rs`
- `agent-diva-autodream/src/inputs.rs`
- `agent-diva-autodream/src/outputs.rs`
- `agent-diva-autodream/tests/outputs.rs`

### Change Log

- 2026-06-14: Created ready-for-dev story for session-local Context Compaction boundaries.
- 2026-06-15: Implemented session-local compaction evidence guardrails and moved story to review.
