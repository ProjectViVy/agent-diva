# Iteration Summary

## Scope

- Restored VRM visibility inside the embedded Diva Pet page while the separate desktop pet window is active.

## Changes

- Updated `agent-diva-gui/src/features/diva-pet/components/DivaPetView.vue`.
- Removed the `v-if="!desktopPetActive"` gate on the embedded `DivaVrmAvatar`.
- Replaced the full-page placeholder with a lightweight status badge so the embedded page keeps rendering the avatar while still indicating that desktop pet mode is active.

## Expected Impact

- The Diva Pet page no longer appears as if VRM failed to load just because the desktop pet button is enabled.
- Users can now view the in-page VRM and the floating desktop pet at the same time.
