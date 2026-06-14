---
baseline_commit: 5d072f6
---

# Story 3.4: Emit Proposal and Run Output Artifacts

Status: ready-for-dev

## Story

As a reviewer,
I want AutoDream outputs to become review-required proposals,
so that reflection can suggest changes without applying them.

## Acceptance Criteria

1. Given reflection completes successfully, when output artifacts are written, then `.agent-diva/autodream/runs/{run_id}/autodream_run.json` contains schema version, evidence refs, confidence, output summaries, proposal candidates, and `review_required: true`.
2. Emitted proposals use the shared `EvolutionProposal` contract.
3. Events are appended to `.agent-diva/autodream/events.jsonl`.
4. The run record links to proposal IDs.

## Tasks / Subtasks

- [ ] Add `agent-diva-autodream/src/outputs.rs` for run artifact and event writing. (AC: 1, 3)
- [ ] Define a versioned `autodream_run.json` artifact schema with bounded evidence refs, output summary, confidence, proposal candidates, and `review_required: true`. (AC: 1)
- [ ] Convert proposal candidates into `agent_diva_core::evolution::EvolutionProposal` values with `state = PendingReview` and `source_run_id = run_id`. (AC: 2)
- [ ] Persist proposals through the Laputa proposal API/service, never by writing `.laputa/proposals` directly. (AC: 2)
- [ ] Append structured AutoDream events to `.agent-diva/autodream/events.jsonl` with atomic or locked append behavior. (AC: 3)
- [ ] Update the run record with created proposal IDs and output summary. (AC: 4)
- [ ] Add tests for artifact schema, proposal conversion/routing, event append, run-proposal linking, and no direct authority writes. (AC: 1-4)

## Dev Notes

### Architecture Context

- The governance spine requires `EvidenceRef -> EvolutionProposal -> user review -> Laputa apply`. AutoDream stops at proposal creation and report output.
- `EvolutionProposal` minimum fields are already implemented in `agent-diva-core/src/evolution/types.rs`.
- Valid proposal types are `MemoryPatch`, `JournalNote`, `LearningNote`, `IdentityPatch`, `RelationshipUpdate`, `CommitmentSet`, `SopCreate`, and `Deprecation`; unknown kinds must fail before persistence.
- Every durable authority change remains pending review. AutoDream must set `review_required: true` and must not apply proposals.

### Current Code State

- Laputa proposal create/list/update/apply exists from Epic 1.
- `AutoDreamRunRecord` has `proposal_ids`, `summary`, and `error` fields that can link outputs to review surfaces.
- No AutoDream artifact writer exists until Epic 3 implementation starts.

### Implementation Guardrails

- Do not write `.laputa/` files directly. Use the Laputa Rust API or manager boundary selected by Story 3.1/3.3.
- Do not create proposal IDs that collide with Laputa ID rules.
- Events must be append-only and recoverable. Avoid unlocked read-modify-write cycles for `events.jsonl`.
- Artifact content may include summaries and bounded excerpts, not full raw session dumps.
- A failed proposal persistence should mark the run `Failed` or `NeedsAttention`-equivalent diagnostics; it must not pretend the run fully succeeded.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-autodream outputs`.
- Add integration-style tests with a temp Laputa store or fake proposal sink.
- Validate JSON artifact shape with serde round-trip tests.

## Previous Story Intelligence

- Story 3.3 should produce structured stage outputs. This story owns durable serialization and proposal emission.
- Story 3.2's evidence refs must remain bounded and stable; do not expand them into unbounded content in artifacts.

## Completion Note

Ultimate context engine analysis completed - comprehensive developer guide created.
