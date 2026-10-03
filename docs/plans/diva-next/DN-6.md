# DN-6 — DIVA-native online speech and media

Current scope: [DN-C2](p0-design.md); status/dependencies: [index.md](index.md).
The owner's 2026-10-03 directive supersedes the VIVY-speech-module prerequisite.
DN-C2 supplies detailed architecture and exact proposed commands in the
ledger. Implementation/native/provider probes remain pending; not Ready.

## Outcome and ownership

SiliconFlow STT plus SiliconFlow/MiniMax TTS work through Tauri native media
services and the existing text Agent path. Vue captures/plays/subtitles/renders;
Tauri owns cloud HTTP, speech secrets and bounded reference assets. WAV
uploads/MP3 replies use Raw/Response IPC; Vue owns temporary Blob audio.
VIVY remains sole Agent/run/approval authority. OLVRS/local ONNX and
pet window/neuro-link restoration are not prerequisites.

## Existing files and proposed additions

Reuse `agent-diva-gui/src/features/diva-pet/voice/` provider/player/preprocessing
logic, avatar-runtime-vrm and main chat/settings. Change the frozen native
seam `src/platform/desktop-host.ts`, shell `src-tauri/src/lib.rs` and lifecycle,
plus AST/native/boundary gates. Proposed native module `src-tauri/src/speech/`;
proposed typed facade `src/api/speech.ts`. Do not restore old business crates.

## Required proof for designed contracts

- Implement ledger command/config DTOs and OS keyring adapter; probe secure
  persistence/locked/unavailable states and presence-only readback.
- Bundled Tauri Raw/Response IPC and WebView decode/resample to PCM16 WAV on
  Windows/Linux; bounded reference-voice commands and leases. No audio handles
  or speech_release_audio command; no general VRM manager.
- Real provider request/error fixtures, supported region/base URL/model.
- Request/session/run/utterance/generation correlation; abort and release.
- Narrow native command allowlist; remove activated speech from dormant scope.

## Work and evidence

Replace old pet_* and frontend cloud fetch; make voice reachable independently
of dormant pet windows. Distinguish provider/device/credential/no-speech
failures, preserve reference configuration or reject unsupported use visibly.
Bind fresh answers only; never auto-speak history/replay. Cancel releases audio
and discards late output; text mode survives cloud failure. Shutdown aborts
owned speech requests before native resources are closed.

Scoped checks cover real request shape, redaction, bounds, cancel races,
resource release and replay exclusion. Prepare installed mic -> STT -> Agent
-> both TTS providers -> playback/VRM -> interrupt -> Quit scenarios for owner
acceptance. No cloud quality/native success is claimed by this design.
