# v0.4.10 pnpm build fix summary

## Changes

- Fixed `pnpm build` TypeScript failures caused by unused runtime imports and fields.
- Tightened `NormalMode` config props to match the required `SettingsView` contract.
- Forwarded `ChatView` send payloads with explicit `(content, attachments)` arguments.
- Normalized optional `isTyping` handling in `useVoicePlayer` with a default `Ref<boolean>`.

## Impact

- Scope is limited to GUI build correctness and TypeScript type safety.
- No intended change to VRM runtime behavior, chat send behavior, or TTS playback semantics.
