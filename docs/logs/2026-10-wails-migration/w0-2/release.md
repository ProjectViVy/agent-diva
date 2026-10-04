# W0-2 Release notes — Windows x64 leg

## What this leg released

- A Windows-built sealed go-host artifact:
  `repos/artifacts/diva-go-host/diva.exe`
  (106,653,696 B, sha256 `623bfc05…`, generation `a24a62a2…`, development
  mode, release=false). The artifact directory ships `consumer.mod/sum`,
  `checksums.sha256`, `generation.json`, `zz_assembly.go`,
  `ui-assembly.ts`, `frontend/` payload and `build-report.json`.
- Windows evidence for W0 Task 5 (keyring) — fully green.
- Windows evidence for W0 Task 4 lifetime rows — proven except the gaps
  below.
- Repo-local changes carried for this leg (uncommitted at build time,
  committed alongside this log):
  - `scripts/build-desktop.py` — strip synthetic exec bits from Windows
    `os.lstat` so `source_tree_hash` matches the SDK digest.
  - `tools/speech-credprobe/` — standalone probe for the production
    credential namespace (`-op set|delete|cycle`).

## Blockers for a one-click Windows release

1. **F1 — frontend payload not sealed.** The Vue dist is gitignored, so
   the sealed binary embeds only `dist/.gitkeep`; the app shows a raw file
   listing. The packaged `frontend/` tree is never served. Fix direction:
   teach the host to serve the staged `frontend/` payload (e.g. resolve
   `<exe dir>/frontend` in `divagui.Frontend()` when the embedded tree is
   empty) or inject built dist into the staged host tree before compile.
2. **F2 — reopen path missing.** Attach the tray to the main window
   (`tray.AttachWindow(w)` gives Wails' ToggleWindow default) and/or move
   the single-instance check before `hostv1.Open` so
   `OnSecondInstanceLaunch` can fire `d.reopen()`.
3. **F3 — quit affordance missing.** Add a tray menu with Reopen/Quit
   items (`tray.SetMenu`) or a UI quit control; graceful teardown already
   works end-to-end once `a.Quit()` is invoked.
4. **F4 — no audio endpoints on the build VM.** Mic capture and playback
   verification need a machine/VM with an audio device (and operator
   credentials for real-provider rows).
5. **Environment caveat — deps links:** Windows checkouts must use real
   directory symlinks (`mklink /D`), not junctions — `mklink /J` breaks
   `hashSourceTree`. Only elevated shells can create them; consider
   documenting this or adding a setup step.

## Compatibility notes

- govector stays at v0.1.8 — upstream chose the vendored-renameio fix
  (vivy `third_party/renameio` + `replace`), so the earlier local
  v0.1.10 bump was reverted and is not part of this leg.
- `taskkill` without `/F` is safe-by-design on this host: it posts
  WM_CLOSE, which the close-to-hide hook cancels — runtime survives.
  `/F` is the only external way to kill it; the lease then gates
  relaunch for up to its 30 s TTL.
