# Summary

## Changes

- Added a `startup` VRMA motion kind alongside `idle` and `oneshot`.
- Classified `appearing.vrma` and `greeting.vrma` as startup motions in the GUI scanner and runtime catalog.
- Added `startMotionId` to VRM appearance configuration, defaulting and migrating to `appearing`.
- Added a startup motion selector to the appearance editor, limited to `appearing` and `greeting`.
- Updated the animation panel to separate idle, startup preview, and one-shot preview motion groups.
- Wired startup motion playback into embedded and desktop pet avatar entry points.

## Impact

- Existing appearance configs without `startMotionId` are migrated to `appearing`.
- Runtime one-shot playback now allows both `startup` and `oneshot` motion kinds.
- Desktop and embedded pet views replay the configured startup motion after model load or appearance switch.
