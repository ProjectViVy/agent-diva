# Desktop Pet Camera Position Summary

## Changes

- Added mode-specific default avatar transforms in `avatar-runtime-vrm`.
- Set `desktop-pet` mode to a front-facing, level default camera.
- Kept embedded avatar mode on the existing angled default view.
- Reused the same default transform source for scene initialization and runtime transform reset.
- Explicitly re-applied the desktop-pet camera preset after model load, scale changes, and render resume.
- Synchronized `OrbitControls` immediately after programmatic camera changes so stale internal orbit state cannot pull the camera back on the next frame.
- Tuned the shared camera baseline to `distance=3.0`, `targetY=0.6`, and added `targetX=-0.12` for final horizontal framing.

## Impact

- Desktop pet windows open with the avatar centered and facing the camera by default.
- The desktop-pet camera baseline now has explicit X/Y target constants instead of relying on transform offsets for default framing.
- Desktop pet windows should keep the same front-facing camera after hide/show or render pause/resume.
- Existing embedded preview behavior is intentionally unchanged.
