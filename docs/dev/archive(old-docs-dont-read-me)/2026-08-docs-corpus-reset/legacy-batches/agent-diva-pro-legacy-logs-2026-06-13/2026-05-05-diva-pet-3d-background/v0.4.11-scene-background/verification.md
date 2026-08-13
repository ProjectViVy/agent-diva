# Verification

## Type Check

```bash
cd agent-diva/agent-diva-gui
npx vue-tsc --noEmit
```

Result: Only pre-existing TS6133 errors in external files (`avatar-runtime-vrm`, `DesktopPetOverlay.vue`). Zero new type errors from the scene background changes.

## Unit Tests

```bash
cd agent-diva/agent-diva-gui
npm run test
```

Result: 15 test files, 248 tests — ALL PASSED

## Key Verification Scenarios

### Types (`types.test.ts`)

| Scenario | Expectation | Status |
|----------|-------------|--------|
| `DEFAULT_PET_CONFIG.selectedGaussSceneId` | `'transparent'` | Pass |
| `DEFAULT_PET_CONFIG.gaussSceneList.length` | `4` | Pass |
| Each `GaussSceneEntry` shape | Has `id`, `name`, `path`, `isDefault` | Pass |
| `transparent` entry path | Empty string `""` | Pass |

### Config Migration (`pet-config.test.ts`)

| Scenario | Expectation | Status |
|----------|-------------|--------|
| Old config (no scene fields) | Auto-fills `selectedGaussSceneId: 'transparent'`, `gaussSceneList` with 4 entries | Pass |
| Invalid scene ID | Falls back to `'transparent'` | Pass |
| Valid scene ID (`home`) | Preserved as-is | Pass |

### DivaVrmAvatar Scene Integration (`DivaVrmAvatar.test.ts`)

| Scenario | Expectation | Status |
|----------|-------------|--------|
| `backgroundScene='home'` prop | `runtime.setBackgroundScene('home', undefined)` called | Pass |
| `backgroundScene='transparent'` | `setBackgroundScene('transparent', ...)` called, no model reload | Pass |
| Scene switch (`home` -> `sea`) | Old scene disposed, `setBackgroundScene('sea', ...)` called | Pass |
| Scene load failure | Rejects -> calls `setBackgroundScene('transparent')` fallback | Pass |
| Rapid 3 switches (`home`->`sea`->`space`->`transparent`) | All calls made, sequence number race protection active | Pass |
| Model loading phase scene set | Scene still applied after model load completes | Pass |
| `desktopPet=true` | `setBackgroundScene()` NOT called (scene loading skipped) | Pass |

### DivaPetView Scene UI (`DivaPetView.test.ts`)

| Scenario | Expectation | Status |
|----------|-------------|--------|
| Scene button visible | `Image` icon rendered next to gear button | Pass |
| Click button -> dropdown | Shows 4 scene names (`透明背景`, `室内场景`, `海边场景`, `太空场景`) | Pass |
| Active scene highlighted | Active scene has `text-pink-600` class, others don't | Pass |
| Click scene item | `config.selectedGaussSceneId` updates | Pass |
| Click outside | Dropdown closes, scene items removed | Pass |

### PetSettings UI

| Check | Status |
|-------|--------|
| "3D background scene" section present between "Basic settings" and ASR sections | Pass |
| 4 radio options with icons and descriptions | Pass |
| Radio selection matches `config.selectedGaussSceneId` | Pass |
| Selection updates config reactively | Pass |
