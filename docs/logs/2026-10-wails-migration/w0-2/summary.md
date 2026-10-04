# W0-2 Summary — Windows x64 leg (Wails migration, W0 Tasks 4/5)

Windows Server 2022 (Build 20348), interactive desktop session, all work on
native Windows (no Linux emulation). Executor: Devin session
`devin-ae03047a0aea482196c02fa73c416630` on behalf of mastwet.

## Source pins (post-repin)

| repo | ref | commit |
|------|-----|--------|
| agent-diva | `feat/wails-go-host` | `81c381f8` |
| agent-vivy | `feat/wails-migration` | `3862b768` |
| laputa | detached | `6f2eed2d71c8` |

Toolchain: Go 1.26.8, Node 24.0.1 + pnpm 12.9.1 (`@pnpm/exe`), wails3
v3.0.0-beta.27, mingw-w64 (choco), WebView2 Runtime **154.0.4258.53**
(installed via evergreen bootstrapper — was absent).

## Sealed candidate (second build, after upstream renameio fix)

`python scripts/build-desktop.py --mode build --development`
→ `repos/artifacts/diva-go-host/`:

- `diva.exe` 106,653,696 bytes, sha256
  `623bfc05cf70997f2163668da2646294de46f5ad8457aa3e8a29b2acc2052800`
- generationId `a24a62a2cd2b816528de07c2b5c19af88ab63f8138d8b97ee9a2e1dbc81bd033`
- build-report: host 81c381f8, vivy 3862b768, laputa 6f2eed2,
  go1.26.8, wails v3.0.0-beta.27, windows/amd64, cgo true, release=false
- (first build at prior pins produced generation `1d626e40…`, sha256
  `f02c69ba…`, 106,665,984 bytes — same code surface, older dependency fix)

## Proven on real Windows

- **W0 Task 5 (Credential Manager)** — go-keyring v0.2.8 on wincred:
  set/get/delete/post-delete probe OK; real slot
  `dev.projectivy.diva.speech` / `v1.siliconflow` visible via `cmdkey /list`
  as a LegacyGeneric credential (local persistence); absent after delete;
  no plaintext copy anywhere under the profile. `internal/speech` tests
  green on Windows (`go test -tags vivy_headless ./internal/speech`).
- **W0 Task 4 (lifetime, partial)** — window opens with WebView2 154.0
  rendering; close-to-hide (X and `taskkill` WM_CLOSE both hide, process
  and runtime survive); tray icon present ("DIVA desktop" tooltip);
  second launch rejected by organism lease (-32086); taskkill /F → relaunch
  blocked by 30 s lease TTL then opens on the same profile; graceful quit
  via CTRL_BREAK → `Quitting application...` → `speech teardown:
  inflight=0 joined=0` → `teardown complete` → lease released, immediate
  relaunch opens instantly; F5 reload leaves runtime stable;
  `.vivy/logs/vivy.log.2026-10-04` written.

## Real findings (all host-side, platform-independent)

- **F1 — sealed exe cannot render the Vue UI.** `agent-diva-gui/dist/` is
  gitignored; pack stages host sources via `git ls-files`, so the embedded
  FS contains only `dist/.gitkeep`. The packaged `frontend/` payload sits
  beside the exe but `divagui.Frontend()` never serves it. Every
  UI-dependent acceptance row is unreachable end-to-end.
- **F2 — no reopen affordance.** The tray is created without
  `AttachWindow`/`SetMenu`: click is inert, no menu. `d.reopen()` is only
  reachable via `OnSecondInstanceLaunch`, which can never fire because
  `Compose()` opens the sealed host first and the organism lease kills
  instance 2 before the Wails app exists. Once hidden, the window cannot
  be reopened without killing the process.
- **F3 — no explicit-Quit affordance.** No tray menu item, no UI quit
  control. Graceful quit exists (Wails signal handler → `a.Quit()` → full
  teardown order) but only via console CTRL_C/CTRL_BREAK — an end user
  can only taskkill.
- **F4 — no audio endpoints on this VM.** Audiosrv disabled, zero PNP
  audio devices → microphone capture/playback unverifiable →
  pending-with-reason, not failed.
- **F5 — build fix kept local:** Python `os.lstat` marks `.cmd`/`.exe`
  files +x on Windows while Go `os.Lstat` reports 0666 →
  `source_tree_hash` diverged from the SDK digest. `scripts/build-desktop.py`
  now strips synthetic exec bits on `os.name == "nt"`.
- **F6 — renameio Windows blocker handled upstream:** first unblocked
  locally via `DotNetAge/govector v0.1.8→v0.1.10` (vendors hnsw with
  `os.Rename`); then reverted in favour of the pushed fix — vendored
  `agent-vivy/third_party/renameio` + `replace` in both go.mods
  (3862b768/81c381f8).
- **F7 — deps links need real directory symlinks** (`mklink /D`): NTFS
  junctions are not classified as dir-or-symlink by Go `os.Lstat` →
  `hashSourceTree` fails with `Incorrect function`. Symlinks are identical
  to the committed git entries (not committed).

## GO/NO-GO

- **W0 Task 5 (keyring): GO.** Full set/get/delete/presence/no-fallback
  proven on Windows Credential Manager including the production namespace.
- **W0 Task 4 (lifetime + voice prereqs): NO-GO for full sign-off.**
  Core lifetime mechanics are proven, but F1 (UI not sealed), F2 (no
  reopen path) and F3 (no quit affordance) are product-level gaps, and
  the mic/playback prerequisite is untestable here (F4).
