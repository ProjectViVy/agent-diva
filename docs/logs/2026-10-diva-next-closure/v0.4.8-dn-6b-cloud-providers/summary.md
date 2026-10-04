# v0.4.8 — DN-6B bounded cloud speech providers + cancellation

Scope: `docs/plans/diva-next/closure/DN-6B.md`. Builds on DN-6A state
(config/credentials/assets) to land the real SiliconFlow STT +
SiliconFlow/MiniMax TTS providers with admission, cancellation, and
honest diagnostics.

## What changed

`crates/diva-speech` (tauri-free, fully testable):
- `wav.rs` — RIFF/WAVE chunk walk; requires PCM16 mono 16kHz, data ≤120s,
  file ≤8MiB. Every violation → `invalid_audio`.
- `registry.rs` — `SpeechContext{session_id,generation}` (strictly newer
  generation adopts and aborts inflight; identical is idempotent;
  older/conflicting → `stale_context`). Slot leases for STT/TTS
  (`busy`), per-request `CancelToken`, inflight ledger, finished
  JoinHandle reaping, quit flag.
- `providers.rs` — reqwest 0.13 rustls clients. SiliconFlow STT
  multipart `/v1/audio/transcriptions`; SiliconFlow TTS JSON
  `/v1/audio/speech` with the three-state voice union (system voice /
  `speech:<name>:<id>` / inline base64 `references`); MiniMax `t2a_v2`
  with strict `base_resp.status_code==0` + `data.status==2` + bounded
  hex decode + MP3 magic. Bodies are streamed through byte caps; HTTPS
  endpoints only (loopback http allowed behind a test-only flag).
- `service.rs` — `SpeechService`: validate → admit (lease + token) →
  prepare outside the registry lock → `tokio::spawn` racing
  `token.cancelled()` against provider work → pre-settle → settle
  (deliverability check converts late results into `stale_context`) →
  oneshot reply. `shutdown(grace)` drains join handles with a deadline
  and aborts leftovers, reporting `inflight`/`remaining` honestly.

Shell (`src-tauri/src`):
- `speech_context_set`, `speech_transcribe` (InvokeBody::Raw only,
  `x-diva-speech-meta` ≤2KiB, mime `audio/wav`), `speech_synthesize`
  (returns `tauri::ipc::Response` Raw MP3), `speech_cancel` added to
  `generate_handler!` — 13 native commands total.
- `speech:diagnostic` emitted to the main window only, carrying
  phase/kind/provider/safe code/http status/elapsed/bytes/identity/
  config revision — no text, no audio, no secrets.
- Window Hide invalidates the speech context; Quit runs
  `service.shutdown(5s)` before `shell.shutdown()`/`app.exit(0)`.

CI allowlists updated together with the commands:
- `check_vivy_backend_boundary.py` — invoke allowlist now 13 commands.
- `check_legacy_frontend_calls.mjs` — `NATIVE_CMDS` explicit set for
  desktop-host.ts; new negative fixtures `speech-browser-fetch.ts`
  (fetch() to api.siliconflow.cn in renderer) and `pet-restore.ts`
  (restored pet_* command) — 8/8 selftest fixtures fire.

## Ordered-steps coverage

stale admission, cancel-before-HTTP, late-completion discard, oversize
cap, MiniMax business error, quit-with-inflight — all six named tests
exist and pass, plus five more (happy paths ×3, WAV rejection,
context idempotence). Slots/leases return to zero on every terminal
path (asserted via `active_count`).

## Commit

`feat(speech): add bounded cloud media and cancellation` on
`feat/dn-closure-wave1`.
