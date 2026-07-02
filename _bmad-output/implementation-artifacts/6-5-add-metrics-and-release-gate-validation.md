---
baseline_commit: 86c00ec
---

# Story 6.5: Add Metrics and Release-Gate Validation

Status: review

## Story

As a maintainer,
I want observability and release gates for EVO-DIVA governance,
so that failures are visible before users rely on durable evolution.

## Acceptance Criteria

1. Given EVO-DIVA flows run, when writes, write errors, rollbacks, AutoDream runs, or governance failures occur, then metrics are emitted for Laputa writes, write errors, rollbacks, AutoDream runs, and failures.
2. Release validation checks zero silent writes, rollback success for eligible records, manual AutoDream run stability, and review surface availability.
3. Mentle regression tests confirm no EVO-DIVA governance role in v1.

## Tasks / Subtasks

- [x] Add a thin metrics surface for Laputa write/apply, write error, rollback, and governance failure counters. (AC: 1)
- [x] Add AutoDream run and failure counters at the service boundary without adding new scheduler ownership. (AC: 1)
- [x] Define a release-gate validation command or documented command set that includes direct-write guard, governance proof loop, rollback eligibility, manual AutoDream stability, and Evolution review surface smoke. (AC: 2)
- [x] Add Mentle regression tests proving existing Mentle behavior remains compatible but does not authorize, write, sync, index, or own EVO-DIVA governance flows. (AC: 3)
- [x] Record release-gate results in `docs/logs/.../verification.md` when the story is implemented. (AC: 2)
- [x] Update any operator-facing docs only if the metrics or release gate become public commands. (AC: 1, 2)

### Review Findings

- [ ] [Review][Patch] Add the Story 6.3 runtime authority-boundary test suite to `epic6-release-gate`; the current gate runs `direct_write_guard` but omits `authority_boundaries`, so it does not actually validate zero direct runtime reads/writes before declaring release readiness. [justfile:102]
- [ ] [Review][Patch] Replace the Mentle release-gate regression with coverage that exercises enabled runtime governance boundaries instead of only `MentleToolRuntimeConfig` filtering. [agent-diva-agent/tests/mentle_governance_boundaries.rs:1]

## Dev Notes

### Architecture Context

- Observability should be thin and local. Do not introduce a metrics backend, database, or remote telemetry dependency.
- Laputa PRD names `laputa_writes_total`, `laputa_write_errors_total`, and `laputa_rollbacks_total`; preserve these names unless implementation finds an existing project metrics naming convention.
- AutoDream PRD success metric requires manual run success >= 95% after stabilization; this story should expose data points, not implement broad auto mode.
- Mentle remains existing tool integration only in v1 and has no governance role.

### Current Code State

- Laputa event emission exists for proposal/changelog/error events.
- AutoDream crate exists with manual run lifecycle in progress from Epic 3.
- Evolution workspace shell exists from Story 2.1; review surface availability can be a compile/smoke check if later UI stories are not complete.

### Implementation Guardrails

- Do not add network telemetry.
- Do not make metrics emission a reason to block authority writes if metrics persistence fails; log and expose diagnostic counters where practical.
- Do not add Mentle dependency to Laputa, AutoDream, Report System, or SelfImprove governance flows.
- Release-gate validation may be staged if GUI smoke is blocked by pre-existing GUI compile issues; record blockers in `TODOLIST.md` if not fixed in-story.

### Testing Requirements

- Minimum targeted validation:
  - `cargo test -p agent-diva-laputa metrics`
  - `cargo test -p agent-diva-autodream`
  - targeted Mentle regression test in `agent-diva-agent`
- Release gate should include Story 6.3 and 6.4 tests once those stories land.
- If GUI smoke is included, follow AGENTS `/validate` rule and record observation points in verification logs.

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 6.5 requirements.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` — NFRs, success metrics, Mentle boundary.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-706 metrics and FR-6xx Mentle boundary.
- `docs/prds/prd-autodream-2026-06-12/prd.md` — AutoDream success metric and manual-first scope.
- `docs/prds/prd-selfinprove-2026-06-12/prd.md` — SelfImprove consumes Laputa metrics, does not own separate metrics.
- `agent-diva-agent/src/mentle_runtime.rs` — Mentle runtime boundary.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- `cargo test -p agent-diva-laputa --test service`
- `cargo test -p agent-diva-autodream --test service`
- `cargo test -p agent-diva-agent --test mentle_governance_boundaries`
- `just epic6-release-gate`

### Completion Notes List

- Added thin in-process metrics counters for Laputa governance writes/errors/rollbacks/failures and AutoDream manual runs/failures.
- Added `just epic6-release-gate` to consolidate Story 6.5 release validation into one documented command path.
- Added Mentle regression coverage proving v1 Mentle runtime selection remains compatible but does not own EVO-DIVA governance write flows.
- Verified GUI review-surface availability with `cargo check -p agent-diva-gui` as part of the release gate.

### File List

- agent-diva-laputa/src/metrics.rs
- agent-diva-laputa/src/lib.rs
- agent-diva-laputa/src/service.rs
- agent-diva-laputa/tests/service.rs
- agent-diva-autodream/src/metrics.rs
- agent-diva-autodream/src/lib.rs
- agent-diva-autodream/src/service.rs
- agent-diva-autodream/tests/service.rs
- agent-diva-agent/tests/mentle_governance_boundaries.rs
- justfile
- docs/logs/2026-06-epic6-release-gate/v0.0.1-metrics-and-release-gate-validation/summary.md
- docs/logs/2026-06-epic6-release-gate/v0.0.1-metrics-and-release-gate-validation/verification.md
- docs/logs/2026-06-epic6-release-gate/v0.0.1-metrics-and-release-gate-validation/release.md
- docs/logs/2026-06-epic6-release-gate/v0.0.1-metrics-and-release-gate-validation/acceptance.md
