# W0-2 Verification — commands and observed output (Windows x64)

All commands run natively on Windows Server 2022 (Build 20348), session 1,
Git-Bash + interactive desktop + PowerShell. `$REPOS` = `C:\Users\Administrator\repos`.

## 1. Toolchain

- `go version` → `go1.26.8 windows/amd64`; node `v24.0.1`; `wails3 version` →
  `v3.0.0-beta.27`; mingw-w64 via choco; Python 3.13.12.
- WebView2 was absent → installed evergreen bootstrapper → Runtime
  **154.0.4258.53** (confirmed in Wails platform line at runtime).
- `deps/agent-vivy`, `deps/laputa` checked out as plain files (git symlink
  semantics) → replaced with real directory symlinks via
  `cmd /c mklink /D deps\agent-vivy ..\..\agent-vivy` (same for laputa).
  Junctions (`mklink /J`) were tried first and **fail**: Go `os.Lstat`
  does not classify junctions as dir-or-symlink → `hashSourceTree` hits
  `read ... Incorrect function`. Do not commit — links now match the
  committed git symlink entries.

## 2. Sealed build

`python scripts/build-desktop.py --mode build --development`

- Obstacle 1: `go run ./sdk pack` → `coder/hnsw@v0.6.1 encode.go:
  undefined: renameio.TempFile` — upstream renameio v1.0.1 is
  `+build !windows`-only. Local unblock: `DotNetAge/govector`
  v0.1.8→v0.1.10 (vendors hnsw with `os.Rename`, API-compatible with
  laputa usage). Later reverted when upstream vendored
  `third_party/renameio` + `replace` landed (vivy 3862b768, diva 81c381f8)
  — rebuilt on those pins.
- Obstacle 2: `lock/pack treeSHA256 mismatch` — Python `os.lstat` gives
  `.cmd`/`.exe` files synthetic +x bits; Go `os.Lstat` reports 0666. Fixed
  in `source_tree_hash` (`mode &= ~0o111` for regular files on
  `os.name == "nt"`). Verified by byte-level diff of both hash streams.
- Result (final build): `diva.exe` 106,653,696 B, sha256
  `623bfc05cf70997f2163668da2646294de46f5ad8457aa3e8a29b2acc2052800`,
  generationId `a24a62a2cd2b816528de07c2b5c19af88ab63f8138d8b97ee9a2e1dbc81bd033`.
  build-report: host `81c381f8` (dirty: build-desktop.py fix +
  tools/speech-credprobe — both kept uncommitted at build time),
  vivy `3862b768` clean, laputa `6f2eed2`.

## 3. W0 Task 5 — Credential Manager

- `go test -tags vivy_headless -count=1 ./internal/speech` → `ok` (0.084 s).
- `go test -tags vivy_headless ./tools/wails-probe/...` → `ok` (0.121 s).
- `go run ./tools/wails-probe --keyring` →
  backend `Windows Credential Manager (wincred)`, `set: ok`, `get: ok`,
  `delete: ok`, `post_delete: not found (expected)`, `available: true`,
  `fallback: none — backend failure is an error, never plaintext`.
- New `tools/speech-credprobe` (real namespace via
  `speech.NewCredentialStore(speech.OsKeyring{})`):
  - `-op set` → `presence(before): absent` → `set: ok` →
    `presence(after set): present`.
  - `cmdkey /list` (via PowerShell) shows
    `LegacyGeneric:target=dev.projectivy.diva.speech:v1.siliconflow`,
    Generic, `Local machine persistence` — the credential is really in
    Credential Manager, not a file.
  - `grep -rl "w0-2-credprobe-7f3a9c51"` over `%APPDATA%`,
    `%USERPROFILE%/.config`, `%USERPROFILE%/.vivy` → no match →
    no plaintext fallback copy.
  - `-op delete` → `presence(before): present` → `delete: ok` →
    `presence(after delete): absent`; `cmdkey /list` no longer lists it.
- Locked-store row: **not exercisable** — Windows Credential Manager has
  no per-user lock/unlock surface; backend errors map to
  `credential_unavailable` (OsKeyring error path, no fallback). Recorded
  as N/A-for-backend in the probe fixture.

## 4. W0 Task 4 — native lifetime (interactive desktop)

Run: `diva.exe > diva-runN.log 2>&1` (console subsystem → logs captured).

- **Startup** → `vivy host opened generation a24a62a2…` (first build
  `1d626e40…`), `WebView2=154.0.4258.53`, `[WebView2] Environment created
  successfully`, `Platform Info: Windows Server 2022 Standard`. `%APPDATA%
  \DIVA\vivy.yaml` seeded on first launch; `~/.vivy/{vivy.db,garden,logs,
  transfers}` created; `.vivy/logs/vivy.log.2026-10-04` written.
- **Close-to-hide**: window X click → window disappears, `tasklist` still
  shows `diva.exe` (PID unchanged). Re-confirmed at OS level:
  `taskkill /PID <pid>` (no /F, posts WM_CLOSE) → "Sent termination
  signal" → window hidden, process still alive → hook cancels destroy.
- **Tray**: overflow icon exists, tooltip `DIVA desktop`. Left/right
  click → **no action** (no `AttachWindow`, no `SetMenu` in
  `internal/desktop/app.go`; Wails smart defaults never engage ToggleWindow).
- **Reopen**: unreachable — `d.reopen()` is only invoked from
  `OnSecondInstanceLaunch`, but `Compose()` calls `hostv1.Open` before the
  Wails app exists; the second instance dies on the organism lease and the
  hidden primary is never notified. Code path + live behaviour confirmed.
- **Single instance**: second `diva.exe` while primary runs →
  `diva: open vivy host: host: internal (code -32086): ... storage:
  organism lease held: the shared Vivy workspace is already in use by
  another process`; second process exits, primary unaffected. Wails
  SingleInstance (`foundation.undefine.diva`) is armed in code but
  unreachable in practice (lease fires first).
- **Crash-restart**: `taskkill /F /PID <pid>` → immediate relaunch →
  `organism lease held` (lease row survives; TTL 30 s, heartbeat 10 s —
  crash-safety margin) → relaunch after TTL expiry → `vivy host opened`
  on the same `.vivy/vivy.db`, window renders again.
- **Explicit quit**: no UI affordance (no tray menu/quit control). Graceful
  path proven via console signal: `GenerateConsoleCtrlEvent(CTRL_BREAK)`
  → `INF Quitting application...` →
  `diva: speech teardown: inflight=0 joined=0` → `diva: teardown complete`
  → prompt returns. Shutdown ordering: admission closes → speech drain
  joins inflight under CloseBudget → host close → PostShutdown reports
  complete. (Cosmetic noise: Chromium
  `Failed to unregister class Chrome_WidgetWin_0` on teardown.)
- **Lease release on clean quit**: immediate relaunch after graceful exit
  → `vivy host opened` with no TTL wait — contrasting the post-crash path.
- **WebView reload**: F5 in the window → page reloads (re-renders
  embedded tree), process/runtime unaffected, no errors in log.
  Context-invalidation/turn-dedup needs the real frontend → still pending.
- **Shutdown while request in flight**: no UI/credentials to dispatch a
  real request; the join-under-CloseBudget path is the one exercised by
  w4-1 service tests (`Shutdown(2s)` reports inflight/joined honestly).
- **Mic/audio**: `Get-PnpDevice -Class AudioEndpoint` → empty;
  `Win32_SoundDevice` → empty; `Audiosrv` Disabled+Stopped. No endpoint
  exists → capture/playback rows pending-with-reason.
- **Frontend finding (F1)**: the window renders only a `.gitkeep` link —
  `agent-diva-gui/dist/` is gitignored → `git ls-files` staging embeds
  just `.gitkeep`; the packaged `frontend/` payload beside the exe is
  never served by `divagui.Frontend()`. The Vue UI cannot load in the
  sealed artifact on any platform.

## 5. Fixture check

`python scripts/ci/check_wails_candidate.py docs/plans/diva-next/fixtures/wails-candidate-acceptance.json --build-report ... --binary ...`
→ see acceptance.md for final counts (one row moves to `failed`:
W5-T4-HIDE-REOPEN — the reopen affordance does not exist).
