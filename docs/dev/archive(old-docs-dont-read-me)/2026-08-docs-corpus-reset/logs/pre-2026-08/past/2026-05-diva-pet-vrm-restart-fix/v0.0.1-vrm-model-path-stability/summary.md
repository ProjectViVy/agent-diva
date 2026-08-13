# Iteration Summary

## Scope

- Fixed the Diva Pet VRM model path resolution flow in `agent-diva-gui`.
- Stabilized model selection state so existing configs saved as `Alice`, `Alice.vrm`, or `/vrm/models/Alice.vrm` all resolve to the same active model.

## Changes

- Added `src/features/diva-pet/utils/vrm-model.ts` to centralize VRM model path resolution and model-id normalization.
- Updated `DivaPetView.vue` and `DesktopPetOverlay.vue` to use the shared resolver instead of hardcoding `/${name}.vrm`.
- Updated `DivaPetModelManager.vue` to:
  - compare active model state via normalized model id;
  - persist `model.path` when switching models, which avoids extension mismatch on future reloads.
- Added `src/features/diva-pet/utils/vrm-model.test.ts` to cover empty/default values, extension handling, and normalized id comparison.

## Expected Impact

- Prevents malformed paths like `/vrm/models/Alice.vrm.vrm`.
- Avoids repeated VRM load retries caused by path mismatch, which could appear to users as the avatar or window repeatedly restarting.
- Keeps backward compatibility with older saved values while normalizing newly saved values to the file path form.
