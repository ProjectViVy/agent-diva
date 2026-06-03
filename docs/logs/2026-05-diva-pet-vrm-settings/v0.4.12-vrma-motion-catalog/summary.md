# Summary

## Changed

- Synced newly added VRMA files from `agent-diva-gui/avatar-runtime-vrm/public/vrm/animations` into the main GUI-served `agent-diva-gui/public/vrm/animations` directory.
- Expanded the Diva pet animation catalog with the new motions:
  - `Angry`, `Blush`, `Clapping`, `Goodbye`, `Jump`, `LookAround`, `Relax`, `Sad`, `Sleepy`, `Surprised`, `Thinking`.
- Added existing public motions `shoot`, `spin`, and `squat` to the catalog.
- Repaired Chinese display names in `vrm-animation-scanner.ts`.
- Classified `LookAround`, `Relax`, and `Sleepy` as idle motions available for appearance standby motion sets.
- Classified expression/action motions as `oneshot`, so they remain available for preview without entering standby loops.

## Impact

- New VRMA files are now available to the main GUI.
- The appearance editor's standby motion set shows the newly classified idle motions.
- The animation panel can preview the expanded one-shot motion catalog.
