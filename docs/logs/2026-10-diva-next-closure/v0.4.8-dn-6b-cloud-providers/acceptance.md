# v0.4.8 acceptance — DN-6B

## Claims supported by evidence on this VM

- Context/admission registry with generations, slot leases, per-request
  cancellation, and honest slot return on every terminal path — proven
  by 11 contract tests against real loopback HTTP fixtures.
- SiliconFlow STT multipart, SiliconFlow TTS JSON voice union, MiniMax
  strict business-status + bounded hex decode — request bytes asserted
  by fixtures; response bodies capped while streaming.
- Raw WAV validation rejects non-PCM16-mono-16kHz before any upload.
- `speech:diagnostic` emits only safe fields, main-window only.
- Native command set + AST allowlists updated together; negative
  fixtures reject browser HTTP and restored pet_* invokes.

## Not proven here (owner acceptance pending)

- Compile of the Tauri shell wrapper (`src/speech/*`, `src/lib.rs`
  wiring) — deferred to DN-8C / Windows host.
- Authenticated provider roundtrips — deferred (no keys).
- End-to-end WAV→transcript and text→MP3 playback inside the real app —
  DN-6C owns the frontend wiring; DN-8C owns E2E.

Owner acceptance remains the gate; green tests ≠ acceptance.
