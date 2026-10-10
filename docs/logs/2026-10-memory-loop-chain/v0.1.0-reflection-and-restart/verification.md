# Recorded verification and repeatable commands

| Check | Actual result | Raw record |
|---|---|---|
| Trusted workflow budget RED | budget_exceeded after one effect | raw/chain-cognitive-budget-red.jsonl |
| Budget GREEN | targeted test passes | raw/chain-cognitive-budget-green.jsonl |
| Runtime/workflow regression | 69 pass / 0 fail / 0 skip / exit 0 | raw/chain-cognitive-runtime-regression.jsonl |
| Actual automatic reflection | pass; source, canonical effect, receipt and watermark | raw/chain-reflection-green.jsonl |
| Restart RED | unsupported Restart | raw/chain-fixture-restart-red.jsonl |
| Repeated Close RED | timeout waiting for an already consumed exit notification | raw/chain-fixture-close-red.jsonl |
| Repeated Close GREEN | pass / exit 0 | raw/chain-fixture-close-green.jsonl |
| Post-restart action RED | helper used the nil old peer | raw/chain-fixture-action-restart-red.jsonl |
| Combined composition regression | 10 pass / 0 fail / 0 skip / exit 0 | raw/chain-fixture-regression-recorded.jsonl |
| Unmodified default App package | 133 pass / 0 fail / 9 conditional skip / exit 0 | raw/chain-app-default-regression.jsonl |

The nine default-suite skips are not product acceptance. That recipe omits DIVA; helper/opt-in capture entrypoints are also inactive unless explicitly invoked. The composition run enables the action capture, invokes child helpers through owned processes and covers its required tests with no skips. Parent and child counts are not added together.

From `/workspace/work/memory-loop/agent-vivy`, use the task-local environment and diagnostic SDK-generated overlay:

```bash
source /workspace/work/memory-loop/tools/environment.sh
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-cognitive-control-fixture.json
mapfile -t memory_loop_sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -overlay /workspace/work/memory-loop/tools/diva-source-bound-overlay/overlay.json -json "${memory_loop_sources[@]}" -run '^(TestMemoryLoopFixtureUsesRealComposition|TestMemoryLoopAutomaticReflection|TestMemoryLoopFixtureRestart|TestMemoryLoopFixtureCloseAfterRestartIsIdempotent|TestMemoryLoopProcessRestart|TestCognitiveActions.*|TestCognitiveActionFixtureCapture)$' -count=1
go test -json ./internal/app -count=1
go test -json ./internal/runtime -run '^(TestCognitive|TestINOFY|TestWorkflow)' -count=1
```

In the managed environment, these commands need loopback network permission. The named-file diagnostic compilation excludes the default-generation inventory test because its assembly contract differs from the selected DIVA recipe. The following unmodified package command includes that test and verifies the default assembly independently.

The overlay predates the new source commits. It selects the real DIVA composition for diagnostic execution but does not attest the new source tree. Final full CI/conformance reproduction/pack/Inspect and native candidate rebuilding remain pending for this phase. Never merge its proof with a different native generation to declare one accepted candidate.
