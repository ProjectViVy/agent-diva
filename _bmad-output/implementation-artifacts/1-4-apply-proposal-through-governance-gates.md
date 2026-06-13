# Story 1.4: Apply Proposal Through Governance Gates

Status: ready-for-dev

## Story

As a reviewer,
I want approved proposals to apply only through Laputa,
so that subject, memory, SOP, skill, identity, relationship, commitment, preference, and policy writes are auditable and reversible.

## Acceptance Criteria

1. Given an approved proposal targets a valid Laputa section, when the reviewer applies it, then Laputa validates the section, stages rollback data, writes the new authority state, appends changelog, appends audit, and marks the proposal applied.
2. Any failure during apply rolls back the staged write atomically.
3. Unknown proposal types, unauthorized targets, schema mismatches, and unresolved conflicts return typed errors.

## Tasks / Subtasks

- [ ] Implement `apply_proposal` as the only authority mutation entrypoint in `agent-diva-laputa`. (AC: 1)
- [ ] Validate proposal state, proposal type, target section, and write authority before mutation. (AC: 1, 3)
- [ ] Stage rollback data before changing authority state. (AC: 1, 2)
- [ ] Append changelog and audit records as part of the apply transaction. (AC: 1, 2)
- [ ] Add failure injection tests proving rollback on mid-apply failure. (AC: 2, 3)

## Dev Notes

### Architecture Context

- The required sequence is validate -> stage rollback -> write section -> changelog -> audit -> mark proposal applied.
- Any durable authority write bypassing Laputa is a P0 architecture violation.
- Changelog and audit are not optional logging; they are part of the write contract.

### Required Behavior

- Only approved proposals can be applied.
- `deprecation` proposal type writes the deprecation/changelog flow and must not mutate arbitrary authority content.
- TBD sections must be handled explicitly according to the Laputa PRD instead of returning null or silently skipping.
- On changelog or audit failure after section write, restore previous section content and mark the failure as needs attention where the model supports it.

### Implementation Guardrails

- Do not add GUI approval UX; this story is the backend governance write path.
- Do not implement manager/Tauri routes; Story 1.5 exposes APIs.
- Do not let Report System, AutoDream, SelfImprove, Mentle, or Context Compaction write `.laputa/` directly.
- Keep conflict handling minimal but typed: unresolved conflict must fail without partial authority mutation.

### Project Structure Notes

- Keep apply orchestration in `agent-diva-laputa`, near storage and proposal repository code.
- Use shared `ChangelogRecord`, `AuditEvent`, and `RollbackRequest` types from `agent-diva-core::evolution`.
- Add error variants in the Laputa crate for schema incompatibility, unauthorized target, conflict, rollback failure, lock timeout, and I/O.

### Testing Requirements

- Run at minimum: `cargo test -p agent-diva-laputa apply`.
- Tests must cover:
  - Successful apply creates authority change, changelog, audit, rollback staging, and applied status.
  - Applying a non-approved proposal fails without mutation.
  - Unknown target/type fails without mutation.
  - Injected audit/changelog failure restores previous authority state.
  - Lock acquisition is honored during apply.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.4 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` — direct-write ban and authority spine.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-101 through FR-106 governance write requirements.
- `_bmad-output/implementation-artifacts/1-3-implement-proposal-crud-and-state-transitions.md` — proposal lifecycle prerequisite.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

### Completion Notes List

### File List
