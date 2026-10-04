# MyGo observation ledger

This file owns upstream pins, observation runs and recommendations.
[index](index.md) alone owns Story readiness.
Evidence labels: source-read, compile/unit, native-probe, installed-product,
configured-provider and owner-accepted. A source-read is never a native pass.

## Initial source-read — 2026-10-04

| Input | Exact baseline |
| --- | --- |
| DIVA main | 5444795a2d9db31e158c2cf009d64697e6289e50 |
| VIVY main | fc559e6b03ce4e65c0099b9745855dccc4fb067e |
| Wails plan | DIVA docs commit 1e9dca1aed557009f5d868c79e3c907d963ede42; candidate v3.0.0-beta.27 |
| MyGo released candidate | v0.2.4, published 2026-10-03T19:47:20Z; commit 154449f603ede999a1dfc8692717de62f7bacf12 |
| MyGo observed main | 5681bb2038eacf2e5563a8a2be945093e205ed70; not substituted for the released candidate |
| Compiler declaration | Candidate go.mod requires Go 1.27.1; Wails delivery plan uses Go 1.26.4 |
| Actual compile/native/performance evidence | Not run; no pass asserted |

Immutable upstream links use
[the candidate source](https://github.com/egoist/mygo/tree/154449f603ede999a1dfc8692717de62f7bacf12).

| Evidence | Source at candidate commit | Observation / implication |
| --- | --- | --- |
| Dependencies | go.mod | purego dependency; no-CGO framework does not eliminate native FFI or prove whole-product no-CGO compatibility. |
| Lifetime/generation | app.go | App.Run on main thread; MYGO_GENERATE writes TS and skips native loop; BeforeQuit/WillQuit/Quit hooks need explicit resource coordination. |
| Caller identity | ipc.go | CallerWindow uses injected bound-method context; page context and root Host lifetime must differ. |
| Flow control | channel.go | Channel has a 1 MiB outstanding-payload window; isolate frontend backpressure from runtime producers. |
| Raw protocol | protocol.go | HTTP method/body/headers are forwarded; RemoteAddr is framework-assigned mygo:<window id>. This needs native trust/lifecycle proof. |
| TS generation | cmd/mygo/generate.go | Generator executes configured app generation path; sealed build integration must account for staged module/overlay inputs. |

Earlier inspection of main at c81a80b59e971e80f4a184cb027fc49dea5994d3 found
macOS/Linux/Windows GUI CI lanes and Windows ARM64 native-UI tests in
.github/workflows/ci.yml. Those are upstream workflow definitions, not our
candidate test runs. Optional native toolkit/terminal features are outside scope.

Known delivery dependencies: public Go Host, sealed external pack, Go speech
and installed Wails baseline are not accepted outputs at inspected main.
No secrets, fixtures containing user audio or binary artifacts belong in this ledger.

## Run record format

Append one row per relevant trigger and link redacted evidence below it.

| Run ID / trigger | Candidate + baseline commits | Evidence level | Affected gates | Result | Missing evidence | Recommendation |
| --- | --- | --- | --- | --- | --- | --- |
| SOURCE-20261004 / user research request | Initial pins above | source-read | MY-0 through MY-3 | Source review only | Compile, native, installed/provider and owner acceptance | Keep Wails delivery; maintain the research branch |

Each run supplies: date/operator; Go module/sum and frontend lock hashes; CLI/npm
versions resolved from the pinned candidate; OS/arch/WebView2/native tools;
clean source and Generation IDs; artifact hashes from build reports; commands
and raw observations; tested scenarios and failures; changed adapter/generated
lines; dependency and packaging maintenance effort; measurement method/variance.
Mark every result pass/fail/pending/not-applicable with a reason and evidence.
Do not track this branch's own commit inside a tracked source lock; resolve the
host commit into an untracked build report to avoid a self-referential pin.

## Reevaluation and stop conditions

Reevaluate relevant quit/IPC/media/Windows/build/toolchain fixes or accepted
Host/speech interface changes. Preserve unchanged evidence, invalidate affected
passes and perform focused rechecks. Pause a candidate that needs public contract
weakening, an unsealed rebuild, a second runtime or unbounded upkeep; record the
concrete issue and cheaper alternative. No automatic monitoring is configured.
