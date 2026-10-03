# DN-6 — DIVA-only speech and avatar integration

- **Epic:** C · **Requirements:** R-5 · **Outcome:** input → transcript → VIVY turn → actual audio → avatar playback; mid-stream interrupt leaves no stale audio; reconnect does not re-speak history; disabling optional modules does not break text mode.
- **Authoritative design:** issue #13 DN-P1 §4 (Speech rules) · **Baseline:** `0fd005a1` · **Status:** see index; speech/resource contracts unverified · **Predecessor:** DN-2 (includes DN-5 host) · **Index:** [index.md](index.md)
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

## P0-D1 execution amendment (takes precedence)

### Concrete P0 domain slices

Consume P0-D1, DN-2's accepted run/cancel state and DN-0 speech/resource/native mappings. Initial host is the selected Tauri thin shell; the old non-Rust host premise no longer applies. Confirm exact ASR/provider-wrapper paths in DN-0 before editing; proposed correlation test `agent-diva-gui/src/features/diva-pet/voice/services/vivy-voice.test.ts`.

- [ ] Add tests for replay not re-speaking history, cancellation clearing queued playback, late audio after run replacement, missing device/resource and provider failure.
- [ ] Replace voice-api/tts-service and ledger-identified direct provider paths with verified VIVY capability operations; keep browser capture/playback and native window effects in DIVA.
- [ ] Bind media to backend run/utterance/resource IDs, invalidate queued output on cancellation and detach, and reject mismatched late results. Do not persist provider secrets in frontend state.
- [ ] Run real microphone/transcription/playback and interrupt/reopen scenarios on the frozen native host, alongside GUI tests/build; prove text-only mode works without optional media modules.

Every old voice/avatar/native call still needs a disposition. Optional module absence may be visible and valid only if the scope is explicitly optional; it is not permission to mark required legacy behavior migrated. Exact producer schemas/resource lifetime are blockers until DN-0 verifies them.
