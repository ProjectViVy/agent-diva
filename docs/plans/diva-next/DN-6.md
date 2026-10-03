# DN-6 — DIVA-native online speech and media

Current scope: [DN-C1](p0-design.md); status/dependencies: [index.md](index.md).
The owner's 2026-10-03 directive supersedes the VIVY-speech-module prerequisite.
This is a preliminary work outline; detailed contracts/plans are not Ready.

## Outcome and ownership

SiliconFlow STT plus SiliconFlow/MiniMax TTS work through Tauri native media
services and the existing text Agent path. Vue captures/plays/subtitles/renders;
Tauri owns cloud HTTP, speech secrets, temporary audio and required new media
assets. VIVY remains sole Agent/run/approval authority. OLVRS/local ONNX and
pet window/neuro-link restoration are not prerequisites.

## Existing files and proposed additions

Reuse `agent-diva-gui/src/features/diva-pet/voice/` provider/player/preprocessing
logic, avatar-runtime-vrm and main chat/settings. Change the frozen native
seam `src/platform/desktop-host.ts`, shell `src-tauri/src/lib.rs` and lifecycle,
plus AST/native/boundary gates. Proposed native module `src-tauri/src/speech/`;
proposed typed facade `src/api/speech.ts`. Do not restore old business crates.

## Inputs to freeze before implementation

- Native command/config DTOs, secure credential store and masked readback.
- Recording MIME and bounded binary IPC/audio resource transport on Windows /
  Linux; exact media asset commands where existing references need them.
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
