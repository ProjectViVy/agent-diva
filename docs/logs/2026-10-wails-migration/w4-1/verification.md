# W4-1 Verification

## Commands & results
- `go test -race -tags 'gtk3 vivy_headless' ./internal/speech` — 16/16 pass
  (wav vectors, config CAS + validation, credentials, asset store
  caps/leases/reconcile, transcribe settle/no-speech/no-credential,
  admission guards, cancel-during-call, late-success-discard, both
  providers, provider error mapping, shutdown drain, single-flight).
- `go test -race -tags 'gtk3 vivy_headless' ./internal/desktop` — all pass
  incl. 4 new media route tests: import→read round-trip, forged-token 403
  before body read, malformed/traversal ids rejected with typed body,
  meta bounds (missing/oversize), not_configured typed error.
- `go build -tags 'gtk3 vivy_headless' ./...` + `go vet` — clean.
- `python3 scripts/build-desktop.py --mode test` — sealed pack + staged
  consumer build + `go vet` + race tests under consumer.mod: PASS.
- `python3 scripts/build-desktop.py --mode build --development` — sealed
  ELF artifact produced.
- xvfb smoke `./diva` — `vivy host opened`, AssetServer middleware+handler
  up, systray degraded, speech service opened (no error path).
- `pnpm --dir agent-diva-gui test` — 551/551 (one transient cross-file
  timer flake in first run; clean on rerun).
- `pnpm exec vue-tsc --noEmit` — clean.

## Notable test-caught fix
`ConfigStore.Preferences()` returned a shallow copy; mutating nested
TTS fields leaked into the stored config — Rust `.clone()` was deep.
Fixed with a deep copy; covered by `TestConfigStoreCAS`.

## Fixture findings
httptest `Server.Close` blocks on active conns: deferred `close(release)`
must precede `srv.Close` (LIFO). Applied to the three slow-handler tests.
Client-side ctx cancel aborts the provider HTTP request (verified by
`TestCancelDuringProviderCall`).

## Pending rows
- Real provider calls (operator credentials required).
- Windows Credential Manager + lifetime leg (W0 Windows probe).
- `device_unavailable` path (no audio device concept on headless Linux CI).
