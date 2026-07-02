# Acceptance

## User-Facing Checks

1. Open Diva Pet voice settings and select `MiniMax` as TTS provider.
2. Trigger a desk pet reply and confirm audio synthesis still succeeds.
3. Switch TTS provider to `SiliconFlow` and trigger another reply.
4. Confirm standard synthesis still succeeds.
5. If a reference voice is configured for SiliconFlow, confirm cloned voice playback still works.

## Expected Results

- Provider switching does not break TTS playback.
- MiniMax continues to use its own command path.
- SiliconFlow continues to support standard synthesis and voice cloning paths.
- Browser fallback still works when provider synthesis fails.
