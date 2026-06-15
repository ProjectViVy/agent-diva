---
baseline_commit: 8a1114d
---

# Story 5.2: Enforce Mentle Governance Exclusion

Status: in-progress

## Story

As a maintainer,
I want EVO-DIVA flows to avoid new Mentle dependencies,
so that v1 governance has one authority owner.

## Acceptance Criteria

1. Given existing feature-gated Mentle tool integration is enabled, when EVO-DIVA proposal, AutoDream, Report, Notebook, Audit, or rollback flows run, then they do not write to Mentle.
2. EVO-DIVA flows do not sync reports to Mentle.
3. EVO-DIVA flows do not index AutoDream outputs into Mentle.
4. EVO-DIVA flows do not inject Mentle recall by default into governance context.

## Tasks / Subtasks

- [ ] Add regression tests or compile-time guardrails showing AutoDream, Laputa, Report/Notebook proposal creation, audit, rollback, and SelfImprove governance paths do not call Mentle write/index APIs. (AC: 1-4)
- [ ] Preserve existing feature-gated Mentle tool runtime behavior outside EVO-DIVA governance. (AC: 1)
- [ ] Ensure proposal creation, apply, rollback, report solidification, and AutoDream output paths have no new dependency on `mentle_active()` or Mentle runtime state. (AC: 1-4)
- [ ] Ensure governance prompt/context assembly does not inject Mentle recall by default. (AC: 4)
- [ ] If a future on-demand recall adapter is touched, keep it read-only and explicitly user-triggered. (AC: 4)
- [ ] Document any remaining allowed Mentle compatibility behavior in completion notes so reviewers can distinguish compatibility from governance. (AC: 1-4)

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` section 9 says Mentle is explicitly excluded from v1 evolution governance.
- Allowed: existing feature-gated Mentle tool settings, existing tool runtime behavior, existing compatibility tests.
- Forbidden: per-session evolution writes, report sync, AutoDream indexing, proposal evidence pipeline, apply/audit/rollback/report-solidification participation, default context injection, and new EVO-DIVA dependencies on `mentle_active()`.

### PRD Context

- Governance PRD FR-601 through FR-603 preserve existing Mentle integration but forbid expanding it into the autonomous evolution loop.
- Laputa PRD FR-601 through FR-603 says Mentle must not call authority writes, must not directly read `.laputa/state.json`, and must not inject recall by default.

### Current Code State

- `agent-diva-agent/src/agent_loop.rs` owns feature-gated Mentle runtime construction and `mentle_active()`.
- `agent-diva-agent/src/context.rs` currently gates Mentle prompt routing with `mentle_enabled` and tool names.
- Existing Mentle tests live behind `#[cfg(feature = "mentle")]` in `agent-diva-agent/src/agent_loop.rs`.
- `agent-diva-manager/src/state.rs` exposes Mentle tool listing/config behavior that should remain compatibility-only.

### Implementation Guardrails

- Do not remove existing Mentle feature-gated tools unless a test proves they are broken and a separate story owns the fix.
- Do not add Mentle dependencies to `agent-diva-laputa` or `agent-diva-autodream`.
- Do not sync AutoDream run records, reports, proposals, changelog, audit, rollback, or report solidification results into `memory/palace.db`.
- Do not treat Mentle recall as authority. Any future recall must be on-demand evidence only.

### Testing Requirements

- Add negative tests that fail if governance flows require Mentle runtime state.
- Add grep-style or unit guard coverage for no writes/indexing into Mentle from EVO-DIVA crates and handlers.
- Preserve existing Mentle compatibility tests where the feature is available.
- Run targeted validation:
  - `cargo test -p agent-diva-agent mentle`
  - `cargo test -p agent-diva-autodream`
  - `cargo test -p agent-diva-laputa`
  - `cargo check -p agent-diva-manager`

### References

- `_bmad-output/planning-artifacts/epics.md` - Story 5.2 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - Mentle boundary.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` - FR-601 through FR-603.
- `docs/prds/prd-laputa-2026-06-12/prd.md` - Laputa/Mentle read/write boundary.
- `agent-diva-agent/src/agent_loop.rs` - `mentle_active()` and feature-gated Mentle runtime.
- `agent-diva-agent/src/context.rs` - Mentle prompt routing behavior.

## Previous Story Intelligence

- Story 5.1 should establish the Laputa-backed `MemoryProvider` prompt boundary. This story must keep Mentle outside that boundary by default.
- Story 3.x AutoDream work must remain proposal/report producing only and must not index outputs into Mentle.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

- 2026-06-14: Story context prepared from Epic 5, EVO-DIVA architecture section 9, Governance PRD FR-6xx, Laputa PRD FR-6xx, and existing Mentle runtime/prompt wiring.
- 2026-06-15: Added Mentle exclusion guardrails for AutoDream output/report paths, Laputa governance service paths, and default agent context assembly. Full story validation is blocked by unrelated current AutoDream/Laputa test failures recorded in `TODOLIST.md`.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Added negative guardrails that assert AutoDream output emission and rhythm report writing do not create `memory/palace.db` or `.mentle` state.
- Added dependency guardrails asserting `agent-diva-autodream` and `agent-diva-laputa` manifests do not introduce Mentle/Memtle dependencies.
- Added default context guardrail asserting governance prompt assembly does not expose Mentle recall/routing unless Mentle prompt state is explicitly enabled.
- Existing feature-gated Mentle runtime behavior remains allowed outside EVO-DIVA governance; no Mentle tool runtime code was removed or disabled.
- Story remains in progress because required full validation is blocked by unrelated current failures: `cargo test -p agent-diva-autodream` fails in a compaction capsule evidence assertion, and `cargo test -p agent-diva-laputa` fails to compile unrelated migration/apply recovery tests.

### File List

- `_bmad-output/implementation-artifacts/5-2-enforce-mentle-governance-exclusion.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `TODOLIST.md`
- `agent-diva-agent/src/context.rs`
- `agent-diva-autodream/tests/mentle_governance.rs`
- `agent-diva-laputa/tests/mentle_governance.rs`
- `docs/logs/2026-06-mentle-governance-exclusion/v0.0.1-mentle-governance-exclusion/acceptance.md`
- `docs/logs/2026-06-mentle-governance-exclusion/v0.0.1-mentle-governance-exclusion/release.md`
- `docs/logs/2026-06-mentle-governance-exclusion/v0.0.1-mentle-governance-exclusion/summary.md`
- `docs/logs/2026-06-mentle-governance-exclusion/v0.0.1-mentle-governance-exclusion/verification.md`

### Change Log

- 2026-06-14: Created ready-for-dev story for Mentle governance exclusion.
- 2026-06-15: Added Mentle governance exclusion guardrails; story remains in progress pending unrelated validation blocker cleanup.
