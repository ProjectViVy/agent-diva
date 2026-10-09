# Verification record

## Passed

- TDD regression tests failed before the H1 change in a temporary headless
  harness that copied the production `runtime_service.go` and its test file,
  then passed after the shared authorization helper was added.
- `go test -race ./internal/desktop -count=1 -v` passed in that isolated
  harness (8 top-level runtime-service tests, including authorization cases).
  The harness replaced only the Wails application surface, speech error type,
  host-v1 wire types, and existing media capability collaborator.
- `go test ./sdk/internal -run '^TestPrepareConsumerModfileMapsHostLaputaReplaceToSnapshot$' -count=1 -v` passed after first failing on the old `./deps/laputa/garden` path.
- `python -m unittest scripts.ci.test_desktop_ci_contract -v` passed after
  first failing because test mode had no embedded `index.html`.
- `corepack pnpm test src/api/vivy/client.test.ts` passed with DIVA's pinned
  pnpm `10.33.2`: 1 file, 11 tests.
- `gofmt` and `git diff --check` passed for the changed Go and Python files.
- The frozen GUI install left the tracked `agent-diva-gui/pnpm-lock.yaml` at
  SHA-256 `01ef0ea82f41b54be2103a8bc7cf54a407a2be6b39a48a6137010a8acecea249`.

## Pending local environment gates

- The final `python scripts/build-desktop.py --mode test --host-dir ...`
  reached native host compilation after resolving the SDK source closure, then
  stopped because this container has no `glib-2.0`, `gtk+-3.0`,
  `webkit2gtk-4.1`, or `libsoup-3.0` development packages. The required Linux
  race/native host run must execute on a runner with those packages.
- The baseline VIVY `just ci` attempt was stopped after repeated package
  registry failures during frozen pnpm installation; it did not pass.
- Windows, PostgreSQL, microphone, and real-provider acceptance were not run.

The headless harness is focused unit evidence. It does not substitute for the
canonical sealed host build, platform matrix, product acceptance, or release
acceptance.
