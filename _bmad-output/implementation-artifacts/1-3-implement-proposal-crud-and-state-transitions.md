# Story 1.3: Implement Proposal CRUD and State Transitions

Status: ready-for-dev

## Story

As a user,
I want proposals to be created, listed, updated, and reviewed consistently,
so that every durable evolution change has an explicit lifecycle.

## Acceptance Criteria

1. Given a valid proposal create request with evidence refs, when Laputa persists the proposal, then it appears in list responses with status, type, target, source, risk, timestamps, and evidence count.
2. Valid transitions follow the architecture state machine: pending_review, approved, rejected, edited, applied, reverted, superseded, needs_attention, and run_failed.
3. Invalid state transitions return typed errors without mutating persisted proposal state.

## Tasks / Subtasks

- [ ] Add proposal repository APIs to `agent-diva-laputa`. (AC: 1)
- [ ] Persist proposals using the file-first storage from Story 1.2. (AC: 1)
- [ ] Implement list/filter primitives needed by later SelfImprove and manager routes. (AC: 1)
- [ ] Implement transition validation for all v1 states. (AC: 2, 3)
- [ ] Add tests for create, read, list, update/edit, valid transitions, and invalid transition non-mutation. (AC: 1, 2, 3)

## Dev Notes

### Architecture Context

- `EvolutionProposal` is the common envelope from Story 1.1.
- Proposal persistence must not apply authority changes. Applying approved proposals is Story 1.4.
- Proposal list data must be sufficient for the future Evolution Inbox without requiring UI code in this story.

### Required Behavior

- Creating a proposal starts in `pending_review` unless the caller explicitly creates a diagnostic state such as `run_failed`; reject unsafe shortcuts.
- Editing a proposal must update `updated_at` and preserve original evidence refs unless the caller provides a valid replacement.
- Listing must expose enough summary data for status, proposal type, target section, source/run id, risk, timestamps, and evidence count.
- Invalid transitions must leave the stored proposal byte-for-byte or semantically unchanged.

### Implementation Guardrails

- Do not implement changelog/audit authority mutation here.
- Do not expose HTTP/Tauri surfaces here; keep this as Rust API + storage behavior.
- Do not silently delete terminal proposals.
- Keep IDs stable and generated deterministically enough for tests when a test injects an id.

### Project Structure Notes

- Place proposal logic in a focused module such as `agent-diva-laputa/src/proposals.rs` or a small `proposals/` module.
- Put state transition logic near the domain or repository boundary, not duplicated in callers.
- Reuse `agent_diva_core::evolution` types.

### Testing Requirements

- Run at minimum: `cargo test -p agent-diva-laputa proposals`.
- Tests must cover:
  - Create and fetch by id.
  - List summaries with evidence count.
  - Approved/rejected/edited transitions from pending review.
  - Invalid terminal-state mutation attempts.
  - Unknown proposal type or target error paths from Story 1.1 routing.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.3 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` — authority spine and producer/consumer table.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — proposal lifecycle, proposal events, and write boundary.
- `_bmad-output/implementation-artifacts/1-1-define-governance-domain-types.md` — required shared types.
- `_bmad-output/implementation-artifacts/1-2-create-laputa-file-first-authority-storage.md` — required storage substrate.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

### Completion Notes List

### File List
