# W0-4 verification — commands, captures, exact errors

## 1. F6 root cause (WebView2 CDP)

`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` is **not**
honored by the wails3 loader path (webview process cmdline lacks the flag —
wails `chromium.go` passes only `application.Options.Windows.AdditionalBrowserArgs`).
CDP access was obtained via a temporary uncommitted patch in
`internal/desktop/app.go`:

```go
Windows: application.WindowsOptions{
    AdditionalBrowserArgs: []string{"--remote-debugging-port=9222"},
},
```

then `http://localhost:9222/json` + playwright `connect_over_cdp`
(`w03-evidence/cdp_probe.py`, capture in `cdp-capture.log`).

### Console at app start (exact capture)

```
08:35:51 [resp4xx] 404 http://wails.localhost/wails/custom.js
08:35:51 [console.error] Failed to load resource: the server responded with a status of 404 (Not Found)
08:35:51 [console.warning] [pet-config] Failed to load core config: dc
(w0-3 run) [console.log] Running in browser mode - Tauri listeners skipped
```

`window._wails.environment` evaluated seconds later:
`{"OS":"windows","Arch":"amd64","Debug":true}` — present **after** injection.

### Mechanism (exact root cause)

Wails3 injects `window._wails.environment` via `w.execJS(runtime.Core(...))`
inside `webview_window_windows.go` on WebView **NavigationCompleted**
(~line 2767) — i.e. it can land **after** the page has loaded and mounted.
`App.vue:onMounted` sampled `isTauri()` (`window._wails?.environment != null`)
exactly once at mount, got `undefined`, logged
`Running in browser mode - Tauri listeners skipped` and `return`ed before
`vivyChat.connect()`. Consequences observed in w0-3: status pill stuck
"连接中", `currentSessionId` never bound (sends created orphan sessions),
persisted messages never rendered. It was **not** an `Events.On('vivy:event')`
rejection and **not** a VivyCall failure — connect was simply never invoked.
(All connect-path RPCs were proven green in w0-3 via the embedded probe.)

The injected runtime core also dispatches a `wails:runtime-config-ready`
window event (microtask after `environment` is set —
`internal/runtime/runtime.go`). Fix (`dda9d7da`, `agent-diva-gui/src/App.vue`):
when `isTauri()` is false at mount, wait for `wails:runtime-config-ready`
(bounded 3s, with a post-subscribe re-check) before declaring browser mode.

### Post-fix verification (same CDP probe + `cdp_bootcheck.py`)

Fresh boot of the fixed sealed exe, evaluated after ~15s
(`cdp-bootcheck.log`):

```
has_online_text: true        # connect() reached ready
has_connecting_text: false   # no residue of the stuck state
msg_bubble_count: 7          # persisted history rendered on first paint
visible msg texts: ["write a note called probe-final.txt", ..., "done", ...]
```

No console errors, no pageerrors, no failed binding calls across the boot —
the only noise remains the benign `wails/custom.js` 404 and `[pet-config]`
warning.

## 2. F5 verify (lazy frozen-core capture)

Clean sealed rebuild (`build-desktop.py --mode build --development`, output
`artifacts/diva-go-host-r4`, then driven with the mock provider env
`DEEPSEEK_API_KEY=mock-key VIVY_API_BASE=http://127.0.0.1:18321 VIVY_PROVIDER=deepseek`):

- GUI send on a fresh session → `turn/start` no longer fails; the turn
  reached the model call.
- `garden.db` `frozen_core_sessions`: new row
  `sess_4d13a567743cd881`, `captured_at 2026-10-04T08:36:22.0376255Z`
  (lazy capture on first Prepare — the w0-3 failure's exact missing write).
- `vivy.db` `messages`: 3 user/assistant pairs persisted;
  `runs`: `run_a81e8bf6124cdbf5`, `run_8cfc3d4b5b736b38`,
  `run_ae53700a2413bc42` — all `completed`.
- Screenshots: `first-turn-rendered.png`, `reload-history-restored.png`,
  `r4-restart-restore.png`.

## 3. Event-loss lane

- F5 webview reload mid-session: `vivy:event` subscription dropped with the
  page; `session/get` resync restored the identical 4-message history —
  zero duplicate turns, zero fabricated deltas (`reload-history-restored.png`).
- Process kill + relaunch (lease TTL wait): latest session restored on boot
  with no re-enqueued turn (`r4-restart-restore.png`).
- Queue-overflow / active-call-timeout lanes stay on the Go-level w4-1
  registry/`Shutdown(grace)` evidence.

## Artifact (r4 candidate)

`artifacts/diva-go-host-r4/diva.exe`
sha256 `eb024b6e1eac2c51ef72de036a76534eff822ed267b21378684d5ceb5499ebfd`
generationId `6b11b6ea7b3f22984f8c47830cdc03dc05da0c1e39d7468cd981eb12e3325089`
pins: host `dda9d7da` · vivy `e3b60280` · laputa `6f2eed2` · go1.26.8 ·
wails v3.0.0-beta.27 · tags `vivy_headless` · WebView2 154.0.4258.53
