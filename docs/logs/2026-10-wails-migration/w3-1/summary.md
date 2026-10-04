# W3-1 summary — Wails desktop host + frontend seam

Delivered on `feat/wails-go-host` (human commits, unpushed).

## Go host (internal/desktop, cmd/diva)

- `internal/desktop/app.go` — Compose: absolute config path → `hostv1.Open`
  (partial-failure unwind closes the host) → `application.New` with
  `Services:[RuntimeService]`, `Assets{Handler:NewMediaMux, Middleware:senderGate}`,
  `SingleInstance{UniqueID:"foundation.undefine.diva", OnSecondInstanceLaunch→reopen}`,
  `ShouldQuit→true`, `OnShutdown→closeAdmission`, `PostShutdown→truthful teardown log`.
  Main window "diva-main" 1280×800, `vivy:window` hidden/shown events, system tray.
- `internal/desktop/runtime_service.go` — `VivyCall` (CallReply envelope
  `{ok,result,error}` preserving kind/code/data), one Go pump: blocking
  `host.Next` → emits `vivy:event` wire events (`kind:vivy|bridge`,
  `status:gap|lost`); `ServiceShutdown` stops pump + `host.Close(5s budget)`
  + records teardown error; `MediaToken` issues capability only to the bound
  main window; `DesktopDispatch` generic native-command channel gated on the
  bound window (handlers land in W4; unknown → `not_ready`, never a noop).
- `internal/desktop/sender.go` + `media.go` — W3-3 contract: origin allowlist
  + natively-overwritten window-id header gate; in-process mux (no TCP
  listener); 10MiB-bounded raw WAV POST/GET, token+window+revocation checks.
- `internal/desktop/config.go` — `DIVA_VIVY_CONFIG` env > flag > seeded
  `os.UserConfigDir()/DIVA/vivy.yaml` (0600), absolute-path enforced.
- `internal/desktop/lifecycle.go` — close-to-hide via synchronous
  `WindowClosing` hook (`Cancel+Hide`), `reopen` Show+Focus+emit.
- `agent-diva-gui/assets.go` — `//go:embed all:dist` + default vivy config
  seed; `cmd/diva/main.go` entrypoint.
- `deps/` symlinks keep dev replaces inside the host tree (pack contract:
  host replaces must not escape the staged host tree; pack rewrites the
  whole VIVY closure itself).

## Frontend seam (agent-diva-gui)

- `src/generated/wails/` committed TS bindings (`wails3 generate bindings
  -ts -f '-tags=gtk3,vivy_headless'`): `RuntimeService.VivyCall /
  DesktopDispatch / MediaToken`.
- `src/platform/desktop-host.ts` rewritten — `vivyCall` unwraps CallReply,
  `onVivyEvent` = one `Events.On('vivy:event')`, `nativeCall`/`nativeListen`
  generic channel, `mediaFetch` raw-binary route (token header, natively
  injected window id), speech/voice facade with identical command names.
- `isDesktopShell` detects the injected runtime (`_wails.environment`), not
  the importable library — browser/test mode stays browser mode.
- `createWailsTransport` replaces `createTauriTransport` (same contract:
  one listener-install promise, close detaches listeners only).
- All Tauri imports removed from `src/`: invoke→nativeCall, listen→
  nativeListen, emitTo→`Events.Emit`, getCurrentWindow→`thisWindow`,
  plugin-opener→`Browser.OpenURL`, `__TAURI_INTERNALS__`→`_wails.environment`.
  Pet overlay stays deferred (DN-W3-5); `desktop_pet_start_drag` is a
  not_ready stub until pet expansion.
- `@wailsio/runtime 3.0.0-beta.27` pinned in package.json/pnpm-lock.

## Packaging

- `scripts/build-desktop.py`: lock emits `buildTags:["gtk3"]` on Linux
  (sealed into hostBuild.tools.buildTags); mode test appends them;
  mode build restores `dist/.gitkeep` after vite empties dist.
- VIVY SDK (`agent-vivy d9dfdf49`): go-host lock accepts optional
  `buildTags`, validated + joined into the sealed `-tags` argument.
- `justfile`: `desktop-bindings`, `desktop-dev`, `desktop-test`,
  `desktop-bindings-build` recipes.
