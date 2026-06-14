---
baseline_commit: 1c02af7f
---

# Story 2.4: Build Runs, Audit, and Policy Views

Status: ready-for-dev

## Story

As a user,
I want to see reflection runs, audit history, and governance policy,
so that I can understand what the system proposed and what was applied.

## Acceptance Criteria

1. Given AutoDream run records and Laputa audit events exist, when I open Runs or Audit, then Runs shows trigger, start/end time, duration, status, inputs, outputs, errors, and proposal count.
2. Audit shows timestamp, actor, source proposal, target, change type, summary, and rollback availability.
3. Policy shows configuration only, not a review queue.
4. Policy includes the exact copy: `Durable personality, memory, SOP, skill, and policy changes require review before they are applied.`

## Tasks / Subtasks

- [ ] Add Runs, Audit, and Policy tab content under `EvolutionView.vue`. (AC: 1-4)
- [ ] Implement Audit from existing Laputa changelog/event APIs first; do not wait for AutoDream runtime. (AC: 2)
- [ ] Implement rollback availability display using `ChangelogRecord.reverted`, `stale`, and action eligibility. (AC: 2)
- [ ] Implement Runs view with current available data and clear empty/TBD state until Epic 3 adds AutoDream run APIs. (AC: 1)
- [ ] Add `AutoDreamRunRecord` frontend DTO matching `agent-diva-core/src/evolution/types.rs`. (AC: 1)
- [ ] Implement Policy as configuration/status summary only and include the required safety copy exactly. (AC: 3, 4)
- [ ] Link Policy to existing Settings Self Evolution panel where detailed edits belong. (AC: 3)
- [ ] Add tests/smoke for Audit rendering, rollback availability state, Policy copy exactness, and Runs empty/error states. (AC: 1-4)

## Dev Notes

### Previous Story Intelligence

- Stories 2.1-2.3 should have created the Evolution shell, proposal inbox, detail panel, and governance actions.
- Reuse the same tab system. Do not create a second top-level page or settings-only view.

### Data Sources

- Audit v1 source:
  - `laputa_list_changelog(page, page_size)`
  - `laputa_get_changelog(id)`
  - `laputa_poll_events("changelog", since)`
- Rollback action source:
  - `laputa_rollback_changelog(id, payload)`
- Runs source at story creation time:
  - `AutoDreamRunRecord` exists in `agent-diva-core`, but `agent-diva-autodream` does not yet exist in this workspace.
  - Show an honest empty/TBD state and design the DTO boundary so Epic 3 can plug in run APIs without rewriting the view.
- Policy source:
  - existing `SelfEvolutionSettings.vue` invokes `get_self_evolution_config` / `save_self_evolution_config`.

### Audit Rendering Requirements

- Required fields:
  - timestamp: `created_at`
  - actor: `applied_by` or related audit event actor if available
  - source proposal: `proposal_id`
  - target: `target_section`
  - change type: `action`
  - summary: derived from diff or action/target
  - rollback availability: true only when action is rollback-eligible, not already reverted, and not stale
- Rollback is a new audited change, not a delete action.
- If rollback data is unavailable, explain why.

### Runs Rendering Requirements

- Show columns/cards for trigger, start/end time, duration, status, inputs, outputs, errors, and proposal count.
- Until backend run APIs exist, support:
  - empty state: "No AutoDream runs are available yet";
  - unavailable state: backend not implemented;
  - future DTO mapping via `AutoDreamRunRecord`.
- Do not fake run records.

### Policy Requirements

- Policy is not a review queue.
- Must include exact English copy:
  `Durable personality, memory, SOP, skill, and policy changes require review before they are applied.`
- Remove or clearly disable any auto-merge durable-change affordance if exposed from current settings. Existing `auto_merge_confidence` must not appear as an enabled v1 auto-merge control.

### Testing Requirements

- Audit list renders changelog entries with rollback availability.
- Rollback-unavailable cases show reason.
- Policy tab includes the exact required copy.
- Runs tab handles backend unavailable without spinner-only UI.
- GUI validation plus Rust validation if Tauri/backend commands are changed.

### References

- `agent-diva-core/src/evolution/types.rs` - `AutoDreamRunRecord`, `ChangelogRecord`.
- `agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue` - existing policy settings surface.
- `agent-diva-gui/src/components/settings/MemoryChangelog.vue` - existing changelog pattern.
- `agent-diva-laputa/src/service.rs` - changelog and rollback rules.
- `_bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/EXPERIENCE.md` - learning rhythm, audit, rollback, policy flows.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

- 2026-06-14: Story context prepared from Epic 2, Evolution experience flows, existing SelfEvolutionSettings, Laputa changelog/rollback APIs, and current absence of AutoDream runtime crate.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.

### File List

- `_bmad-output/implementation-artifacts/2-4-build-runs-audit-and-policy-views.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-14: Created ready-for-dev story for Runs, Audit, and Policy views.
