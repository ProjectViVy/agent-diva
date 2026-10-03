# v0.3.3 — DN-0S speech seam probes

Delivered `docs/plans/diva-next/fixtures/closure-speech.json` and amended ledger
C2-4 with the proven dependency pins.

Proven: `tauri 2.12.1` IPC Raw/Response semantics (pinned source + Windows-target
`cargo check`), WebView audio pipeline in Chromium 137 (WAV PCM16 decode, MP3
decode, OfflineAudioContext 16k/mono resample, WAV re-encode; MediaRecorder only
emits webm/opus or mp4 — the contract WAV is synthesized in-engine),
`keyring 4.2.0` v1 API surface and `NoDefaultStore`→`credential_unavailable`
mapping, `reqwest 0.13` `rustls`+`webpki-roots` minimal TLS (live HTTPS GET to
api.siliconflow.cn), refreshed SiliconFlow STT/TTS + MiniMax t2a_v2 offline
request/error fixture shapes.

Pending (recorded, not patched): live IPC roundtrip in bundled WebView, Windows
WebView2 audio, Credential Manager roundtrip, MSVC+cmake requirement for
aws-lc-sys, authenticated provider calls — all reserved for DN-6 / owner
acceptance.
