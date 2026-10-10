# Reproduction commands

From the task VIVY checkout, loopback permission for actual App:

```bash
source /workspace/work/memory-loop/tools/environment.sh
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-activity-app-final-prefix-fixture.json
mapfile -t memory_loop_sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -overlay /workspace/work/memory-loop/tools/activity-diagnostic-overlay/overlay.json -json "${memory_loop_sources[@]}" -run '^(TestMemoryLoopActivity.*|TestMemoryLoopAutomaticReflection|TestMemoryLoopNoChangeAndTriggerPolicy|TestMemoryLoopReconciliationReadsExistingWork|TestMemoryLoopCaptureReplayAndContentConflict|TestMemoryLoopCaptureTerminalMatrix)$' -count=1
go test -race -json ./internal/runtime -run '^(TestCognitive.*|TestMemoryLoopCapture.*|TestDeleteSession.*)$' -count=1
go test -race -json ./internal/observerhost -count=1
```

From task Laputa garden: go test -race -json ./internal/ingest ./agentapi -count=1; go test -json ./internal/ingest ./internal/runtimecore ./agentapi -count=1. From task Laputa laputa: go test -race -json ./actmem -count=1.

Diagnostic overlay reproduction uses the v1.0 retained official SDK helper copied temporarily to sdk/internal/cmd/memory-loop-diagnostic/main.go, run from VIVY with explicit task output directory, then remove the helper. It compiles/generates/seals for diagnostic construction; final source/native/artifact attestation remains required. Never print embedded manifest base64 or repin the baseline merely to pass.
