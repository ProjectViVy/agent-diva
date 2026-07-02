# Shared Avatar Protocol

Transport-neutral protocol types for Diva host applications and avatar runtimes.

## Scope

Version `0.1.0` intentionally covers the minimum shared contract needed for:

- runtime initialization and lifecycle
- character loading
- mood and speech-state updates
- avatar transform updates
- metrics and runtime error reporting

It does not include:

- Vue, Tauri, DOM, Three.js, or Live2D SDK types
- TTS/ASR configuration schemas
- chat/session/application-level state

## Public modules

- `types.ts`: shared value types
- `commands.ts`: host-to-runtime command map
- `events.ts`: runtime-to-host event map
- `runtime.ts`: transport-neutral runtime and bridge interfaces

## Design note

The package is intentionally independent from the current `agent-diva` app structure so it can be reused by:

- `agent-diva` host adapters
- future `avatar-runtime-vrm`
- future `avatar-runtime-live2d`
