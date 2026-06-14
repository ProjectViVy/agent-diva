# Release — Sprint 1

## Deployment Method

Not applicable — Sprint 1 is an intermediate milestone. Changes are feature-gated behind Diva Pet sidebar entry (Sprint 0 pet.enabled config flag).

## Build Instructions

```bash
cd agent-diva-gui
pnpm install        # ensure three + @pixiv/three-vrm are installed
pnpm tauri dev      # development mode
pnpm tauri build    # production build
```

## Rollback

To disable VRM rendering:
1. Open browser DevTools → Application → Local Storage
2. Set `agent-diva-pet-config` to `{"enabled":false,...}`
3. Refresh — Diva Pet sidebar entry disappears

## Breaking Changes

None. All changes are incremental within the `diva-pet` feature module.

## Dependencies

| Package | Version | License |
|---------|---------|---------|
| `three` | ^0.184.0 | MIT |
| `@pixiv/three-vrm` | ^3.5.2 | MIT |

## Version

v0.0.2-sprint1-vrm-mvp (internal milestone)
