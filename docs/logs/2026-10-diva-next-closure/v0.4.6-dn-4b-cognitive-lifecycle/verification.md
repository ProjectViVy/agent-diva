# v0.4.6 verification — DN-4B

Repo: agent-vivy `feat/dn-closure-wave1` (base `2f009eee`).
Toolchain: `GOSUMDB=off go test` on Linux VM.

## Named plan tests (Step 1)
- `TestPrimaryFrozenCoreActualModelInput` — PASS. Real model request shows
  7 `# kind` frozen sections after persona, no world/actmem leakage;
  payload carries frozen digest; second run same digest; Prepare error
  gates the run.
- `TestMissionAdmissionFence` — PASS. workflowNodes refuses foreign scope
  and mission-revision drift; per-admission `Resolve` pin persisted in
  revision.InputJSON; resolved-binding drift refused.
- `TestSupervisorCaptureExcluded` — PASS. Supervisor-session primary
  produces 0 captures with accepted-skip receipt; user primary 1 capture.
- `TestUnknownOutcomeNoNewAttempt` — PASS. Failed run with
  `outcome_unknown` node event → `Blocked="unknown_outcome"`, Attempt
  stays 0, fenced wakes spawn nothing.
- `TestEmbeddedCognitionStartsOnce` — PASS. `StartEmbeddedServices`
  activates the loop once (second call still once); `Close` stops it;
  second `Close` clean.

## Suite gates (Step 7)
- `go test ./internal/runtime ./internal/observerhost ./internal/embedded
  ./internal/app ./internal/modules/diva-cognitive -count=1` — all PASS.
- `go test -race ./internal/runtime -run 'TestPrimaryFrozenCore|
  TestMissionAdmissionFence|TestSupervisorCapture|TestUnknownOutcome|
  TestCognitive|TestEmbedded'` — PASS; `internal/app` same test — PASS.
- `gofmt -l` clean on touched packages.

## Honest limits
- Unknown-outcome classification reads journal error_category/message —
  `recovery_required` engine status also blocks; covered by test.
- GUI/tauri-free bound: none in this story (Go only).
