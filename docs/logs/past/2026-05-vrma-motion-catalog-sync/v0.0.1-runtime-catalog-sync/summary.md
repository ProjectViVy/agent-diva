# Iteration Summary

## Changed

- Expanded `avatar-runtime-vrm` builtin motion catalog to include all 22 VRMA motions exposed by the GUI animation scanner.
- Added a catalog sync test that compares GUI scanner motion IDs, kinds, and paths with runtime catalog IDs, kinds, and sources.
- Copied missing runtime demo VRMA assets into `agent-diva-gui/avatar-runtime-vrm/public/vrm/animations`.

## Impact

- New one-shot motions such as `Clapping`, `Goodbye`, and `Thinking` are registered by the runtime and no longer fail with `MOTION_NOT_FOUND`.
- New idle motions such as `LookAround`, `Relax`, and `Sleepy` are accepted by runtime idle motion selection instead of being filtered out.
- Standalone runtime demo has the same animation asset coverage as the GUI bundle.
