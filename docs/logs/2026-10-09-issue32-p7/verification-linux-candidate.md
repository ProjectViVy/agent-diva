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
