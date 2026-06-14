---
baseline_commit: 1c02af7f
---

# Story 2.2: Build Proposal Inbox List and Filters

Status: review

## Story

As a reviewer,
I want to scan, filter, search, and batch proposals,
so that review work stays fast even when AutoDream emits multiple candidates.

## Acceptance Criteria

1. Given proposals exist in pending or needs-attention states, when I open Evolution Inbox, then each row shows type, status, risk, target, source, evidence count, age, and blocked reason when present.
2. Filters support status, type, risk, source, target, unread, and search text.
3. Batch actions support approve, reject, read/unread, and defer where the proposal state allows it.
4. Keyboard shortcuts `/`, `J`, `K`, `Enter`, `A`, `E`, `R`, `D`, and `Esc` work outside text inputs.
5. Empty, loading, and error states are visible without spinner-only panels.

## Tasks / Subtasks

- [x] Add an Inbox panel under `EvolutionView.vue`, preferably split into `agent-diva-gui/src/components/evolution/ProposalInbox.vue` and smaller row/filter components. (AC: 1, 2, 5)
- [x] Load proposals through the typed `desktop.ts` Laputa wrapper created in Story 2.1. (AC: 1, 5)
- [x] Implement row rendering for proposal type, state, risk, target section, source, evidence count, age, and blocked reason. (AC: 1)
- [x] Implement client-side filters for status, type, risk, source, target, unread, and search text. (AC: 2)
- [x] Implement selection state and keyboard navigation across the filtered list. (AC: 4)
- [x] Implement batch approve/reject/read/unread/defer behavior only for legal proposal states. (AC: 3)
- [x] Persist unread/read state locally if backend does not yet expose read markers; keep it separate from proposal authority state. (AC: 2, 3)
- [x] Add loading, empty, and recoverable error views with retry. (AC: 5)
- [x] Add focused tests or smoke coverage for filtering, keyboard behavior, and disabled illegal batch actions. (AC: 2-5)

## Dev Notes

### Previous Story Intelligence

- Story 2.1 should have added the Evolution shell, navigation, tabs, badge count, and typed Laputa wrappers.
- Reuse the shell's proposal loader or state management. Do not create a second competing proposal fetch loop.
- Preserve the Inbox as the default tab.

### Data Contract

- `EvolutionProposal` is defined in `agent-diva-core/src/evolution/types.rs`.
- Required UI fields map as follows:
  - type: `proposal_type`
  - status: `state`
  - risk: `risk_level`
  - target: `target_section`
  - source: prefer `source_run_id`; fallback to `created_by`
  - evidence count: `evidence_refs.length`
  - age: `created_at`
  - blocked reason: no dedicated field exists yet; derive from `state === "needs_attention"` and any structured error/event metadata if available, otherwise show a concise generic needs-attention reason.
- Do not mutate `.laputa/` directly. Proposal state changes must go through Tauri/manager/Laputa APIs.

### Batch Action Requirements

- Approve: transition proposal to `approved` where backend transition command is available.
- Reject: transition proposal to `rejected`.
- Defer: architecture lists `defer` as a UI action, but `ProposalState` does not currently include `deferred`. Until the backend adds a durable state, implement defer as UI-local snooze/read marker or route to `needs_attention` only if product explicitly accepts that behavior. Record this limitation in completion notes.
- Read/unread: UI-local unless a backend field is added intentionally.
- If a Tauri wrapper for `laputa_transition_proposal` is missing, add it to `agent-diva-gui/src-tauri/src/commands.rs`, register it in `agent-diva-gui/src-tauri/src/lib.rs`, and wrap it in `desktop.ts`.

### Keyboard Requirements

- `/`: focus search input.
- `J` / `K`: move selection down/up.
- `Enter`: open selected proposal detail.
- `A`: approve selected proposal where legal.
- `E`: edit selected proposal or open detail edit affordance.
- `R`: reject selected proposal where legal.
- `D`: defer selected proposal.
- `Esc`: close search/detail/selection mode without losing unsaved edits.
- Shortcuts must not fire while focus is inside input, textarea, select, or contenteditable elements.

### UX Requirements

- High-risk rows pin a visible marker to the left edge.
- Blocked rows explain the blocker in one line.
- Rows must not resize when status text changes.
- Empty/loading/error states must include useful text and actions; do not show a spinner-only panel.
- Use dense operational styling consistent with current settings/notebook components.

### Testing Requirements

- Filter combinations: status + risk + search.
- Keyboard navigation skips no rows and does not trigger inside inputs.
- Illegal batch actions are disabled and do not call backend.
- Empty and backend error states render recoverable UI.
- Minimum GUI validation plus `just fmt-check` if Rust/Tauri command files are touched.

### References

- `_bmad-output/implementation-artifacts/2-1-add-evolution-workspace-shell.md` - shell and API wrapper setup.
- `agent-diva-core/src/evolution/types.rs` - proposal states and DTO shape.
- `agent-diva-manager/src/server.rs` - `/api/laputa/proposals/:id/transition`.
- `agent-diva-gui/src-tauri/src/commands.rs` - existing Laputa Tauri commands; may need transition wrapper.
- `_bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/DESIGN.md` - Proposal Row and Inbox layout.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Story context prepared from Epic 2, Story 2.1 dependency, core evolution DTOs, manager Laputa transition route, and Evolution UX keyboard requirements.
- 2026-06-15: Implemented `ProposalInbox.vue` as the dense Evolution inbox panel and wired it into the existing `EvolutionView.vue` proposal loader/detail flow.
- 2026-06-15: Added local read/unread and deferred marker persistence via GUI localStorage keys; proposal authority state remains owned by Laputa transition APIs.
- 2026-06-15: Added focused EvolutionView Vitest coverage for status+risk+search filtering, keyboard navigation/input isolation, and disabled illegal batch approval.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Inbox rows now show proposal type, state, risk, target, source, evidence count, age, unread/deferred markers, high-risk left-edge marker, and one-line blocker text for needs-attention/run-failed states.
- Filters cover status, type, risk, source, target, unread-only, and search text without creating a second proposal fetch loop.
- Keyboard shortcuts are handled globally outside text controls: `/`, `J`, `K`, `Enter`, `A`, `E`, `R`, `D`, and `Esc`.
- Batch approve/reject route through existing typed `transitionLaputaProposal`; read/unread and defer are UI-local markers because the backend has no durable read/deferred proposal state yet.
- Recoverable loading, empty, no-match, and error states are visible in the inbox, including retry for backend errors.
- Validation: `pnpm --dir agent-diva-gui test -- EvolutionView.test.ts` passed. `pnpm --dir agent-diva-gui build` and full `pnpm --dir agent-diva-gui test` remain blocked by unrelated pre-existing GUI issues recorded in verification logs.

### File List

- `_bmad-output/implementation-artifacts/2-2-build-proposal-inbox-list-and-filters.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `TODOLIST.md`
- `agent-diva-gui/src/components/EvolutionView.vue`
- `agent-diva-gui/src/components/evolution/ProposalInbox.vue`
- `agent-diva-gui/src/components/EvolutionView.test.ts`
- `docs/logs/2026-06-epic-2-inbox/v0.0.1-proposal-inbox-list-and-filters/summary.md`
- `docs/logs/2026-06-epic-2-inbox/v0.0.1-proposal-inbox-list-and-filters/verification.md`
- `docs/logs/2026-06-epic-2-inbox/v0.0.1-proposal-inbox-list-and-filters/release.md`
- `docs/logs/2026-06-epic-2-inbox/v0.0.1-proposal-inbox-list-and-filters/acceptance.md`

### Change Log

- 2026-06-14: Created ready-for-dev story for proposal inbox list and filters.
- 2026-06-15: Implemented proposal inbox list, filters, keyboard navigation, legal batch actions, local read/defer markers, states, and focused GUI tests.
