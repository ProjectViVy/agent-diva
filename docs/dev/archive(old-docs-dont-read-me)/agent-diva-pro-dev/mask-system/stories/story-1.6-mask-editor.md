# Story 1.6: Create, Edit, and Ship Predefined Masks

**Epic:** Epic 1 — Mask Management & User-Facing Lifecycle
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 1.2

## Story

As a DiVA user,
I want to create and edit masks in the GUI and start from useful built-in mask patterns,
So that I can personalize mask behavior without manually authoring every file from scratch.

## Acceptance Criteria

- [ ] AC1: Settings > Masks shows mask card grid
- [ ] AC2: Create button opens mask editor modal
- [ ] AC3: Editor supports basic mode (form) and advanced mode (YAML)
- [ ] AC4: Predefined masks available: researcher, coder, reviewer, assistant
- [ ] AC5: Delete mask with confirmation

## Tasks

- [ ] Create `agent-diva-gui/src/components/settings/MaskSettings.vue` — settings panel
- [ ] Create `agent-diva-gui/src/components/settings/MaskEditorModal.vue` — editor modal
- [ ] Create predefined mask files in `workspace/masks/`
- [ ] Implement save/delete operations via Tauri IPC

## Dev Notes

- UX: MaskSettingsPanel and MaskEditorModal
- Reuse ChannelCard/ProviderWizardModal patterns
- Predefined masks: researcher🔍, coder💻, reviewer📝, assistant🤖

## File List

- `agent-diva-gui/src/components/settings/MaskSettings.vue` (new)
- `agent-diva-gui/src/components/settings/MaskEditorModal.vue` (new)
- `workspace/masks/researcher.md` (new)
- `workspace/masks/coder.md` (new)
- `workspace/masks/reviewer.md` (new)
- `workspace/masks/assistant.md` (new)