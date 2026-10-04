# w0-1 — Verification

Commands run on `linux/amd64`, go1.26.4, wails v3.0.0-beta.27 (`-tags gtk3`).

## Compile + unit tests

```
go build -tags gtk3 ./tools/wails-probe      # OK
go test -tags gtk3 ./tools/wails-probe -count=1 -v
```

Result: 12 PASS, 1 SKIP, 0 FAIL.

| Test | Result |
|---|---|
| TestSenderGateDeniesForeignOrigin (bundled, wails.local, wails://wails OK; https, data: denied) | PASS |
| TestSenderGateDeniesForeignAndMissingWindows (foreign/missing/forged window header -> 403) | PASS |
| TestMediaCapabilityTokenWindowAndRevocation (missing/wrong token, wrong window, grant, revoke) | PASS |
| TestMediaRouteWAVRoundTripRawBytes (upload->download byte-identical, sha256 match) | PASS |
| TestMediaRouteRejectsInvalidWAV (non-RIFF -> 400) | PASS |
| TestMediaRouteRejectsOverBound (>10 MiB -> 413) | PASS |
| TestMediaRouteAtBound (exactly 10 MiB -> 200) | PASS |
| TestMediaRouteDeniedCapability (bad token -> 403) | PASS |
| TestMediaRoutePeakMemoryAtBound (10 MiB upload: live-heap delta 22.39 MiB ~2.2x, <40 MiB assert) | PASS |
| TestMediaDownloadHonoursCancel (cancelled ctx aborts response) | PASS |
| TestBoundMethodReadsNativeWindowIdentity (ctx WindowKey -> ID, foreign mismatch) | PASS |
| TestKeyringProbeNeverFallsBack (unavailable backend classified, no crash/fallback) | PASS |
| TestKeyringNotFoundIsTyped | SKIP (headless: org.freedesktop.secrets absent — backend unavailable) |

## Headless API report

`go run -tags gtk3 ./tools/wails-probe` — printed the pinned API surface
(application.Options, Window/Tray/Events/Service lifecycle, sender contract,
close-to-hide, media route). Output in fixture `wails-native-probe.json.apiSurface`.

## Keyring probe

`go run -tags gtk3 ./tools/wails-probe --keyring`:

```
keyring set: error: The name org.freedesktop.secrets was not provided by any .service files
keyring available: false
```

Backend absence is surfaced as an error — no plaintext fallback path exists.

## Generated-binding round trip

`wails3 generate bindings -f '-tags gtk3' -d frontend/bindings .`
→ `Processed: 247 Packages, 1 Service, 2 Methods` — `Identify`/`CallerIsMain`
generated with stable `$Call.ByID` ids; hashes recorded in fixture.
`wails3 version` → `v3.0.0-beta.27` (CLI pin = module pin = npm runtime pin).

## NOT verified on Linux leg

- GUI lifetime (`--gui` needs a display): close-to-hide, tray, single-instance
  second-launch focus, shutdown order with in-flight call — Windows x64 leg.
- WebView2 deployment, microphone permission/capture, audio playback — Windows.
- Credential Manager set/read/delete/locked-store — Windows.
