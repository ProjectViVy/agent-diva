# Release

## Method

Not released yet in this iteration.

## Suggested Rollout

1. Start gateway/manager with updated binaries.
2. Launch GUI and verify stop behavior against:
   - local backend deployment
   - remote backend deployment exposing `/api/chat/stop`
3. After smoke verification, include in next tagged release.
