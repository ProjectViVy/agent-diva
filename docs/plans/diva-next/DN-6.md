# DN-6 — DIVA-only speech and avatar integration

- **Epic:** C · **Requirements:** R-5 · **Outcome:** input → transcript → VIVY turn → actual audio → avatar playback; mid-stream interrupt leaves no stale audio; reconnect does not re-speak history; disabling optional modules does not break text mode.
- **Authoritative design:** issue #13 DN-P1 §4 (Speech rules) · **Baseline:** `0fd005a1` · **Status:** Blocked — VIVY speech/resource/event contracts under #63 · **Predecessor:** DN-5 · **Index:** [index.md](index.md)
- **Files:** `src/features/diva-pet/voice/services/{voice-api,tts-service,asr-service}.ts`, provider wrappers/composables, `DesktopPetOverlay.vue`, `avatar-runtime-vrm/`, `shared-avatar-protocol/`; Rust voice/VRM commands are retirement references. **Escalate:** speech capability absent in pinned VIVY and not optional → parity blocker.

## Prerequisites / contracts

- Consumes DN-5 host + VIVY optional-capability contracts for synthesis/transcription/resource import-read-delete.
- Playback linked to active run/utterance identity; cancel/barge-in invalidates queued output; browser owns playback/capture; credentials and resource authority stay in VIVY.

## Tasks

- [ ] Move provider credentials, synthesis/transcription, resource import/read/delete, voice configuration onto real VIVY optional capability contracts.
- [ ] Keep capture, playback, subtitles, avatar rendering, native windows on the DIVA side.
- [ ] Replace old Tauri/neuro-link response delivery with run/utterance-correlated events; replay must not generate duplicate speech.
- [ ] Implement cancellation/barge-in cleanup; clear missing-device/provider/resource errors.
- [ ] Delete legacy direct provider bypasses and voice/asset business commands once the replacement is accepted.

## Verification

- `pnpm --dir agent-diva-gui test` and `build` pass.
- Live chain smoke with real audio out; interrupt test; reconnect test; Lite mode unaffected.

## Evidence to supervisor

Chain smoke evidence, interrupt/reconnect results, list of retired bypasses.
