# P7.1/P7.2 continuation — native Linux amd64 candidate gates

Continuation run on a Linux amd64 box with full GTK3/WebKit4.1 dev
dependencies, at `feat/issue32-desktop-gates` `fa7e40f4`, VIVY
`feat/issue32-remediation` `825876b1`, Laputa `30fa208e`, Inofy
`v0.0.0-20260930141905-71e2c9bbe47d`.

Toolchain: Go 1.26.4, Node 24.9.0, pnpm 10.33.2, just 1.43.1,
wails3 v3.0.0-beta.27, pwsh 7.5.4 (`powershell.exe` shim present).

## Defects found and fixed on the pin branch

- `justfile` `desktop-bindings-check`: `$$` inside double quotes reaches
  bash as the PID, so the untracked-files assertion could never pass.
  Fixed to `$(...)` (`8f0db95b`).
- `agent-diva-gui/dist/.gitkeep`: force-tracked inside a gitignored
  `dist/`; `vite build` (emptyOutDir) deletes it, after which
  `go-test`'s `stage_tracked` lstat fails — i.e. `just ci` could never
  pass after `gui-build`. Untracked the file (`356f6c81`).
- `justfile` `desktop-seal-check`: same `$$` class — `tmp=$$(mktemp -d)`
  was a bash syntax error on every platform (`001d7041`).
- `.gitignore`: `artifacts/` candidate output tree was unignored, making
  the host dirty once a candidate existed and rejecting subsequent
  release-mode builds. Added `artifacts/` (`fa7e40f4`).

## Canonical gates (all on `fa7e40f4`)

- `just gui-test`: 72 files / 578 tests passed.
- `just gui-build`: vue-tsc + vite build passed.
- `just go-test` (consumer-modfile race tests): passed.
- `just desktop-bindings-check`: zero tracked/untracked drift.
- `just desktop-boundary-check` (+selftest), `just desktop-contract-tests`,
  `just shell-bridge-test`, `just legacy-selftest`,
  `just transition-boundary-check`: all passed.
- `just desktop-seal-check`: disposable sealed build + derived-lock check
  passed.
- `python3 -m unittest discover -s scripts/ci -p 'test_*candidate*.py' -v`:
  13/13 strict-candidate tests passed (P7.2 Steps 1–2 gate behavior).

## Retained candidate — `artifacts/candidate/linux-amd64`

- `python scripts/build-desktop.py --mode build --platform linux-amd64
  --output artifacts/candidate/linux-amd64`: sealed artifact produced
  (`diva` ELF 241,017,880 bytes, sha256
  `0c3d947612e89b34593d24636ebc1c7a4b67609ce72b87b66669cfb698879692`).
- `--mode check-lock --platform linux-amd64 --derived-lock
  artifacts/candidate/linux-amd64/input-lock.json`: verified.
- `go run ./sdk inspect-artifact artifacts/candidate/linux-amd64`
  (VIVY `825876b1`): passed; manifest internal digest
  `d61f174459bdf3157f2a29ea857ca58474360ec7f1eee7cc29f331d395e581e7`,
  module digest `43f52909310ba2646fb280ee5fbd16fb541b8c5dfc664fe9c9951b72eec1cc89`.
- Evidence file hashes captured in
  `artifacts/candidate/linux-amd64/checksums.sha256`.

## VIVY literal CI

- `just ci` (with `powershell.exe` on PATH): passed end-to-end —
  ensure-laputa, bootstrap-test, fmt-check, ui-ci (stage-ui + typecheck +
  620 vitest), vet, `go test ./...`, headless-compile, plugin-ci. One
  known flake observed once in `internal/app`
  (`TestPlanGoalIntegratedRoundLimitBlocksDurably`, embedded-sqlite
  `schema_meta` init race under parallel package load); package and suite
  both green on rerun.

## Still pending (owner/platform gates)

- Windows amd64 candidate build + Inspect + WebView2/install acceptance.
- `check_wails_candidate.py --require-all-passed` needs both platform legs
  plus the 17-row installed-product acceptance — blocked on Windows and
  owner sign-off.
- Real voice/provider scenarios (SiliconFlow STT, SiliconFlow/MiniMax TTS,
  first real model turn).
- Annotated `archive/tauri-cabi-20261004` tags and W6 cutover decision
  before any publication.

## Installed-product smoke (2026-10-09, on retained candidate)

### Defect found & fixed during smoke
- `internal/desktop/sender.go` `allowedOrigins` lacked `wails://localhost` — the exact
  asset baseURL wails v3.0.0-beta.27 uses on Linux. `/wails/runtime` requests carried a
  forbidden Origin → 403 → native window rendered only the CSS background (flat navy).
  Discriminated vs environment: WebKitGTK 4.1 `MiniBrowser` paints correctly on the
  same VNC display, so it was product-side, not renderer-side.
- Fix: added `"wails://localhost": true` + test case in `sender_media_test.go`.
  Committed as `0921e0ee`; candidate rebuilt (old bytes `0c3d9476…` superseded).
- Windows child session notified to build from `0921e0ee` so both legs share source SHA.

### New candidate identity (source SHA `0921e0ee`)
- `diva` sha256 `8bd341447299a8d32c5df31f93f394ff70137b5507c4ffa2bef7f6ee247c3a33`
- `build-report.json` `a39861ea…`, `input-lock.json` `f21d68c5…` (unchanged)
- generation `7076b8bcbf71617344cd70e231bbb8ca4a936f28de48a0687fcd2bb3dea6a8d5`
- Re-gated: `check-lock` derived-lock pass; `go run ./sdk inspect-artifact` pass.

### Smoke results on new bytes (DISPLAY=:0, isolated HOME profile)
- Fresh-profile launch: host opens, generation `7076b8bc…` matches inspect; window maps.
- First-run onboarding UI renders correctly (screenshot evidence; was flat navy pre-fix).
- Single-instance: second launch logs `second instance rejected; focusing primary window`
  and exits cleanly (verified ×2); primary window stays mapped.
- Stale lease reclaim: previously verified — killed process's lease is reclaimed on next launch.
- Bounded quit: SIGTERM → process exits <1s, window unmaps.
- Path-with-spaces: binary copied to `/tmp/diva path with spaces/diva` launches and
  renders the same onboarding UI; only benign dbus/systray warnings.

### Still pending (owner / env-blocked)
- Tray hide/reopen: no `org.kde.StatusNotifierWatcher` on this VM — env-blocked.
- First real model turn, chat matrix, cognition/console/speech lanes, mic/keyring,
  real STT/TTS providers: owner gate.
- Clean install/uninstall packaging: no installer artifact for linux-amd64 (bare binary).

## Portability defect found by windows-amd64 leg (2026-10-09)

`source_tree_hash` (python) and `hashSourceTree` (sdk/internal/go_host.go)
hashed `lstat` perm bits; Windows reports 0666/0777 vs Linux index
0644/0755, so the pinned treeSHA256s could never reproduce on Windows —
`check-lock` and `sdk pack` failed verbatim. Fixed upstream and committed:

- agent-vivy `6045c0a6`: `hashSourceTree` canonicalizes to git index modes
  (regulars -> index perm & 0777, dirs/gitlinks -> 0755, links -> 0777;
  untracked regulars on Windows -> 0644; symlink-as-text-file -> blob payload).
- agent-diva `7b427510`: byte-exact python twin + repin to vivy `6045c0a6`
  (treeSHA256 `31e467db04da…`).

Verification: `ed4592b1` (vivy pin at 825876b1) and `b4181c39` (laputa)
both reproduce byte-exact on clean Linux trees under the new scheme;
Go and python ports agree per-entry.

### Third candidate identity (source SHA `7b427510`, vivy `6045c0a6`)
- `diva` sha256 `eb89baed15e44401afdd3dae452d7f26f41ca5eaeaae87bab5e56f1f1bca6f52`
- generation `55d4a3f94ae2b87e0262e99b9f28fcd8f7f9629da3debe507172a06e8b73aece`
- manifest internal digest `d61f174459bdf315…` (unchanged)
- Re-gated: build + derived-lock + inspect-artifact all pass; fresh-profile
  launch renders onboarding UI (screenshot), host generation matches.

All previous candidate bytes (0c3d9476, 8bd34144) are superseded.
