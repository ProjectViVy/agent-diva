# wails-probe — DN-W3 W0 native feasibility probe

Pinned contract probe for `github.com/wailsapp/wails/v3 v3.0.0-beta.27`.
Separate from `cmd/diva`; nothing here ships.

## Pins

| Piece | Pin |
|---|---|
| Go module | `github.com/wailsapp/wails/v3 v3.0.0-beta.27` |
| CLI | `wails3 v3.0.0-beta.27` (`go install github.com/wailsapp/wails/v3/cmd/wails3@v3.0.0-beta.27`) |
| npm runtime | `@wailsio/runtime 3.0.0-beta.27` (verified in tagged source `internal/runtime/desktop/@wailsio/runtime/package.json` — NOT guessed from the Go tag) |
| keyring | `github.com/zalando/go-keyring v0.2.8` |

## Run

```sh
# Linux needs the gtk3 backend (beta.27's GTK4 path needs GTK >= 4.10;
# Ubuntu 22.04 ships 4.6.9). Windows/macOS build untagged.
go test -tags gtk3 ./tools/wails-probe -count=1   # unit proofs
go run  -tags gtk3 ./tools/wails-probe            # headless API report
go run  -tags gtk3 ./tools/wails-probe --keyring  # OS keyring probe
go run  -tags gtk3 ./tools/wails-probe --gui      # window/tray/close-to-hide (needs a display)

wails3 generate bindings -f '-tags gtk3' -d frontend/bindings .
```

## Sender contract (W3-2/W3-3 identity)

Bound methods receive the calling window natively:

```go
func (s *ProbeService) Identify(ctx context.Context) (map[string]any, error) {
    w, ok := ctx.Value(application.WindowKey).(application.Window)
    if !ok { return nil, errors.New("no native window in binding context") }
    return map[string]any{"id": w.ID(), "name": w.Name()}, nil
}
```

`x-wails-window-id` / `x-wails-window-name` are **overwritten per-request** in
`webViewAssetRequest.Header()` (application.go ~L385): a forged client header
cannot win on the desktop webview path. The media capability is issued to
`{windowID, token}` and every request must match both and not be revoked.

External pages: beta.27 has **no navigation veto**. Defense = `AssetOptions.
Middleware` origin allowlist (`""`, `http://wails.local`, `wails://wails`,
`http://wails.localhost`) + the window-id gate. A navigated-away page carries
a real window id but is denied at the origin layer.

## Media route (W3-3)

`AssetOptions.Handler` mux inside the internal wails scheme — **no TCP
listener**: `POST /media/wav` (`http.MaxBytesReader` 10 MiB, RIFF/WAVE magic,
capability check) → `{id,bytes,sha256}`; `GET /media/wav/{id}` → `audio/wav`
raw bytes, honouring `r.Context()` cancellation. Bytes are never
JSON/base64-expanded; measured live-heap delta for a 10 MiB upload ≈ 22 MiB
(~2.2× payload, request+store copies).

## Lifetime (proven API surface; runtime exercise = Windows leg)

`Options.OnShutdown` → `ServiceShutdown` (reverse registration) →
`Options.PostShutdown`. Close-to-hide: `w.RegisterHook(events.Common.
WindowClosing, e.Cancel()+w.Hide())` — hooks run synchronously before the
internal destroy listener. Single instance: `SingleInstanceOptions{UniqueID,
OnSecondInstanceLaunch}`.

## Keyring (W0 Task 5)

`zalando/go-keyring v0.2.8`: Windows Credential Manager / macOS Keychain /
Secret Service. Backend failure is an error — **no plaintext fallback**.
