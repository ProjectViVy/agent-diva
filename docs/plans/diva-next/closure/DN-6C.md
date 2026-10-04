# DN-6C — Connect main-chat voice and generation-safe playback Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` task by task after implementation authorization. Use `superpowers:subagent-driven-development` only when delegation is explicitly authorized. Read the specification and this plan; check the parent index before starting.

**Goal:** Offer explicit mic→editable transcript and fresh-answer TTS in main chat with secure settings and reliable interruption.
**Architecture:** Reuse recorder/player/preprocessing algorithms while moving active voice outside dormant pet code. One voice controller owns tracks, Blob URLs, context generation and fresh-answer eligibility.
**Tech Stack:** Vue/Web Audio, Tauri typed native facade, Vitest
**Spec:** [DN-C2](../p0-design.md), especially the sections cited below. [Contract ledger](../backend-separation-contracts.md) is the shared schema authority.
**Epic / requirements:** DN-6 / R-3, R-5, R-9.
**State and immediate dependencies:** [authoritative index](../index.md#executable-story-package). The index owns readiness; this file does not duplicate status.
**Baseline:** DIVA `f5866a0`, VIVY `1db8b55`, Laputa `dc6066e`; exact pins are in the index. Reconcile changed source before execution.

## Global Constraints

- One VIVY Agent; one Garden owner per configured profile; no restored Rust business crates, HTTP domain daemon, duplicate authority or hidden writer fallback.
- Keep the five C exports and `vivy_call` / `vivy:event`; speech bytes stay outside Go JSON. No old-data import, local voice, generic resources or pet-host restoration.
- Native final acceptance is Windows x64 and belongs to the owner; engineering evidence and unavailable checks are reported separately.
- Read each owning repository's instructions and claim its lock before edits. Paths below are relative to that repository. New files are **proposed**; generated output is regenerated, never hand-edited.
- Use current [execution constraints and handoff](../index.md#execution-contract). No product test or implementation has been performed by generating this plan.

## Review Focus

- STT failure differs from no_speech and never automatically sends a transcript.
- Old binary Promise resolving after session switch/interrupt is dropped and URL revoked.
- Controller reload must increment native generation rather than reset to zero.
- New recording/regeneration/hide/Quit releases tracks, nodes and Blob URLs.
- Long TTS text/reference incompatibility is explicit, not silently truncated/dropped.

## Task 1: Deliver the Story boundary

### Files

- **DIVA / modify existing:** `agent-diva-gui/src/platform/desktop-host.ts`
- **DIVA / modify existing:** `agent-diva-gui/src/App.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/ChatView.vue`
- **DIVA / modify existing:** `agent-diva-gui/src/components/SettingsView.vue`
- **DIVA / modify existing:** `scripts/ci/check_legacy_frontend_calls.mjs`
- **DIVA / create proposed:** `agent-diva-gui/src/api/speech.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/features/voice/useVoiceController.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/features/voice/useVoiceController.test.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/features/voice/encodeWav.ts`
- **DIVA / create proposed:** `agent-diva-gui/src/features/voice/VoiceControls.vue`
- **DIVA / create proposed:** `agent-diva-gui/src/features/voice/SpeechSettings.vue`

### Interfaces

Consumes DN-6B C2-4 native commands/MP3/events, DN-2B conversation invalidation and OBS-08 bounded recorder. Produces typed native facade and one controller: explicit start/Stop→decode/resample16k PCM16 WAV→STT→editable draft; Send stays the normal Agent path.

Only desktop-host.ts calls literal invokes/listen, including Raw/header arguments. `speech_synthesize` resolves ArrayBuffer tied to captured Promise identity. Before any new request, obtain native current window generation from config_get, increment and await context_set; cancel stops local audio/tracks first. Reply usable iff full captured identity equals current context. Failed/cancelled/cognitive/child/replayed/reopened runs never auto-read. Auto-read is opt-in; explicit message replay creates a fresh utterance. UI key forms clear after set/delete/close; no localStorage key/PetConfig persistence.

### Ordered steps

- [ ] **Step 1:** Add `sttDraftNeverAutoSends`, `lateMp3NoPlayback`, `reloadReadsGeneration`, `onlyFreshPrimaryAutoReads`, `allExitPathsReleaseAudio` tests with deferred native promises and actual request identities.

- [ ] **Step 2:** Run tests red; implement encodeWav and move reusable recorder/player/preprocessor logic into active `features/voice`. Reuse optional existing avatar runtime only for main-window lip motion; do not import DesktopPetApp or legacy voice-api services.

- [ ] **Step 3:** Add typed native functions to desktop-host/speech facade and controller fencing. Stop local media first on invalidation, advance/ack native context, then admit later work; stale completions cannot restart audio.

- [ ] **Step 4:** Mount explicit mic/Stop/transcript/replay/interrupt and secure native SpeechSettings independently of pet windows. Config/key/asset operations show CAS/partial/unavailable states and clear transient key fields.

- [ ] **Step 5:** Route safe speech diagnostics into OBS-08 recorder. Plain browser mode reports native_unavailable, text chat stays usable. Add EN/ZH errors and verify no active cloud fetch/old pet_* path remains reachable.

- [ ] **Step 6:** Run GUI/boundary and available native record/STT/TTS/interrupt smoke; commit `feat(voice): connect online speech to main chat`. Leave cloud credentials/recognition/listening quality to owner acceptance.

### Verification

`pnpm --dir agent-diva-gui exec vitest run src/features/voice/useVoiceController.test.ts` plus proposed WAV/settings/component tests; `just gui-test`, `just gui-build`; AST selftest and native boundary gate. Expected: draft-only STT, opt-in fresh primary playback, no late playback, native-generation restart safety and resource counters at zero after teardown. Native device/codecs require actual host evidence.

### Acceptance and handoff

Return copied/moved reusable source map, typed native facade, generation/cancel/resource tests, diagnostics redaction and available native smoke. DN-P-C bundles these exact commits. No realtime duplex, VAD, sentence queue, local voice or plugin framework is added.
