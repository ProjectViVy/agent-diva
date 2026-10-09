# Issue #32 P4.2 verification

## Regression evidence

The new Wails drain-barrier regression first failed to compile because
`registerRuntimeServices` and `runtimeDrainBarrier` did not exist. After
implementation, it passed and confirmed the barrier is registered before the
runtime, remains blocked after a bounded runtime shutdown returns, then
releases only after the pump drains and the host closes.

Prior red runs for this same P4.2 patch recorded regressions for error
preservation, shared deadlines, repeated close, pump panic, capability
revocation, lifecycle handoff, and `ShutdownContext`. The late-pump-error test
initially returned only `context.DeadlineExceeded`; after joining the producer's
late error it retained both conditions.

## Local commands

The following isolated headless commands passed with Go 1.26.4 and a temporary
`agent-vivy/sdk/host/v1` stub module. This harness checks DIVA lifecycle and
speech logic; it does not prove compatibility with the sealed SDK artifact.

```text
CGO_ENABLED=0 GOSUMDB=off go test -modfile=/workspace/work/issue32/agent-diva-test.mod -mod=mod -tags 'vivy_headless server' ./internal/desktop ./internal/speech -count=1
ok internal/desktop
ok internal/speech

CGO_ENABLED=0 GOSUMDB=off go vet -modfile=/workspace/work/issue32/agent-diva-test.mod -mod=mod -tags 'vivy_headless server' ./internal/desktop ./internal/speech
passed

GOSUMDB=off go test -race -modfile=/workspace/work/issue32/agent-diva-test.mod -mod=mod -tags 'vivy_headless server' ./internal/speech -count=1
ok internal/speech
```

## Blocked gates

- The desktop race build cannot compile Wails on this Linux host because GTK4,
  WebKitGTK 6 and libsoup development packages are missing. Speech race passed.
- The canonical DIVA wrapper against its pinned source stops at VIVY/Laputa
  package-path and missing `go.sum` mismatches. Against the active VIVY branch,
  generation reaches native compilation but stops at missing GTK3,
  WebKit2GTK 4.1, glib and libsoup development packages.
- Native secondary-launch, failed-primary relaunch, crash-restart lease and
  clean-quit process acceptance were not run. These remain P1/P7 acceptance
  gates; local headless tests are not a substitute.
