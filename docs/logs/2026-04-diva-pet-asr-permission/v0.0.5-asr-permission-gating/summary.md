# ASR Permission Gating Summary

## Changes

- Added an explicit `navigator.mediaDevices.getUserMedia({ audio: true })` permission preflight before enabling Web Speech API ASR.
- Prevented persisted ASR configuration from auto-starting microphone capture during GUI startup or config hydration.
- Updated Diva Pet voice toggling so failed permission requests roll the ASR config back to disabled.
- Added focused Vitest coverage for permission granted and denied ASR enable flows.

## Impact

- User-visible ASR behavior in `agent-diva-gui`.
- No provider, channel, or backend routing behavior changed.
