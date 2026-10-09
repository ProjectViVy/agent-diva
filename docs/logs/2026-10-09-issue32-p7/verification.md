# P7 DIVA verification record

## Passed

- `python3 scripts/build-desktop.py --mode repin --vivy-dir ... --laputa-dir ...`
  regenerated the canonical lock from VIVY `54462d15…` and Laputa
  `30fa208e…`.
- `python3 scripts/build-desktop.py --mode check-lock --platform linux-amd64`
  passed with the locked Go, Wails CLI, Node, pnpm, CGO, and Linux target.
- `python3 -m unittest discover -s scripts/ci -p 'test_*.py' -v`: 35 tests
  passed, including strict two-platform candidate rejection and Go race tests
  under the sealed consumer modfile.
- Wails binding generation with Wails `v3.0.0-beta.27` exited successfully;
  generated bindings had no tracked or untracked drift. The generator reported
  276 package-analysis warnings while running without CGO/native metadata.
- `vue-tsc --noEmit` passed.
- `vite build` passed (2,403 modules); Vite reported existing large output
  chunks over its 500 kB advisory threshold.
- `vitest run`: 72 files, 578 tests passed after setting `maxWorkers: 1` to
  keep Wails' import-time drag-init interval inside the happy-dom worker
  lifetime.
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml -p
  vivy-bridge`: 7 integration tests passed.
- `check_desktop_boundary.py --selftest` rejected 4/4 negative fixtures and
  the boundary scan was clean.
- `check_legacy_frontend_calls.mjs --selftest`: 9/9 fixtures fired; gate clean.
- `check_vivy_backend_boundary.py`: 485 packages, no legacy crates; boundary
  gate clean.
- `git diff --check` passed before commit.

## Blocked native pack

The final retry of:

```text
python3 scripts/build-desktop.py --mode test --platform linux-amd64 --test-packages ./...
```

failed during sealed Go-host generation because these pkg-config development
packages are absent: `glib-2.0`, `gobject-2.0`, `gtk+-3.0`, `webkit2gtk-4.1`,
`gio-unix-2.0`, and `libsoup-3.0`. The command did not reach native package
creation or Inspect. A Linux lock check is not a native build result.

No Windows runner was available, so Windows candidate generation, installation,
and WebView2 acceptance were not run.
