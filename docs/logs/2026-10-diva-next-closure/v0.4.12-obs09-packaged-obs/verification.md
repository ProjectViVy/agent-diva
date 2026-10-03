# Verification — v0.4.12 OBS-09

- dlopen C driver over the staged `.so` (five exports only, ABI v1):
  session create → turn → approval → respond → cancel → subscribe +
  poll → diagnostics append/read → restart replay. All executed on the
  immutable candidate binary; scripts + journal dumps retained in the
  session scratch (`/tmp/obs09/`).
- `agent-diva-gui`: `pnpm exec vitest run src/state/vivy-chat.test.ts`
  → 32/32 passed.
- `cargo test -p vivy-bridge -p diva-speech` → 25 tests green including
  real-artifact tests on the staged `.so`.
- JSON fixture validated (`python3 -m json.tool`).
- Not run (environment): whole-workspace cargo check/clippy
  (GTK/WebKitGTK), live Tauri window, Windows x64, real provider.
