---
baseline_commit: f5bbcf6
---

# Story 6.4: Add End-to-End Governance Proof Loop

Status: review

## Story

As a maintainer,
I want a named E2E test for the governance loop,
so that the core authority spine is proven before broader runtime consumption.

## Acceptance Criteria

1. Given a valid proposal can be created, when the E2E proof loop runs, then it creates a proposal, approves and applies it, reads changelog, reads snapshot, rolls back changelog, and verifies audit contains both apply and rollback events.
2. The test confirms no direct writes occurred outside Laputa.
3. The test is documented as the release gate for prompt/report consumption.

## Tasks / Subtasks

- [x] Add a named E2E test, for example `governance_proof_loop`, under the crate or integration test target that can exercise Laputa and manager boundaries. (AC: 1)
- [x] Create a valid proposal through the public service/API boundary; do not seed internal files except workspace fixtures. (AC: 1)
- [x] Approve and apply the proposal, then verify section content, changelog detail, audit event, and proposal state. (AC: 1)
- [x] Roll back the changelog, then verify section content returns to prior state, original changelog is marked reverted, and rollback audit/changelog records exist. (AC: 1)
- [x] Invoke or embed the direct-write guard from Story 6.3 so the proof loop fails if authority files were written outside Laputa. (AC: 2)
- [x] Document this test in iteration verification and the release-gate story as the minimum gate before Epic 5 prompt/report consumption is considered safe. (AC: 3)

## Dev Notes

### Architecture Context

- The governance spine is: `EvidenceRef -> EvolutionProposal -> user review -> Laputa apply -> changelog/audit -> rollback -> prompt/report consumption`.
- This story proves the spine, not GUI review ergonomics.
- Story 1.5 already exposed service, HTTP, and Tauri boundaries. Use the highest practical boundary that is stable in tests; if manager HTTP is too heavy, use `LaputaService` and add a separate route smoke.

### Current Code State

- `agent-diva-laputa/tests/apply.rs`, `proposals.rs`, `service.rs`, and `storage.rs` already cover slices of behavior.
- `agent-diva-manager/src/handlers/laputa.rs` exposes HTTP handlers for Laputa operations.
- Existing review fixes hardened rollback, event replay, and typed errors. Do not duplicate those unit tests; compose them into the E2E path.

### Implementation Guardrails

- The test must be deterministic and not depend on external network, real user config, or real `.laputa` data.
- Use `tempfile` workspace roots and Windows-safe relative paths.
- Do not treat direct file seeding as proof of the public workflow. Fixture setup may create an empty workspace, but proposal/apply/rollback must use public APIs.
- If a route-level test is added, assert status codes and typed error payloads where relevant.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-laputa governance_proof_loop`.
- If manager route is included: `cargo test -p agent-diva-manager laputa`.
- The test must verify audit contains both apply and rollback events, not only changelog records.

## References

- `_bmad-output/planning-artifacts/epics.md` — Story 6.4 requirements.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` — governing authority spine and FR-202 rollback proof.
- `docs/prds/prd-laputa-2026-06-12/prd.md` — FR-101 through FR-403 write/changelog/rollback contract.
- `agent-diva-laputa/src/service.rs` — current service API.
- `agent-diva-laputa/tests/service.rs` — existing integration-test style.
- `agent-diva-manager/src/handlers/laputa.rs` — route-level boundary if used.

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

- `CARGO_TARGET_DIR=/Users/mastwet/Desktop/morediva/agent-diva-pro/target CARGO_BUILD_JOBS=1 cargo test -p agent-diva-laputa governance_proof_loop`
- `CARGO_TARGET_DIR=/Users/mastwet/Desktop/morediva/agent-diva-pro/target CARGO_BUILD_JOBS=1 cargo test -p agent-diva-laputa governance_direct_write_guard_only_allows_laputa_owned_authority_paths`

### Completion Notes List

- Added `agent-diva-laputa/tests/governance_proof_loop.rs` to prove create -> approve -> apply -> snapshot/changelog -> rollback -> audit event flow end to end through `LaputaService`.
- Added `agent-diva-laputa/tests/direct_write_guard.rs` to keep EVO-DIVA authority-path writes constrained to Laputa-owned boundaries while avoiding false positives from read-only references and test fixtures.
- Validation initially hit `No space left on device` in the isolated worktree target directory; reran targeted tests with shared `CARGO_TARGET_DIR` and single-job builds to complete verification without changing the implementation scope.

### File List

- `agent-diva-laputa/tests/governance_proof_loop.rs`
- `agent-diva-laputa/tests/direct_write_guard.rs`
- `_bmad-output/implementation-artifacts/6-4-add-end-to-end-governance-proof-loop.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `docs/logs/2026-06-governance-proof-loop/v0.0.1-governance-proof-loop/summary.md`
- `docs/logs/2026-06-governance-proof-loop/v0.0.1-governance-proof-loop/verification.md`
- `docs/logs/2026-06-governance-proof-loop/v0.0.1-governance-proof-loop/release.md`
- `docs/logs/2026-06-governance-proof-loop/v0.0.1-governance-proof-loop/acceptance.md`

### Change Log

- 2026-06-15: Added the named governance proof loop integration test, added the direct-write guard test, and documented the proof loop as the minimum release gate before prompt/report consumption.
