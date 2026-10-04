# W0-4 release notes

## Artifact under test

`artifacts/diva-go-host-r4/diva.exe` (sealed, development mode)

- sha256 `eb024b6e1eac2c51ef72de036a76534eff822ed267b21378684d5ceb5499ebfd`
- generationId `6b11b6ea7b3f22984f8c47830cdc03dc05da0c1e39d7468cd981eb12e3325089`
- pins: agent-diva `dda9d7da` · agent-vivy `e3b60280` · laputa `6f2eed2`
- toolchain: go1.26.8, wails3 v3.0.0-beta.27, node 24.0.1, WebView2 154.0.4258.53
- build tags: `vivy_headless`, CGO off

## Changes carried by this leg

- agent-diva `dda9d7da` `fix(gui): wait for wails:runtime-config-ready before
  browser-mode gate` — resolves F6 (mount-time `_wails.environment` race).
- Consumes vivy `e3b60280` `fix(diva-cognitive): lazily capture frozen core on
  first Prepare` — resolves F5.

## Notes for the pack path

- `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` is inert under the wails3 loader;
  remote debugging requires `WindowsOptions.AdditionalBrowserArgs` in host
  code (or upstream wails support for the env var). If devtools-in-sealed-app
  is a wanted diagnostic knob, wire a build-tag-gated option — do not ship
  the flag unconditionally.
- `wails/custom.js` 404 on every boot is benign (wails dev-runtime shim
  absent in sealed mode); the `[pet-config] Failed to load core config: dc`
  warning predates this work and is unrelated to F5/F6.

## Known limits (unchanged)

- No audio endpoints on this VM → VOICE-REAL untested on Windows.
- CHAT-MATRIX image/approval/edit/regenerate/rewind lanes untested.
