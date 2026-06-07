# Release

No special deployment procedure is required.

## Delivery Method

- Ship with the normal `agent-diva-gui` desktop build pipeline.
- No config migration is needed because the existing `desktopPetScale` field is reused.

## Rollback

- Revert `agent-diva-gui/src/features/diva-pet/components/DesktopPetOverlay.vue` to remove the wheel listener and shared scale update path.
