# Release

Version: 0.4.11 (patch)

## CHANGELOG

```markdown
## [0.4.11] - 2026-05-05

### Added
- DivaPetView embedded desk pet supports 3D Gaussian Splatting background scenes
- 3 preset scenes: indoor (home), beach (sea), space (cosmos) + transparent default
- Scene files (.spz) deployed to public/vrm/scene/
- DivaPetView scene quick-switch button (gear +4px) with dropdown menu
- PetSettings "3D background scene" configuration section with radio button selection
- Backend: GaussScene types, config migration, auto-fallback on load failure
- Race condition protection for rapid scene switching (sequence number pattern)
- TDD test suite: 10+ new tests across 4 test files
```

## Delivery Method

Ship with the normal `agent-diva-gui` desktop build pipeline. No special deployment steps required.

- Scene `.spz` files are static assets served from `public/vrm/scene/`
- Config persistence uses existing `usePetConfig` / `migrateConfig()` path
- No Tauri sidecar, no native plugin, no npm dependency added

## Rollback

1. Revert the 6 modified files to the pre-0.4.11 state
2. Remove the 4 new test files
3. Scene `.spz` files in `public/vrm/scene/` can remain (unreferenced after revert, no harm)

## Release Notes (Chinese)

### Diva Pet 3D Background Scenes

The embedded desk pet (DivaPetView) now supports 3D background scenes:

- Indoor scene
- Beach scene
- Space scene

Usage: Click the scene button (Image icon) next to the gear button in DivaPetView, then select from the dropdown. Alternatively, go to Settings -> Pet Settings -> "3D background scene" section.

Architecture note: The runtime layer `avatar-runtime-vrm` has zero changes. All work is in the Vue frontend layer. The `@sparkjsdev/spark` dependency is passed through `avatar-runtime-vrm` -- no direct install needed.
