# Acceptance

## User Steps

- Open the GUI settings animation page and verify new VRMA motions are listed.
- Preview `Clapping`, `Goodbye`, and `Thinking`; playback should start without `MOTION_NOT_FOUND`.
- Select `LookAround`, `Relax`, and `Sleepy` as idle motions and enable idle animation; runtime should retain those selected IDs.

## Technical Acceptance

- Runtime builtin motion catalog and GUI scanner known catalog expose the same 22 motion IDs.
- Runtime source paths match GUI scanner paths for all known motions.
- Runtime demo animation directory contains every referenced `.vrma` file.
