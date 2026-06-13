---
baseline_commit: 49d85e4636db2d7f5704b9b3c67f9be7b5d2e654
---

# Story 1.1: Define Governance Domain Types

Status: review

## Story

As a developer,
I want shared governance domain types in `agent-diva-core`,
so that Laputa, AutoDream, Report System, and Evolution UI use one contract.

## Acceptance Criteria

1. Given the workspace builds `agent-diva-core`, when the governance module is added, then it exposes `EvidenceRef`, `EvidenceSource`, `EvolutionProposal`, `ProposalType`, `ProposalState`, `LaputaSectionName`, `ChangelogRecord`, `AuditEvent`, `RollbackRequest`, and `AutoDreamRunRecord`.
2. Each type supports serialization, deserialization, clone/debug behavior, and stable schema-friendly field names.
3. Proposal types route to valid Laputa section names or return a typed unknown-type error.

## Tasks / Subtasks

- [x] Add `agent-diva-core/src/evolution/` and export it from `agent-diva-core/src/lib.rs`. (AC: 1)
- [x] Define the shared governance structs/enums with `serde`, `Clone`, `Debug`, and equality traits where useful. (AC: 1, 2)
- [x] Implement proposal-to-section routing for all v1 proposal types. (AC: 3)
- [x] Add focused unit tests for serde round trips, route mapping, and unknown-type errors. (AC: 2, 3)
- [x] Keep the module independent from manager, GUI, provider, and Laputa crate implementation details. (AC: 1)

## Dev Notes

### Architecture Context

- The governing authority spine is `EvidenceRef -> EvolutionProposal -> user review -> Laputa apply -> changelog / audit -> rollback -> prompt / report consumption`.
- Shared model location is specified by the EVO-DIVA architecture as `agent-diva-core/src/evolution/mod.rs` and `agent-diva-core/src/evolution/types.rs`.
- `agent-diva-core` already owns stable cross-crate contracts such as `agent-diva-core/src/memory/provider.rs`; follow that style: plain domain types, no transport shapes, no manager/Tauri dependencies.

### Required Type Shape

- `EvidenceRef` must include evidence id, source, uri, optional excerpt, optional hash, and created timestamp.
- `EvolutionProposal` must include id, timestamps, creator, proposal type, target section, evidence refs, proposed patch, risk level, state, and optional source run id.
- `ProposalType` must cover: memory patch, journal note, learning note, identity patch, relationship update, commitment set, SOP create, and deprecation.
- `ProposalState` must cover: pending review, approved, rejected, edited, applied, reverted, superseded, needs attention, and run failed.
- `LaputaSectionName` must cover the 14-section contract from the Laputa PRD, including explicit TBD sections.

### Implementation Guardrails

- Do not create `agent-diva-laputa` in this story; that is Story 1.2.
- Do not add HTTP routes, Tauri commands, GUI DTOs, or storage files in this story.
- Use `chrono::DateTime<chrono::Utc>` and existing workspace `serde`/`thiserror` conventions.
- Prefer snake_case serde field names and stable string variants suitable for JSON persisted data.
- Avoid `unwrap`/`expect` outside tests.

### Project Structure Notes

- Add the new module under `agent-diva-core/src/evolution/`.
- Re-export only the domain types that downstream crates need from `agent-diva-core/src/evolution/mod.rs`; keep helper functions private unless needed.
- `agent-diva-core/src/lib.rs` should expose `pub mod evolution;`.

### Testing Requirements

- Run at minimum: `cargo test -p agent-diva-core evolution`.
- Add tests for:
  - JSON round trip of a representative `EvolutionProposal`.
  - Every `ProposalType` routing to the expected `LaputaSectionName`.
  - Unknown proposal type parsing or conversion returning a typed error.

### References

- `_bmad-output/planning-artifacts/epics.md` — Story 1.1 and Epic 1 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` — Shared domain model and authority spine.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — 14-section schema, proposal routing, and state semantics.
- `docs/project-context.md` — Rust workspace conventions and error handling patterns.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

- 2026-06-14: Loaded bmad config, project context, sprint status, story 1.1, EVO-DIVA architecture, Laputa PRD routing/section references, and core crate patterns.
- 2026-06-14: Validation passed: `cargo test -p agent-diva-core evolution`, `cargo clippy -p agent-diva-core -- -D warnings`, `cargo test -p agent-diva-core`, `rustfmt --edition 2021 --check agent-diva-core/src/evolution/mod.rs agent-diva-core/src/evolution/types.rs`.
- 2026-06-14: Workspace-wide `cargo fmt --all -- --check` still fails on pre-existing unrelated formatting diffs; captured in `TODOLIST.md`.

### Completion Notes List

- Added the `agent_diva_core::evolution` module with shared governance domain contracts for evidence refs, proposals, Laputa sections, changelog records, audit events, rollback requests, and AutoDream run records.
- Implemented snake_case serde variants and stable field names for persistence-friendly JSON contracts.
- Added v1 proposal routing from all eight `ProposalType` variants to `LaputaSectionName`, plus typed `EvolutionError::UnknownProposalType` handling for raw proposal type strings.
- Kept implementation domain-only in `agent-diva-core`; no manager, GUI, provider, storage, or Laputa crate dependencies were added.
- Added focused unit tests for proposal JSON round trip, all v1 route mappings, and unknown proposal type errors.

### File List

- `agent-diva-core/src/lib.rs`
- `agent-diva-core/src/evolution/mod.rs`
- `agent-diva-core/src/evolution/types.rs`
- `_bmad-output/implementation-artifacts/1-1-define-governance-domain-types.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/summary.md`
- `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/verification.md`
- `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/release.md`
- `docs/logs/2026-06-governance-domain-types/v0.0.1-governance-domain-types/acceptance.md`
- `TODOLIST.md`

### Change Log

- 2026-06-14: Implemented Story 1.1 governance domain types and moved story to review.
