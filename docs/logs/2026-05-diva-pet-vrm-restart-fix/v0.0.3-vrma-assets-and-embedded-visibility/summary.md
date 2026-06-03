# Iteration Summary

## Scope

- Restored the original embedded/desktop-pet exclusivity behavior.
- Fixed the embedded Diva Pet avatar appearing static by adding the missing VRMA motion assets to the GUI public bundle.

## Changes

- Updated `agent-diva-gui/src/features/diva-pet/components/DivaPetView.vue` so embedded VRM rendering is hidden again when desktop pet mode is active.
- Populated `agent-diva-gui/public/vrm/animations/` with the built-in VRMA motion files required by the runtime motion catalog.

## Expected Impact

- When desktop pet mode is enabled, the embedded pet panel no longer renders the in-page VRM.
- When embedded mode is visible, idle VRMA motions can now load from the GUI bundle instead of falling back to an almost static pose.
