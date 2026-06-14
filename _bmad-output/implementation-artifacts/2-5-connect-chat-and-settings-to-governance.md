---
baseline_commit: 1c02af7f
---

# Story 2.5: Connect Chat and Settings to Governance

Status: review

## Story

As a user,
I want Chat and Settings to initiate governance flows without hiding review,
so that manual triggers and policy edits stay understandable.

## Acceptance Criteria

1. Given I trigger AutoDream from Chat, when the run starts, then Chat shows an AutoDream run card with status and a link to Evolution Runs or Inbox.
2. Generated proposals render as compact Evolution proposal cards.
3. Settings exposes policy options but no auto-merge affordance.
4. Apply failures preserve the proposal and show a recoverable error.

## Tasks / Subtasks

- [x] Add Chat-side card models/components for AutoDream run status and compact Evolution proposal cards. (AC: 1, 2)
- [x] Wire Chat card links into the Evolution workspace tabs and filters from Stories 2.1-2.4. (AC: 1, 2)
- [x] Connect proposal cards to existing proposal detail/governance action flow instead of duplicating review UI inside Chat. (AC: 2)
- [x] Update `SelfEvolutionSettings.vue` so policy options are clear and no enabled auto-merge durable-change affordance is exposed. (AC: 3)
- [x] Ensure apply failures from proposal cards preserve proposal state, show recoverable error, and offer open-in-Evolution retry/review. (AC: 4)
- [x] If AutoDream backend commands are not available yet, implement Chat trigger UI as disabled/unavailable with clear status, or behind the backend capability check. Do not fake a successful run. (AC: 1)
- [x] Add i18n strings for Chat run/proposal cards and Settings governance copy. (AC: 1-4)
- [x] Add smoke coverage for Chat card rendering, Evolution deep links, Settings no-auto-merge state, and apply failure preservation. (AC: 1-4)

## Dev Notes

### Previous Story Intelligence

- Story 2.1 created the Evolution workspace and tab routing.
- Story 2.2 created Inbox filters and selected proposal behavior.
- Story 2.3 created detail/governance actions and apply failure behavior.
- Story 2.4 created Runs, Audit, and Policy views.
- Reuse those behaviors. Chat must initiate and observe, not become the main review workspace.

### Existing Chat / Settings Entry Points

- `agent-diva-gui/src/App.vue` owns chat message state, stream events, and session loading.
- `agent-diva-gui/src/components/NormalMode.vue` passes chat props/events into `ChatView`.
- `agent-diva-gui/src/components/ChatView.vue` renders chat messages.
- `agent-diva-gui/src/components/DecisionCard.vue` is an existing approval-oriented card pattern that can guide compact proposal cards.
- `agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue` currently exposes `auto_merge_confidence`; v1 governance must remove, hide, or clearly disable it as a durable-change control.

### Governance Rules

- Chat supports:
  - manual AutoDream trigger;
  - inline run result card;
  - inline proposal summary card;
  - deep link to Evolution Inbox or Runs.
- Chat does not support:
  - complex diff review;
  - batch triage;
  - rollback flows;
  - direct durable writes.
- Settings owns policy configuration only. It does not own daily proposal review.
- Any durable write remains: proposal -> user review -> Laputa apply.

### AutoDream Dependency

- At story creation time, `agent-diva-autodream` is not present in the workspace. Epic 3 owns real manual run lifecycle.
- If this story is implemented before Epic 3, the Chat trigger must show backend-unavailable/disabled state and deep link to Policy, not fake a run.
- If Epic 3 has landed by implementation time, use its Tauri/manager commands and run DTOs.

### Proposal Card Requirements

- Compact card fields:
  - proposal type;
  - status;
  - risk;
  - target;
  - short summary;
  - evidence count;
  - open in Evolution action.
- Generated proposals should link to Inbox filtered by `source_run_id` when available.
- Apply failures must preserve the proposal and render a recoverable error. Do not remove the card or mark it successful if backend apply failed.

### Settings Requirements

- Include the v1 safety statement where policy is configured:
  `Durable personality, memory, SOP, skill, and policy changes require review before they are applied.`
- Hide or disable `auto_merge_confidence` for durable authority changes.
- Keep enablement, frequency, thresholds, required confirmations, badge/notification options if supported.

### Testing Requirements

- Chat proposal card opens the matching Evolution detail.
- Chat run card opens Runs or Inbox with the correct filter when source/run id exists.
- Missing AutoDream backend renders unavailable state, not success.
- Settings does not expose enabled auto-merge for durable changes.
- Apply failure card shows recoverable error and preserves proposal.
- GUI validation plus Rust validation if Tauri/backend commands are changed.

### References

- `agent-diva-gui/src/App.vue` - chat state and events.
- `agent-diva-gui/src/components/ChatView.vue` - chat rendering.
- `agent-diva-gui/src/components/DecisionCard.vue` - compact decision card pattern.
- `agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue` - policy settings.
- `_bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/EXPERIENCE.md` - Chat manual trigger, report to proposal, policy configuration.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - direct-write ban and supervised autonomy boundary.

## Dev Agent Record

### Agent Model Used

Codex GPT-5

### Debug Log References

- 2026-06-14: Story context prepared from Epic 2, prior 2.x story dependencies, Chat/Settings entry points, and supervised autonomy UX rules.
- 2026-06-15: Implemented Chat governance cards, AutoDream trigger unavailable/success card handling, Evolution deep links, Settings no-auto-merge copy, and focused GUI smoke coverage.
- 2026-06-15: Validation passed for focused GUI smoke tests. Full GUI/Rust validation is blocked by pre-existing unrelated failures recorded in iteration verification.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Added compact AutoDream run and Evolution proposal card rendering in Chat.
- Chat manual AutoDream trigger now renders a run card and shows backend-unavailable state on command failure without faking success.
- Chat governance cards route to Evolution Runs or Inbox with proposal/source-run context; Chat does not duplicate review/governance actions.
- Self Evolution settings now show the review-before-apply safety boundary and do not expose an enabled durable-change auto-merge control.
- Added focused smoke coverage for governance card links, Evolution deep-link filtering, Settings no-auto-merge behavior, and retained apply-failure preservation coverage.

### File List

- `_bmad-output/implementation-artifacts/2-5-connect-chat-and-settings-to-governance.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-gui/src/components/chat/governanceCards.ts`
- `agent-diva-gui/src/components/chat/ChatGovernanceCard.vue`
- `agent-diva-gui/src/components/chat/ChatGovernanceCard.test.ts`
- `agent-diva-gui/src/components/ChatView.vue`
- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/components/EvolutionView.vue`
- `agent-diva-gui/src/components/EvolutionView.test.ts`
- `agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue`
- `agent-diva-gui/src/components/settings/SelfEvolutionSettings.test.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `docs/logs/2026-06-chat-settings-governance/v0.0.1-chat-settings-governance/summary.md`
- `docs/logs/2026-06-chat-settings-governance/v0.0.1-chat-settings-governance/verification.md`
- `docs/logs/2026-06-chat-settings-governance/v0.0.1-chat-settings-governance/acceptance.md`
- `docs/logs/2026-06-chat-settings-governance/v0.0.1-chat-settings-governance/release.md`

### Change Log

- 2026-06-14: Created ready-for-dev story for Chat and Settings governance connections.
- 2026-06-15: Implemented Chat and Settings governance connections; story marked ready for review.
