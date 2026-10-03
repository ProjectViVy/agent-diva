# v0.4.8 verification — DN-6B

Repo: agent-diva `feat/dn-closure-wave1`. rustc 1.97, cargo fmt/clippy.

## Plan-mandated checks

- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml -p diva-speech`
  — **18/18 pass** (7 unit + 11 contract in `tests/speech_contract.rs`).
  Named ordered tests all present and green: stale_context_cannot_admit,
  cancel_before_http, late_completion_discarded, oversize_body_bounded,
  minimax_business_error, quit_with_inflight.
- `cargo clippy -p diva-speech --all-targets` — **0 warnings**.
- `cargo fmt -p diva-speech --check` — clean. Shell files (`src/lib.rs`,
  `src/speech/{mod,commands}.rs`) formatted to match rustfmt for the
  edited hunks only; pre-existing baseline fmt diffs elsewhere in
  lib.rs/lifecycle.rs left untouched.
- `node scripts/ci/check_legacy_frontend_calls.mjs --selftest` —
  **8/8 fixtures fired** (incl. new speech-browser-fetch + pet-restore
  negatives); `legacy frontend calls gate clean`.
- `python3 scripts/ci/check_vivy_backend_boundary.py` —
  **boundary gate clean**, invoke handlers = native allowlist
  (13 commands).
- `just shell-bridge-test` — vivy-bridge 7/7 pass.

## Named-test semantics (fixtures, not mocks-of-self)

- `cancel_before_http`: STT aimed at a blackhole listener; cancel fires
  before connect completes → `cancelled`, second cancel → `settled`,
  `active_count`==0, diagnostic phase `cancelled`.
- `late_completion_discarded`: `pre_settle_hook` advances the context
  generation mid-flight → success result is discarded as
  `stale_context` (1 real HTTP hit recorded, diagnostic `discarded`).
- `oversize_body_bounded`: fixture streams 17MiB → `provider_error`
  ("cap") before the full body is read.
- `minimax_business_error`: HTTP 200 with `status_code:1004` →
  `provider_error` carrying `http_status:200`.
- `quit_with_inflight`: shutdown(5s) during a gated request reports
  `inflight:1, remaining:0`; post-quit admit → `cancelled`.
- `stale_context_cannot_admit`: 0 HTTP hits observed.

## Honest limits (recorded, not claimed)

- `just shell-test` / `just shell-clippy` (whole src-tauri workspace)
  **cannot run on this VM** — tauri links GTK/WebKitGTK and no
  pkg-config/glib exists here. The shell-side files are reviewed +
  rustfmt-clean but compile-unverified; `try_state::<Arc<SpeechState>>()`
  on AppHandle and the four new command signatures get their first real
  compile under DN-8C / the Windows acceptance host.
- No live SiliconFlow/MiniMax credentials — wire shapes are exercised
  against local HTTP fixtures that record and assert the exact request
  bytes (multipart fields, Bearer header, JSON bodies, hex envelope).
- reqwest `webpki-roots` feature does not exist in 0.13; `rustls`
  already includes platform-verifier + webpki root certs.
