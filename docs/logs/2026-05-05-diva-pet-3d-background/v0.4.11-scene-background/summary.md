# Iteration Summary

## Feature

3D Gaussian Splatting background scenes for the DivaPetView embedded desk pet panel. Users can now set a 3D scene background (indoor, beach, space) behind the VRM avatar, or leave it transparent (the default).

## Architecture

Reuses the existing `GaussSceneController` in `avatar-runtime-vrm`. Zero changes to the runtime layer. All work is in the Vue frontend: connecting props, adding UI controls, and wiring config persistence.

## Files Modified (6)

| File | Change |
|------|--------|
| `src/features/diva-pet/types.ts` | Added `GaussSceneId`, `GaussSceneEntry`, extended `PetConfig` with `selectedGaussSceneId` and `gaussSceneList` |
| `src/features/diva-pet/services/pet-config.ts` | `migrateConfig()` auto-fills scene fields for old configs, validates scene ID at load time |
| `src/features/diva-pet/vrm/components/DivaVrmAvatar.vue` | Added `backgroundScene` and `backgroundSceneUrl` props, `syncBackgroundScene()` with race condition protection, desktopPet mode skip |
| `src/features/diva-pet/components/DivaPetView.vue` | Scene quick-switch button (Image icon) with dropdown menu, passes `backgroundScene` prop to DivaVrmAvatar |
| `src/components/settings/PetSettings.vue` | "3D background scene" section with radio button selection |
| `src/features/diva-pet/index.ts` | Re-export `GaussSceneId`, `GaussSceneEntry` types |

## Files Created (4)

| File | Purpose |
|------|---------|
| `src/features/diva-pet/types.test.ts` | GaussScene type defaults: `selectedGaussSceneId === 'transparent'`, `gaussSceneList.length === 4` |
| `src/features/diva-pet/services/pet-config.test.ts` | Config migration for scene fields, invalid ID fallback |
| `src/features/diva-pet/vrm/components/DivaVrmAvatar.test.ts` | Scene integration: prop wiring, load failure fallback, race protection, desktopPet skip |
| `src/features/diva-pet/components/DivaPetView.test.ts` | Scene button UI, dropdown behavior, click-outside, active highlight, config sync |

## Assets Deployed (3)

- `public/vrm/scene/home.spz` (indoor scene)
- `public/vrm/scene/sea.spz` (beach scene)
- `public/vrm/scene/space.spz` (space scene)

## Scope Notes

- `avatar-runtime-vrm/` — zero changes
- Tauri backend — zero changes
- `NormalMode.vue` — zero changes (pass-through unchanged)
- `DesktopPetOverlay.vue` — zero changes (desktop overlay mode out of scope)
