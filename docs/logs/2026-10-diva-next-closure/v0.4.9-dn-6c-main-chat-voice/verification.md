# v0.4.9 verification (DN-6C)

- `pnpm vitest run`: 63 files / 530 tests green (new: 6 controller, 4
  encodeWav, speech api browser-rejection, run-messages runId).
- vue-tsc --noEmit: clean.
- `just gui-test` + `just gui-build`: green.
- `node scripts/ci/check_legacy_frontend_calls.mjs --selftest`: 8/8
  fixtures fire; live scan clean — only desktop-host.ts/openExternal.ts
  invoke; NATIVE_CMDS covers all 13 speech commands.
- Zero pet-path imports in new voice code.
- Live STT/TTS round-trip not exercisable on this VM (no Tauri
  WebView + no provider keys); delegated to developer-native smoke /
  owner acceptance.
