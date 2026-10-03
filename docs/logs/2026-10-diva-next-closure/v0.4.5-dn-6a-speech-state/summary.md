# v0.4.5 — DN-6A speech state (preferences / credentials / assets)

## Scope
Native speech ownership per C2-4: `diva.speech/v1` preferences with
revision CAS, OS-store credentials (SiliconFlow / MiniMax), and a bounded
voice-asset store — no request commands yet (DN-6B), no UI yet (DN-6C).

## Changes
- New tauri-free crate `agent-diva-gui/src-tauri/crates/diva-speech`:
  - `lib.rs` — `SpeechCode` / `SpeechError` (secrets never in messages).
  - `config.rs` — strict DTOs, `validate_base_url`, provider-block and
    reference validation, atomic write + revision CAS.
  - `credentials.rs` — fixed `dev.projectivy.diva.speech` service,
    `v1.<provider>` slots, `SecretStore` injection, `NoDefaultStore` →
    `credential_unavailable`, presence-only readback, no plaintext/env
    fallback.
  - `assets.rs` — 10MiB/20/100MiB bounds, `va-<16 hex>` ids, WAV/MP3
    signature sniff, atomic manifest reconciled both directions,
    `AssetLease` deferred delete.
  - `tests.rs` — 7 tests incl. all plan-named cases.
- Shell glue `src/speech/{mod.rs,commands.rs}` + `lib.rs`: `SpeechState`
  opened at setup (`DIVA_SPEECH_DIR` or `<app_config_dir>/speech`),
  8 commands registered, main-window-only gate, raw-body import and
  raw-bytes read per contract.
- Boundary gate: handler check now an explicit native allowlist
  (vivy_call + 8 speech commands).

## Out of scope (per plan)
- `speech_transcribe/synthesize/cancel/context_set` → DN-6B.
- Settings UI / frontend caller → DN-6C.
- Full shell compile (`just shell-clippy`) → DN-8C native acceptance
  (this VM has no webkit dev libs; all logic verified in-crate).
