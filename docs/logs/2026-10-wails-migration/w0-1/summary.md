# w0-1 — Wails native feasibility probe (Linux leg)

## What changed

- Created root `go.mod` (`module github.com/ProjectViVy/agent-diva`, `go 1.26.4`) with pinned
  `github.com/wailsapp/wails/v3 v3.0.0-beta.27` and `github.com/zalando/go-keyring v0.2.8`.
- New `tools/wails-probe/` Go probe (separate from `cmd/diva`):
  - `main.go` — headless API report + `--gui` app wiring full Options surface
    (services, assets+middleware, RawMessageHandler origin log, SingleInstance,
    OnShutdown/PostShutdown, window `diva-main`, tray, `probe:tick` event,
    close-to-hide via `RegisterHook(events.Common.WindowClosing, cancel+Hide)`).
  - `service.go` — `ProbeService` (`ServiceStartup`/`ServiceShutdown`), bound
    methods `Identify`/`CallerIsMain` reading `ctx.Value(application.WindowKey)`.
  - `sender.go` — origin allowlist + native window-id middleware gate.
  - `media.go` — internal-scheme `POST/GET /media/wav` route, 10 MiB bound,
    RIFF/WAVE check, capability {windowID+token+not revoked}, raw bytes.
  - `keyring.go` — zalando/go-keyring set/read/delete probe, no fallback.
  - `frontend/` + generated bindings (`wails3 generate bindings`).
- Fixture `docs/plans/diva-next/fixtures/wails-native-probe.json` records API
  surface, sender contract, media adapter signature, keyring choice, generated
  binding hashes, and the pending Windows x64 rows.

## Key decisions

- **gtk3 build tag on Linux**: beta.27's default GTK4 backend needs
  `GtkFileDialog` (GTK ≥ 4.10); Ubuntu 22.04 ships 4.6.9, so Linux leg compiles
  with `-tags gtk3` (GTK 3.24.33 + WebKit2GTK 4.1). Windows/macOS unaffected.
- **No navigation veto in beta.27**: external-page defense is the AssetServer
  middleware origin allowlist + window-id gate. Documented as the W3 seam.
- **Sender identity is native**: `x-wails-window-id` is overwritten per-request
  by `webViewAssetRequest.Header()`; bound-method ctx carries the real Window.

## Impact

- W0 Linux rows proven; W3 can be authored against recorded signatures.
- W0 GO/NO-GO stays PENDING until Windows x64 rows (lifetime during active
  call, WebView2, mic/audio, Credential Manager) pass — per the W0 exit gate.
