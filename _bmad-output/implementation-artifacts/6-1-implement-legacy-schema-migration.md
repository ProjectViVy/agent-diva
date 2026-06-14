---
baseline_commit: 8a1114d
---

# Story 6.1: Implement Legacy Schema Migration

Status: review

## Story

As a maintainer,
I want legacy authority material migrated into Laputa safely,
so that the new authority spine can start without losing existing state.

## Acceptance Criteria

1. Given legacy templates or authority files exist, when migration runs, then legacy material is copied into `.laputa/legacy`.
2. Supported material maps into the 14-section Laputa schema.
3. Unsupported or TBD material is stored with explicit TBD status.
4. Migration updates `.laputa/state.json` schema version without deleting legacy sources.
5. Migrated legacy authority files become migration input or backup only, not runtime prompt authority.

## Tasks / Subtasks

- [x] Add a Laputa migration module, likely `agent-diva-laputa/src/migration.rs`, and expose it through `agent-diva-laputa/src/lib.rs`. (AC: 1-4)
- [x] Extend `LaputaPaths` with staging and timestamped legacy backup helpers under `.laputa/staging/` and `.laputa/legacy/{timestamp}/`. (AC: 1, 4)
- [x] Implement legacy source discovery for the current workspace templates and authority files without deleting or rewriting those sources. (AC: 1, 5)
- [x] Map supported legacy material into the existing 14 sections from `agent-diva-core::evolution::LaputaSectionName`. (AC: 2)
- [x] Preserve unsupported or future-owned material as explicit TBD section payloads with metadata describing the original path and reason. (AC: 3)
- [x] Write migration outputs through temp-file plus rename and only swap committed state after all section writes succeed. (AC: 4)
- [x] Add migration tests covering new install, legacy upgrade, unsupported material, simulated mid-migration failure, and idempotent rerun. (AC: 1-5)

## Dev Notes

### Architecture Context

- Laputa is the only durable authority write boundary. Migration is bootstrapping, not a second runtime write path.
- `agent-diva-laputa/src/layout.rs` already owns `.laputa/state.json`, `sections/`, `legacy/`, `migrations/`, `rollback/`, `locks/`, and event paths. Extend this instead of introducing ad hoc paths.
- The schema version currently initializes as `1.0.0` in `agent-diva-laputa/src/layout.rs`; migration must preserve and update that state deliberately.
- The 14-section schema lives in `agent-diva-core/src/evolution/types.rs` as `LaputaSectionName`. Reuse the enum and its `as_str`/routing behavior.

### Current Code State

- Story 1.2 created file-first Laputa storage.
- Story 1.5 added `LaputaService` read/changelog/rollback/event APIs and explicit TBD section reads.
- `LaputaStorage::open` currently creates state when missing. Migration should either be called by `open` through a narrow option or exposed as an explicit service method; avoid silent migration that changes user data before tests prove recovery.

### Implementation Guardrails

- Do not delete legacy sources. Copy to `.laputa/legacy/{timestamp}/` and record source metadata.
- Do not make migrated legacy files part of prompt/runtime context. Epic 5 owns runtime consumption and must read through Laputa APIs.
- Do not add database/vector storage. Laputa remains file-only.
- Do not treat `BOOTSTRAP.md` as persistent authority after initial import.
- Windows-safe paths are required: avoid `:` in timestamp directory names.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-laputa migration`.
- Include a failure-injection test proving staging cleanup or recovery leaves previous state readable.
- Include a test proving unsupported material produces explicit TBD metadata, not dropped content.

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 6.1 requirements.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-501 through FR-506 migration, legacy mapping, bootstrap, and schema version contract.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` — FR-2xx authority contract and direct-write prevention.
- `agent-diva-laputa/src/layout.rs` — current file-first layout and state initialization.
- `agent-diva-core/src/evolution/types.rs` — shared section/type contracts.

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

- 2026-06-14: Started story 6.1 implementation from baseline commit 8a1114d. Current workspace contains unrelated pre-existing dirty changes; scoped work to Laputa migration files and story tracking.
- 2026-06-14: Red-green-refactor cycle used for `agent-diva-laputa/tests/migration.rs`; initial missing API failure confirmed before implementation.
- 2026-06-14: Validated with `cargo test -p agent-diva-laputa migration`, `cargo test -p agent-diva-laputa`, and `cargo clippy -p agent-diva-laputa -- -D warnings`.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Added explicit `LaputaMigration` API that discovers legacy workspace templates, backs them up under `.laputa/legacy/{timestamp}/`, stages writes under `.laputa/staging/`, and updates state only after staged writes succeed.
- Mapped supported legacy material into canonical `LaputaSectionName` targets; `BOOTSTRAP.md` is backed up as one-time initialization input but not persisted as runtime prompt authority.
- Preserved unsupported and TBD material with `status=tbd`, `content_type=tbd`, original path entries, and reason metadata.
- Added migration regression tests for new install, legacy upgrade, unsupported material, failure injection, idempotent rerun, and template discovery.

### File List

- `_bmad-output/implementation-artifacts/6-1-implement-legacy-schema-migration.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-laputa/src/error.rs`
- `agent-diva-laputa/src/layout.rs`
- `agent-diva-laputa/src/lib.rs`
- `agent-diva-laputa/src/migration.rs`
- `agent-diva-laputa/src/proposals.rs`
- `agent-diva-laputa/src/service.rs`
- `agent-diva-laputa/tests/migration.rs`
- `docs/logs/2026-06-laputa-legacy-migration/v0.0.1-legacy-schema-migration/acceptance.md`
- `docs/logs/2026-06-laputa-legacy-migration/v0.0.1-legacy-schema-migration/release.md`
- `docs/logs/2026-06-laputa-legacy-migration/v0.0.1-legacy-schema-migration/summary.md`
- `docs/logs/2026-06-laputa-legacy-migration/v0.0.1-legacy-schema-migration/verification.md`

### Change Log

- 2026-06-14: Implemented legacy schema migration and moved story to review.
