# Summary

## What Changed

- Changed the default embedded Diva pet Gauss scene to `transparent`.
- Preserved `selectedGaussSceneId` and `gaussSceneList` from frontend local storage when hydrating pet settings from the core backend config.
- Kept scene selection frontend-only: the existing backend save path still writes voice/model pet fields only and does not add scene fields to the core config.

## Impact

- Applies to the full-screen Diva pet module opened from the sidebar.
- Existing user scene choices in GUI local storage are no longer overwritten by backend config hydration.
- Non-transparent scene selection behavior remains unchanged after the user selects a scene.
