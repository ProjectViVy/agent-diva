# VRM runtime warning cleanup

## Summary

- Removed the deprecated `VRMUtils.removeUnnecessaryJoints` call from the VRM model loading path.
- Added an internal VRMA loader guard that ensures `VRMLookAtQuaternionProxy` exists before creating VRM animation clips.
- Kept `VRMUtils.combineSkeletons` as a best-effort optimization with the existing tolerant error handling.
- Added missing locale keys for chat attachment and voice controls.
- Moved async startup interval cleanup out of the awaited `onMounted` flow to avoid Vue lifecycle registration warnings.
- Replaced deprecated `PCFSoftShadowMap` with `PCFShadowMap`.

## Impact

- Scope covers `avatar-runtime-vrm` and warning cleanup in the GUI shell / pet voice panel.
- Public runtime APIs and GUI workflows are unchanged.
- The upstream `THREE.Clock` warning from `@sparkjsdev/spark` remains outside this change and should be handled through dependency upgrade or a separate patch workflow.
