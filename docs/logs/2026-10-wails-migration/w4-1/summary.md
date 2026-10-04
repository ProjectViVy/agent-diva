# W4-1 Summary — Go speech/credentials/assets + binary media route

## Scope
Port `src-tauri/crates/diva-speech` (Rust) to native Go `internal/speech`, wire
it into the Wails desktop `DesktopDispatch` channel and the in-process media
mux, replacing the Tauri command surface the Vue frontend already calls.

## Delivered
- `internal/speech/` (8 files, ~2.4k LOC):
  - `errors.go` — `diva.speech/v1` typed errors, full code table preserved.
  - `wav.go` — RIFF/PCM16/mono/16 kHz/≤120 s/≤8 MiB validator (behavioral
    vectors ported from Rust tests).
  - `config.go` — `speech.json` schema v1 store, revision-CAS `Update`
    (`revision_conflict`), strict DTOs, atomic 0600 write, deep-copy
    readback (Rust `.clone()` semantics; shallow copy bug caught by test).
  - `credentials.go` — `SecretStore` seam + `OsKeyring`
    (zalando/go-keyring v0.2.8, W0-pinned), service
    `dev.projectivy.diva.speech`, slots `v1.<provider>`, presence-only
    readback, no plaintext fallback.
  - `assets.go` — voice-asset store: `va-<16 hex>` digest ids, sniffed
    mime, 10 MiB/file · 20 files · 100 MiB caps, orphan reconcile both
    directions, lease-gated reads, delete-pending under lease.
  - `providers.go` — SiliconFlow STT/TTS + MiniMax t2a_v2 HTTP clients:
    verified endpoints (https or test loopback only), 10 s dial / 120 s
    total, no redirects, capped body reads, MP3 magic check, redacted
    errors.
  - `registry.go` — request registry: `{session_id, generation}` context,
    one-STT + one-TTS slots, admit/cancel/settle, late-settle → not
    deliverable (`stale_context` on return), `BeginShutdown` abort-all,
    done-channel tracking for drain.
  - `service.go` — `Service` facade: identity validation
    (`[a-zA-Z0-9._:-]`, bounded), admission → prepare (config snapshot +
    credential + asset lease) → async execute with ctx cancellation,
    pre-settle test hook, `speech:diagnostic` events per phase, graceful
    `Shutdown(grace)` with honest `ShutdownReport`.
- `internal/desktop/media_http.go` — 4 binary routes on the W0-proven
  internal mux: `POST /media/speech/transcribe` (raw WAV + 2 KiB
  `x-diva-speech-meta`), `POST /media/speech/synthesize` (JSON → MP3),
  `POST /media/voice-assets` (raw bytes + `x-diva-asset-meta`),
  `GET /media/voice-assets/{id}`. Capability grant checked BEFORE body
  reads; `MaxBytesReader` caps; typed `SpeechError` JSON bodies with
  mapped HTTP statuses (400/404/409/499/502/503).
- `internal/desktop/speech_dispatch.go` — dispatch table for the 8
  preserved nativeCall commands (`speech_config_*`, `speech_credential_*`,
  `speech_context_set`, `speech_cancel`, `voice_asset_*`); credentials
  reported as presence booleans only.
- Wiring: `app.go` opens `OpenService(<configDir>/speech)` at build,
  emits `speech:diagnostic`, installs dispatch + `onShutdown` drain (2 s);
  `lifecycle.go` hide hook invalidates the speech context;
  `runtime_service.go` maps `SpeechError` → `hostv1.Error`
  (`not_ready`/`invalid_input`/`cancelled`/`timeout`, code −32090, full
  typed error in `Data`).

## Frontend seam
`speech.ts`/`voice-api.ts`/controller untouched — same command names,
payloads, media routes and event channel, all satisfied by the Go side.
`pnpm` suite 551/551, `vue-tsc` clean.

## Deferred / residuals
- Real-provider evidence (SiliconFlow STT, SiliconFlow + MiniMax TTS)
  pending operator-held credentials — recorded as pending rows.
- Windows leg (Credential Manager + lifecycle) pending W0 Windows probe.
- Rust `diva-speech` crate + C ABI removal is W6 scope.
- Slow/oversize/redirect provider behaviours covered by local scripted
  httptest fixtures; device-unavailable row untestable on headless Linux.
