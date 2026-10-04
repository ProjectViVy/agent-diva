# Probe verification

All commands ran in `experiments/mygo/probe` with isolated Go 1.27.1.
[MYGO-001](../../../plans/diva-next/mygo/run-001.md) records the environment,
known initial failures and scope. Commands are reproducible through `verify.sh`.

| Check | Result |
| --- | --- |
| go version/env; go mod download/verify | PASS; exact candidate checksums verified |
| gofmt and go vet ./... | PASS |
| go test -count=1 ./... | PASS; native suite explicitly skipped |
| go test -race -count=1 ./... | PASS; standalone state/generation suite |
| CGO_ENABLED=0 go test and build | PASS; framework-only Linux |
| Default Go build; Windows amd64 CGO=0 build | PASS; Windows execution pending |
| Pinned CLI generate via mygo.json; direct MYGO_GENERATE child | PASS; acquired=0, closed=0, nativeReady=false, windows=0 |
| npm ci --ignore-scripts; tsc --noEmit | PASS; frozen mygo-runtime 0.2.4/TypeScript 5.9.3 |
| MYGO_PROBE_NATIVE=1 go test -count=1 -v -run '^TestNative' -timeout 60s ./... | PASS on Linux WebKitGTK/Xvfb; seven subtests and actual post-loop Quit invariants |
| Product SDK/Generation/speech/VRM/install/provider/comparison | BLOCKED or PENDING; not represented by this probe |
| Whole-branch independent review | One fresh gpt-6-astra reviewer; no Critical/Minor; two Important findings fixed by TDD and green suites |

Native trace: native caller despite JS ID override; raw WAV/foreign replay;
revoked token; reload cancellation; unread Channel close; hide/show; untrusted
navigation/return. All passed. After the real event loop returned:

```json
{"activeCalls":0,"nativeReady":true,"probe":"mygo-v0.2.4","rootAcquisitions":1,"rootCloses":1,"shutdownError":null,"windows":0}
```

No VIVY, actual devices or providers were accessed. Native dependencies were
locally extracted. EGL/DRI3 warnings reflect software rendering. Exact
[input hashes](../../../plans/diva-next/mygo/run-001-inputs.json) and generated
client are recorded; development build hashes are not product seals.
Root Rust/GUI checks are not applicable to untouched product code.

Review fixes: independent startup/post-suite-Quit watchdog plus a forced-timeout
child regression; observed attempted/completed Send progress stops beyond the
channel flow-control budget before closing. All focused/native checks passed.
Windows branch-only CI is prepared; no Windows result is inferred before run
readback. Workflow YAML parses, action pins resolve, permissions are read-only,
job timeout is ten minutes and generated-file drift fails the workflow.

Windows readback: job 111395100533 / run 37188360734 attempt 1 completed success.
OS: Windows Server 2025 Datacenter 10.0.26100; WebView2 153.0.4234.48;
Go 1.27.1 windows/amd64, CGO=0. Module/unit/client generation, drift check,
watchdog and seven real-window subtests passed. Actual post-loop invariants:
root acquired once, closed once, active calls zero, no shutdown error.
See [run 002](../../../plans/diva-next/mygo/run-002.md) and [trace](windows-native.txt).
Earlier Windows-pending notes above are historical pre-CI states; this readback
closes that framework gate. Installed product, sealing and providers remain pending.
