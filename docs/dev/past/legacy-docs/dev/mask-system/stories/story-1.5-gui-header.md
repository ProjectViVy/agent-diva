# Story 1.5: Add Mask Status Header to GUI

**Epic:** Epic 1 — Mask Management & User-Facing Lifecycle
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 1.4

## Story

As a DiVA user,
I want to see the current mask in the GUI header,
So that I always know which mask is active.

## Acceptance Criteria

- [ ] AC1: Topbar avatar shows current mask emoji
- [ ] AC2: Clicking avatar opens mask switcher popover
- [ ] AC3: Popover shows mask list with current mask highlighted
- [ ] AC4: "管理面具" link navigates to Settings > Masks

## Tasks

- [ ] Create `agent-diva-gui/src/components/MaskSwitcher.vue` — popover component
- [ ] Modify `agent-diva-gui/src/components/NormalMode.vue` — add mask switcher to topbar
- [ ] Create `agent-diva-gui/src/composables/useMask.ts` — mask state management
- [ ] Implement Tauri IPC calls for mask operations

## Dev Notes

- UX: MaskSwitcherPopover in topbar-avatar area
- Use existing CSS variables (--panel, --accent, etc.)
- Emoji-based icons

## File List

- `agent-diva-gui/src/components/MaskSwitcher.vue` (new)
- `agent-diva-gui/src/composables/useMask.ts` (new)
- `agent-diva-gui/src/components/NormalMode.vue` (modify)