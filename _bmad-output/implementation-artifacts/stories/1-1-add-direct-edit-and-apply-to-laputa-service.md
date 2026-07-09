---
baseline_commit: 04c3db0412adb676d59273b8c7427c0e06d8ce48
---

# Story 1.1: Add direct-edit-and-apply method to LaputaService

Status: review

## Story

As a developer,
I want a high-level `create_and_apply_direct_edit` method in `LaputaService`,
So that direct user edits from the GUI can reuse the existing proposal governance flow without leaking raw file writes outside the `agent-diva-laputa` crate.

## Acceptance Criteria

1. **Given** a valid `LaputaSectionName` and patch content,
   **When** `create_and_apply_direct_edit(section, patch, actor, now)` is called,
   **Then** it creates an `EvolutionProposal`, transitions it to `Approved`, applies it, and returns the resulting `ApplyOutcome`.

2. **Given** the new method is called,
   **When** the apply succeeds,
   **Then** the target section file is updated, a `ChangelogRecord` with `action == Apply` is written, an `AuditEvent` is emitted, and a `RollbackRequest` is staged.

3. **Given** the new method is called on a non-JournalReflective section with non-JSON patch content,
   **When** apply validation runs,
   **Then** it returns `LaputaError::SchemaIncompatible` and leaves the section unchanged.

4. **Given** the new method is called on a non-writable or mismatched section,
   **When** apply validation runs,
   **Then** it returns `LaputaError::UnauthorizedTarget` and leaves the section unchanged.

5. **Given** the new method is implemented,
   **When** `cargo test -p agent-diva-laputa --test authority_boundaries` and `cargo test -p agent-diva-laputa --test direct_write_guard` are run,
   **Then** both tests still pass, confirming no forbidden direct filesystem writes were introduced outside `agent-diva-laputa/src`.

## Tasks / Subtasks

- [x] **Add `create_and_apply_direct_edit` to `LaputaService`** (AC: #1, #2)
  - [x] Open `agent-diva-laputa/src/service.rs`
  - [x] Add a new public method on `LaputaService` with signature:
        ```rust
        pub fn create_and_apply_direct_edit(
            &self,
            section: LaputaSectionName,
            patch: impl Into<String>,
            actor: impl Into<String>,
            now: DateTime<Utc>,
        ) -> Result<crate::ApplyOutcome>
        ```
  - [x] Internally derive `ProposalType` from `section`:
        - `MemoryMd` → `MemoryPatch`
        - `JournalReflective` → `JournalNote`
        - `Preferences` → `LearningNote`
        - `Identity` → `IdentityPatch`
        - `Relationship` → `RelationshipUpdate`
        - `Commitment` → `CommitmentSet`
        - `Changelog` → `Deprecation`
        - `HistoryMd` → `HistoryPatch`
        - `Daily` → `DailyPatch`
        - `Weekly` → `WeeklyPatch`
        - `Monthly` → `MonthlyPatch`
        - For any other section, return `LaputaError::UnauthorizedTarget`
  - [x] Build an `EvolutionProposal` with:
        - a unique `id`
        - `created_at` / `updated_at` = `now`
        - `created_by` = `actor`
        - `proposal_type` and `target_section` matching the routing above
        - at least one `EvidenceRef` with `source = EvidenceSource::UserInput`
        - `state = ProposalState::PendingReview`
        - `risk_level = RiskLevel::Low`
        - `source_run_id = None`
  - [x] Call `self.create_proposal(proposal)?`, then `self.transition_proposal(&id, ProposalState::Approved, now)?`, then `self.apply_proposal(&id, actor, now)` and return its `ApplyOutcome`.

- [x] **Add integration tests** (AC: #1, #2, #3, #4)
  - [x] Open `agent-diva-laputa/tests/service.rs` (or create `tests/create_and_apply_direct_edit.rs`)
  - [x] Add a test that:
    - creates a temp workspace with `tempfile::tempdir()`
    - opens `LaputaService` with `LaputaService::open(temp.path()).unwrap()`
    - writes initial content to `MemoryMd`
    - calls `create_and_apply_direct_edit(MemoryMd, new_json, "user", Utc::now())`
    - asserts the returned `ApplyOutcome.changelog.action == Apply`
    - asserts `read_section(MemoryMd).content` equals the new JSON
    - asserts changelog/audit/rollback artifacts exist
  - [x] Add a test for `SchemaIncompatible` when patch is not valid JSON for a JSON section.
  - [x] Add a test for `UnauthorizedTarget` when the section is not writable.

- [x] **Run validation gates** (AC: #5)
  - [x] `cargo test -p agent-diva-laputa`
  - [x] `cargo test -p agent-diva-laputa --test authority_boundaries`
  - [x] `cargo test -p agent-diva-laputa --test direct_write_guard`
  - [x] `just fmt-check`
  - [x] `just check`

## Dev Notes

### Relevant architecture patterns and constraints

- **All file writes must stay inside `agent-diva-laputa/src`**. The `authority_boundary_guard.rs` helper scans `agent-diva-agent/src`, `agent-diva-autodream/src`, `agent-diva-manager/src`, `agent-diva-cli/src`, and `agent-diva-gui/src-tauri/src` for any direct `fs::write`, `File::create`, `OpenOptions::new()`, `atomic_write`, or `atomic_write_json` touching forbidden paths such as `.laputa/sections`, `.laputa/changelog`, `MEMORY.md`, `IDENTITY.md`, or `SOUL.md`. This method must therefore live in `agent-diva-laputa/src/service.rs` and perform writes only through existing internal helpers.
- **Proposal-driven governance is mandatory**. Do not add a method that writes the section file without creating a proposal, changelog, and audit event. Reuse `create_proposal` → `transition_proposal` → `apply_proposal`.
- **ProposalType routing must match `LaputaSectionName`**. The existing `ProposalType::target_section()` method in `agent-diva-core/src/evolution/types.rs` is the single source of truth.
- **Patch format depends on section type**. `JournalReflective` is a "raw apply target" and accepts arbitrary strings; all other writable sections require valid JSON.

### Source tree components to touch

- `agent-diva-laputa/src/service.rs` — add the new public method
- `agent-diva-laputa/src/proposals.rs` — no changes expected unless helper extraction is needed
- `agent-diva-laputa/tests/service.rs` or new `agent-diva-laputa/tests/create_and_apply_direct_edit.rs` — tests
- No changes to manager, Tauri, or GUI in this story

### Testing standards summary

- Use `tempfile::tempdir()` for isolated workspaces.
- Use `LaputaService::open(path)` to initialize `.laputa/` layout.
- Use `LaputaService::reset_metrics_for_test()` before assertions if checking metrics.
- Reuse existing helper patterns from `tests/service.rs` for timestamp and evidence construction.
- Run targeted authority-boundary tests after implementation.

### Project Structure Notes

- This story only touches `agent-diva-laputa`. Later stories will consume the new method from `agent-diva-manager` and `agent-diva-gui/src-tauri/src`.
- Keep the method public so `LaputaService` callers in higher crates can use it without direct filesystem access.

### References

- `LaputaService` signatures: [Source: agent-diva-laputa/src/service.rs]
- `EvolutionProposal` / `ProposalType` / `LaputaSectionName` types and routing: [Source: agent-diva-core/src/evolution/types.rs]
- Proposal validation and `ApplyOutcome`: [Source: agent-diva-laputa/src/proposals.rs]
- Authority boundary guard helper: [Source: agent-diva-laputa/tests/authority_boundary_guard.rs]
- Service-level integration test patterns: [Source: agent-diva-laputa/tests/service.rs]
- End-to-end governance test pattern: [Source: agent-diva-laputa/tests/governance_proof_loop.rs]
- Architecture spine: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md]

## Dev Agent Record

### Agent Model Used

Claude Code (kimi-for-coding)

### Debug Log References

- Compile fix: clone `section` before moving into `EvolutionProposal` (`service.rs`).
- Formatting applied with `just fmt`.

### Completion Notes List

- [x] Method added to `agent-diva-laputa/src/service.rs`
- [x] `ProposalType` extended in `agent-diva-core/src/evolution/types.rs` with `HistoryPatch`, `DailyPatch`, `WeeklyPatch`, `MonthlyPatch`; routing and serialization updated.
- [x] Integration tests added and passing in `agent-diva-laputa/tests/create_and_apply_direct_edit.rs`
- [x] `authority_boundaries` and `direct_write_guard` tests still pass
- [x] `just fmt-check && just check` clean

### File List

- `agent-diva-core/src/evolution/types.rs`
- `agent-diva-laputa/src/service.rs`
- `agent-diva-laputa/tests/create_and_apply_direct_edit.rs`
