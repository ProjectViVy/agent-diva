# Verification — v0.4.13 DN-M-C

- `node scripts/ci/check_legacy_frontend_calls.mjs --selftest` → 9/9
  fixtures fired (new `seam-unlisted-command.ts` exercises the
  inside-seam deny branch).
- `python3 scripts/ci/check_vivy_backend_boundary.py` → clean.
- `pnpm exec vitest run` → 67 files / 551 tests.
- `cargo test -p vivy-bridge -p diva-speech` → 25 tests.
- `cargo audit` 0.22.2 on both lockfiles → 0 vulns, 2 warnings recorded.
- `pnpm audit --prod` → 8 advisories recorded with tool/feed noted.
- Not run (environment): whole-workspace cargo check/clippy, live Tauri
  window, Windows x64, real provider credentials.
