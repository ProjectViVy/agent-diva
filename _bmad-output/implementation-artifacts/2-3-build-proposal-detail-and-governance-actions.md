---
baseline_commit: 1c02af7f
---

# Story 2.3: Build Proposal Detail and Governance Actions

Status: in-progress

## Story

As a reviewer,
I want to inspect proposal details and act on them safely,
so that durable authority changes remain user-controlled.

## Acceptance Criteria

1. Given a proposal is selected, when the detail pane opens, then it shows title, status, risk, source, target, timestamps, summary, proposed change, diff, evidence, affected modules, and safety checks.
2. Approve and Apply, Approve Only, Edit Proposal, Reject, Defer, and Rollback actions call the Laputa commands.
3. Destructive actions require confirmation naming the target.
4. Evidence opens in a drawer or inline preview with missing evidence explicitly marked.
5. High-risk proposals open with evidence visible and cannot be approved when required evidence is missing.

## Tasks / Subtasks

- [x] Add `ProposalDetail` and `GovernanceActionBar` components under `agent-diva-gui/src/components/evolution/`. (AC: 1, 2)
- [x] Render all required proposal fields from the selected `EvolutionProposal`, including proposed patch and evidence refs. (AC: 1)
- [x] Add diff/current-proposed presentation. If current section data is needed, read it through `laputa_get_section`. (AC: 1)
- [x] Implement evidence drawer or inline preview with explicit missing/unavailable evidence state. (AC: 4, 5)
- [x] Implement Approve and Apply, Approve Only, Edit Proposal, Reject, Defer, and Rollback action flows through typed Tauri wrappers. (AC: 2)
- [x] Add confirmation dialogs for Reject, Rollback, and any action that applies or reverts durable authority; confirmation copy must name the target section. (AC: 3)
- [x] Block high-risk approval when required evidence is missing. (AC: 5)
- [x] Preserve failed apply proposals and render recoverable error details. (AC: 2, 5)
- [x] Add focused tests/smoke for action enablement, missing evidence blocking, and confirmation flows. (AC: 2-5)

## Dev Notes

### Previous Story Intelligence

- Story 2.2 should provide list selection, filters, batch action affordances, and keyboard shortcuts.
- Reuse selected proposal state from Inbox. Detail should not refetch the whole list unless refresh is requested.
- If Story 2.2 added a transition wrapper, use it for Approve Only, Reject, and any legal state transition.

### Required Backend/Tauri Commands

- Existing Tauri commands:
  - `laputa_get_proposal`
  - `laputa_apply_proposal`
  - `laputa_get_section`
  - `laputa_get_changelog`
  - `laputa_rollback_changelog`
- Existing manager route not yet wrapped in Tauri at story creation time:
  - `POST /api/laputa/proposals/:id/transition`
  - `PUT /api/laputa/proposals/:id`
- If edit/transition Tauri commands are still missing, add them in this story and register them in `agent-diva-gui/src-tauri/src/lib.rs`.

### Action Semantics

- Approve Only: transition selected proposal to `approved`.
- Approve and Apply: transition to `approved` if needed, then call `laputa_apply_proposal`.
- Edit Proposal: call Laputa edit API/Tauri command. Do not rewrite proposal JSON files directly.
- Reject: transition to `rejected`.
- Defer: UI-local or backend-supported only if a durable state exists by implementation time. Current `ProposalState` has no `deferred` variant.
- Rollback: call changelog rollback. If the selected proposal has no rollback-eligible changelog record, disable action and explain why.

### Detail Field Guidance

- Title: derive from proposal id/type/target unless a future field exists.
- Summary: derive from proposed patch metadata if structured, otherwise show a concise patch preview.
- Proposed change: show `proposed_patch` with syntax-safe formatting.
- Diff: prefer `ChangelogRecord.diff` for applied proposals; for pending proposals, compare current `laputa_get_section(target_section)` result with `proposed_patch`.
- Affected modules: derive from `target_section` and proposal type.
- Safety checks: evidence present, target authorized, risk reviewed, backend state legal, rollback availability where applicable.

### UX / Safety Requirements

- High-risk proposals open with evidence visible by default.
- Missing evidence must be explicit; do not hide it behind a generic error.
- Destructive confirmations must name the target, e.g. `memory_md`, `sop`, or the localized section label.
- Failed apply must preserve the proposal, show a recoverable error, and offer retry/open audit where applicable.
- Do not expose physical authority file paths by default; use section labels unless user opens advanced details.

### Testing Requirements

- Low-risk proposal can approve/apply through mocked Tauri commands.
- High-risk proposal with missing evidence disables approval.
- Reject and rollback confirmations include the target section.
- Apply failure leaves the proposal visible and does not clear detail state.
- Minimum validation:
  - GUI type/build path.
  - `just fmt-check` and targeted Rust check if Tauri command files are changed.

### References

- `_bmad-output/implementation-artifacts/2-2-build-proposal-inbox-list-and-filters.md` - selected proposal and list behavior.
- `agent-diva-gui/src/components/DecisionCard.vue` - existing decision/action card pattern.
- `agent-diva-gui/src/utils/appDialog.ts` - confirmation dialog utility.
- `agent-diva-gui/src/utils/appToast.ts` - recoverable error feedback.
- `agent-diva-laputa/src/service.rs` - apply, changelog, rollback behavior.
- `_bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/DESIGN.md` - Proposal Detail Panel and Governance Action Bar.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Story context prepared from Epic 2, Proposal Detail UX, core proposal/changelog DTOs, Tauri Laputa commands, and Laputa apply/rollback service behavior.
- 2026-06-14: Added Tauri wrappers for proposal edit/transition plus changelog proposal filtering; wired Evolution detail pane to section/changelog reads and governance actions.
- 2026-06-14: Added focused Vitest coverage for high-risk missing-evidence blocking, target-naming confirmations, and apply-failure detail retention.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Detail pane now shows proposal metadata, summary, evidence, current/proposed content, diff fallback, safety checks, and governance actions.
- High-risk and critical proposals default evidence open and block approval when evidence is missing.
- `Defer` remains UI-explained only because Laputa still has no durable deferred state; the button surfaces the limitation instead of mutating authority state.
- Story 2.2 dependencies remain incomplete in the current tree, so this story lands on top of a simpler inbox shell rather than the full filtered/keyboard-driven inbox described upstream.
- Workspace-wide `vue-tsc` and `cargo check -p agent-diva-gui` are currently blocked by pre-existing unrelated errors outside this story's changed files.

### File List

- `_bmad-output/implementation-artifacts/2-3-build-proposal-detail-and-governance-actions.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-gui/src/api/desktop.ts`
- `agent-diva-gui/src/components/EvolutionView.vue`
- `agent-diva-gui/src/components/EvolutionView.test.ts`
- `agent-diva-gui/src/components/evolution/GovernanceActionBar.vue`
- `agent-diva-gui/src/components/evolution/ProposalDetail.vue`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`

### Change Log

- 2026-06-14: Created ready-for-dev story for proposal detail and governance actions.
- 2026-06-14: Implemented proposal detail pane, governance actions, Tauri wrappers, and focused GUI tests for story 2.3.
