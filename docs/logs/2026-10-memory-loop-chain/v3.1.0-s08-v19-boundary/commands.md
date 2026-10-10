# Commands and execution record

From `/workspace/work/memory-loop/agent-vivy`:

```sh
source /workspace/work/memory-loop/tools/environment.sh
go test -overlay=/tmp/diva-memory-loop-closeout-overlay.json ./internal/app \
  -run '^(TestMemoryLoopRecallAfterProcessRestart|TestMemoryLoopRecallNegativeControls|TestMemoryLoopOrdinaryRecallAuthorityBoundary)$' \
  -count=1 -v
```

Result: exit 0; 3 top-level tests plus 9 subtests passed; 0 skipped; 69.6 seconds. The exact raw output is in `raw/s08-focused-suite.txt`.

The first attempt failed before test execution because the sandbox rejected the test server's local IPv6 loopback bind (`listen tcp6 [::1]:0: operation not permitted`). Turn-scoped local network permission was then granted solely for the test-local synthetic model, and the command above passed. No external network request was made.

The package-mode invocation preserves `race`/`!race` file selection and the DIVA diagnostic integration overlay. It is a developer run, not a sealed DIVA build or formal candidate.
