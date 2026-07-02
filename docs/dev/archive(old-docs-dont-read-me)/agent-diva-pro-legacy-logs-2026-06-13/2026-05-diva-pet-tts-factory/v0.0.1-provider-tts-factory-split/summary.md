# Iteration Summary

## Scope

- Refactor Diva Pet TTS service to split MiniMax and SiliconFlow synthesis logic into independent provider handlers created via a factory function.
- Preserve the public `ttsService` API and existing TTS fallback behavior.

## Changes

- Added provider factory context and handler interfaces in `agent-diva-gui/src/features/diva-pet/voice/services/tts-service.ts`.
- Extracted MiniMax synthesis logic into `MiniMaxTTSProviderHandler`.
- Extracted SiliconFlow standard / inline clone / reusable clone synthesis logic into `SiliconFlowTTSProviderHandler`.
- Updated `TTSService` to delegate provider-specific work through the factory instead of inline private implementations.
- Added a SiliconFlow-specific unit test and aligned the MiniMax expectation with the current default base URL.

## Impact

- Reduces coupling inside `TTSService`.
- Makes future provider-specific TTS extensions easier to add without continuing to grow one large service file.
- Keeps current desk pet TTS entry points and runtime behavior compatible with existing callers.
