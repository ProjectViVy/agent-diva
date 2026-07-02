---
baseline_commit: 91f07911fb031d119efbbcb5d87b8fb78c176fdd
---

# Story 1.4: Apply Proposal Through Governance Gates

Status: done

## Story

As a reviewer,
I want approved proposals to apply only through Laputa,
so that subject, memory, SOP, skill, identity, relationship, commitment, preference, and policy writes are auditable and reversible.

## Acceptance Criteria

1. Given an approved proposal targets a valid Laputa section, when the reviewer applies it, then Laputa validates the section, stages rollback data, writes the new authority state, appends changelog, appends audit, and marks the proposal applied.
2. Any failure during apply rolls back the staged write atomically.
3. Unknown proposal types, unauthorized targets, schema mismatches, and unresolved conflicts return typed errors.

## Tasks / Subtasks

- [x] Implement `apply_proposal` as the only authority mutation entrypoint in `agent-diva-laputa`. (AC: 1)
- [x] Validate proposal state, proposal type, target section, and write authority before mutation. (AC: 1, 3)
- [x] Stage rollback data before changing authority state. (AC: 1, 2)
- [x] Append changelog and audit records as part of the apply transaction. (AC: 1, 2)
- [x] Add failure injection tests proving rollback on mid-apply failure. (AC: 2, 3)

### Review Findings

- [x] [Review][Patch] Apply uses an `apply` lock while proposal edit/transition uses a separate `proposals` lock, allowing concurrent state mutation during apply [`agent-diva-laputa/src/proposals.rs:186`]
- [x] [Review][Patch] Apply failure compensation restores the section but leaves rollback requests and already-written changelog records visible, violating the atomic rollback boundary [`agent-diva-laputa/src/proposals.rs:231`]
- [x] [Review][Patch] TBD proposal targets such as `journal_note` are forced through JSON parsing even though the PRD requires TBD sections to accept raw bytes [`agent-diva-laputa/src/proposals.rs:480`]

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

GPT-5 Codex

### Debug Log References

- 2026-06-14: Loaded bmad-dev-story configuration, project context, sprint status, Story 1.4, Story 1.3 implementation context, and Laputa storage/proposal modules.
- 2026-06-14: Added red-phase apply tests for successful apply, non-approved rejection, unauthorized target/schema mismatch, unresolved conflicts, deprecation flow, rollback on injected mid-apply failure, and lock timeout.
- 2026-06-14: Implemented `ProposalRepository::apply_proposal` and `apply_proposal_with_options` under the Laputa apply lock, with validation, rollback staging, section write, changelog, audit, proposal applied state, and needs-attention rollback behavior on mid-apply failure.
- 2026-06-14: Validation passed: `cargo fmt -p agent-diva-laputa -- --check`, `cargo test -p agent-diva-laputa`, `cargo check -p agent-diva-laputa`, `cargo clippy -p agent-diva-laputa -- -D warnings`.
- 2026-06-14: Workspace validation attempted. `just fmt-check`, `just check`, and `just test` remain blocked by pre-existing unrelated workspace issues already tracked in `TODOLIST.md`.

### Completion Notes List

- Added the backend-only Laputa apply entrypoint that enforces approved-state mutation and keeps authority writes behind the file-first Laputa repository.
- Added apply-time contract validation for proposal type routing, explicit writable targets, JSON schema compatibility, and unresolved conflict markers.
- Staged rollback requests before authority mutation, wrote authority section JSON, appended changelog and audit records, and marked proposals applied only after the full apply sequence completes.
- Implemented deprecation apply as changelog/audit flow without mutating arbitrary authority section content.
- Added deterministic failure injection for mid-apply rollback tests and ensured failed section writes restore prior content and mark the proposal `needs_attention`.

### File List

- `agent-diva-laputa/src/error.rs`
- `agent-diva-laputa/src/lib.rs`
- `agent-diva-laputa/src/proposals.rs`
- `agent-diva-laputa/tests/apply.rs`
- `_bmad-output/implementation-artifacts/1-4-apply-proposal-through-governance-gates.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-laputa-apply/v0.0.1-proposal-apply-governance-gates/summary.md`
- `docs/logs/2026-06-laputa-apply/v0.0.1-proposal-apply-governance-gates/verification.md`
- `docs/logs/2026-06-laputa-apply/v0.0.1-proposal-apply-governance-gates/release.md`
- `docs/logs/2026-06-laputa-apply/v0.0.1-proposal-apply-governance-gates/acceptance.md`

### Change Log

- 2026-06-14: Implemented Story 1.4 proposal apply governance gates and moved story to review.
