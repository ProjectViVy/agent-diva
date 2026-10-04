# v0.3.3 — DN-0S verification

Probe environment: Linux x86_64 VM, rustc 1.97.1, no GTK/WebKitGTK dev stack.

- Lockfiles inspected: `agent-diva-gui/src-tauri/Cargo.toml` + `Cargo.lock`
  (tauri 2.12.1, reqwest 0.13.5 transitive w/o TLS backend), `vivy-bridge`
  manifest (libloading 0.8), `desktop-host.ts`, `minimax-provider.ts` read.
- Disposable crate `~/staging/dn-0s/probe` (keyring 4.2 + reqwest 0.13 rustls):
  `cargo test` — 3/3 pass: `ipc_metadata_size_bound`,
  `keyring_platform_store_roundtrip` (prints `NoDefaultStore` under both the
  default D-Bus store and `linux-keyutils-keyring-store`),
  `reqwest_https_tls_get` (GET api.siliconflow.cn/v1/models → HTTP 401).
- Disposable crate `~/staging/dn-0s/probe-tauri` (tauri 2.12.1 + keyring
  windows-native-keyring-store): `cargo check --target
  x86_64-pc-windows-msvc` — pass; `ipc_probe` module typechecks
  `InvokeBody::Raw`/`InvokeResponseBody::Raw`/`Response::new`.
- Source-pinned read: `tauri-2.12.1/src/ipc/mod.rs` — Raw body/headers reach
  commands via `Request` + `CommandArg`; `Response::new(Vec<u8>)` yields Raw
  bytes; Android Raw unsupported (desktop-only, fine).
- Web-engine audio probe (`~/staging/dn-0s/audio/probe-page.html` in Chrome
  137 via localhost): WAV PCM16 decode ok; MP3 decode ok; OfflineAudioContext
  44.1k→16k mono resample ok; hand-built PCM16 WAV header verified;
  MediaRecorder support = webm/opus + mp4 only.
- Provider doc refresh (2026-10-03): SiliconFlow create-speech,
  audio-transcriptions (multipart, ≤50MB/≤1h, models SenseVoiceSmall |
  TeleSpeechASR, `{text}` + `{code,message,data}` errors), uploads/audio/voice
  (`speech:<name>:<id>` reusable uri), audio/voice/list; MiniMax t2a_v2 (hex
  audio, `base_resp.status_code==0`, `data.status==2`, backup host).
- `python3 -m json.tool docs/plans/diva-next/fixtures/closure-speech.json` — valid.
