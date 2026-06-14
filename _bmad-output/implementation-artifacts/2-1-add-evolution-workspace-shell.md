---
baseline_commit: 1c02af7f
---

# Story 2.1: Add Evolution Workspace Shell

Status: ready-for-dev

## Story

As a user,
I want a top-level Evolution workspace,
so that governance review is visible and separate from settings or Notebook content.

## Acceptance Criteria

1. Given the GUI is loaded, when proposals or governance events exist, then the sidebar shows an Evolution workspace with a count badge.
2. The workspace contains Inbox, Runs, Audit, and Policy tabs.
3. The Inbox is the default tab.
4. Responsive layout switches between split and collapsed list/detail states without overlapping text.

## Tasks / Subtasks

- [ ] Add `Evolution` as a top-level sidebar section in `agent-diva-gui/src/components/NormalMode.vue`. (AC: 1)
- [ ] Create `agent-diva-gui/src/components/EvolutionView.vue` as the shell component with tabs `Inbox`, `Runs`, `Audit`, and `Policy`. (AC: 2, 3)
- [ ] Add typed Laputa DTOs and invoke wrappers in `agent-diva-gui/src/api/desktop.ts` for proposal list and event polling. (AC: 1)
- [ ] Load pending proposal/event counts through existing Tauri commands and show a badge on the Evolution nav item. (AC: 1)
- [ ] Implement desktop split layout and narrow-width list/detail layout guardrails in the shell, even if child panels are placeholders in this story. (AC: 4)
- [ ] Add i18n keys in `agent-diva-gui/src/locales/zh.ts` and `agent-diva-gui/src/locales/en.ts`. (AC: 1, 2)
- [ ] Add or update focused GUI smoke/unit coverage where available; at minimum run a GUI build/typecheck path and a real Tauri command smoke if the environment supports it. (AC: 1-4)

## Dev Notes

### Architecture Context

- Epic 2 implements the visible SelfImprove / Evolution governance console. It must not write `.laputa/` files directly.
- The authority spine remains: `EvidenceRef -> EvolutionProposal -> user review -> Laputa apply -> changelog / audit -> rollback -> prompt / report consumption`.
- Existing backend/Tauri Laputa commands from Story 1.5 are the source of truth for shell counts:
  - `laputa_list_proposals`
  - `laputa_poll_events`
  - `laputa_list_changelog`
- Existing manager routes are under `/api/laputa/*`; do not introduce a second frontend-only data source.

### Current GUI Entry Points

- `agent-diva-gui/src/components/NormalMode.vue` owns the sidebar and main workspace switching.
- Add `evolution` to `SidebarSection`, `activeMenu`, navigation, overlay navigation, and main content rendering.
- Current primary workspaces are `chat`, `notebook`, `pet`, `console`, plus capability/tool sections. Place `Evolution` between `Chat` and `Notebook` per UX design.
- Use `lucide-vue-next`; preferred icons are `GitBranch`, `WandSparkles`, `Activity`, or an equivalent existing import.
- Do not use the reserved `neuro` placeholder for this feature.

### API / DTO Requirements

- Add typed wrappers to `desktop.ts` instead of calling `invoke` directly from every component.
- Frontend DTOs must mirror `agent-diva-core/src/evolution/types.rs`:
  - `EvolutionProposal`
  - `ProposalState`
  - `ProposalType`
  - `RiskLevel`
  - `LaputaSectionName`
  - `ChangelogRecord`
  - `AutoDreamRunRecord`
- `laputa_list_proposals` currently accepts `since?: string` only. Filtering can be client-side in Story 2.2 unless backend filters are extended deliberately.
- `laputa_poll_events(kind, since)` supports `proposals`, `changelog`, and `errors` event kinds through manager routing.

### UX Requirements

- Evolution is an operational review console, not a landing page.
- Default tab is `Inbox`.
- Badge priority:
  1. danger: failed run or blocked proposal requiring attention;
  2. warning: pending review;
  3. accent: informational new journal/report output.
- Responsive minimums from UX:
  - proposal list: 320px;
  - detail panel: 520px;
  - collapse evidence/diff secondary panel under 1180px.
- No overlapping text, no spinner-only panels, no decorative hero/gradient UI.

### Boundaries

- This story creates the shell and navigation. Detailed inbox rows, filters, proposal detail, runs, audit, policy behavior are implemented in Stories 2.2-2.4.
- Do not create AutoDream runtime in this story. Epic 3 owns the AutoDream crate and run lifecycle.
- Do not expose auto-merge as an enabled durable-change path.

### Testing Requirements

- Verify `NormalMode.vue` can navigate to `EvolutionView` from expanded, collapsed, and overlay/mobile sidebar modes.
- Verify badge count handles empty, loading, error, and non-empty states.
- Verify default active tab is Inbox.
- Verify narrow viewport switches layout without horizontal text overlap.
- Minimum validation:
  - `just fmt-check` if Rust files change.
  - GUI type/build command used by this repo if available.
  - A smoke run of the GUI shell or documented reason if Tauri GUI smoke is unavailable.

### References

- `_bmad-output/planning-artifacts/epics.md` - Epic 2, Story 2.1.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - authority spine and component responsibilities.
- `_bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/DESIGN.md` - sidebar and layout design.
- `_bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/EXPERIENCE.md` - supervised autonomy flow.
- `agent-diva-gui/src/components/NormalMode.vue` - existing shell.
- `agent-diva-gui/src/api/desktop.ts` - existing Tauri API wrapper.
- `agent-diva-gui/src-tauri/src/commands.rs` - Laputa Tauri commands.

## Dev Agent Record

### Agent Model Used

TBD by dev agent.

### Debug Log References

- 2026-06-14: Story context prepared from Epic 2, EVO-DIVA architecture, Evolution UX design, existing GUI shell, and Story 1.5 Laputa API exposure.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.

### File List

- `_bmad-output/implementation-artifacts/2-1-add-evolution-workspace-shell.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-06-14: Created ready-for-dev story for Evolution workspace shell.
