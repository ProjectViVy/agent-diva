# W3-1 verification

## Go seam tests (dev go.mod, `-tags 'gtk3 vivy_headless'`)

`go test -race` internal/desktop: 17 tests, all pass —
CallReply envelope kind/code preservation, pump gap→events→lost ordering,
shutdown stops pump + closes host (idempotent), MediaToken bound-window
only, DesktopDispatch not_ready/deny/handler, sender gate denies foreign
origin + forged/absent window id, media WAV round trip raw bytes, 10MiB
bound + over-bound rejection, token/window/revocation denial, peak-heap
assert <40MiB, close-admission + teardown truth, reopen noop.

## Frontend

`pnpm --dir agent-diva-gui test` — 551/551 pass (67 files), including the 4
updated desktop/pet mock files. `vue-tsc --noEmit` clean. Browser-mode
facade keeps `native_unavailable` honesty (speech.test.ts).

## Sealed pipeline (scripts/build-desktop.py)

- `--mode test`: staged pack → consumer.mod/sum → `go build` +
  `go test -race -tags 'vivy_headless gtk3'` on the staged host — PASS.
- `--mode build --development`: frozen pnpm install+build → sealed ELF
  `diva` (270MB debug, linux/amd64) + generation.json hostBuild
  (`buildTags:["gtk3"]`, tools pins, all digests) + consumer.mod/sum +
  checksums + `inspect-artifact` clean.
- xvfb smoke: `./diva --config …` → `vivy host opened` (generation
  9046bd58…), Wails AssetServer middleware+handler up, window path
  reached; systray gracefully degraded under bare xvfb; clean quit.
- Single-instance: second `./diva` rejected before runtime startup —
  `organism lease held: the shared Vivy workspace is already in use by
  another process` (VIVY workspace lease) + Wails `SingleInstance`
  (UniqueID foundation.undefine.diva, focus-primary) as second layer.

## Not yet covered (recorded, not waived)

- Windows x64 lifetime/voice/keyring leg — W0 Task 4/5 still pending the
  Windows child session; W3 hide/reopen/Quit acceptance is therefore
  Linux-proven only.
- DesktopDispatch native handlers (speech/credentials/voice assets/pet)
  land in W4; dispatch now answers `not_ready`.
- No wails imports exist outside the desktop-host seam + generated
  bindings + deferred pet window ops (Events/thisWindow).
