# Reproduction commands

From the VIVY task checkout, loopback permission is required for the actual-App tests. Use package mode so Go honors the mutually exclusive `race` / `!race` build tags. Do not pass an expanded list of `.go` files: that bypasses build constraints and compiles both race variants together. The overlay hides one generated-default-only test whose referenced method is absent from the DIVA integration assembly; it does not replace product code.

```bash
source /workspace/work/memory-loop/tools/environment.sh
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-recall-serial-fixture.json
python3 - <<'PY'
import json
source = "/workspace/work/memory-loop/tools/recall-diagnostic-overlay/overlay.json"
target = "/tmp/diva-memory-loop-closeout-overlay.json"
with open(source, encoding="utf-8") as f:
    overlay = json.load(f)
overlay.setdefault("Replace", {})[
    "/workspace/work/memory-loop/agent-vivy/internal/app/default_generation_test.go"
] = ""
with open(target, "w", encoding="utf-8") as f:
    json.dump(overlay, f, indent=2)
PY
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -json ./internal/app -run '^(TestMemoryLoopFixtureUsesRealComposition|TestMemoryLoopAutomaticReflection|TestMemoryLoopAutomaticReflectionLargeSource|TestMemoryLoopReflectionProvenanceAfterRestart|TestMemoryLoopRejectedEffectRemainsVisibleAndFenced|TestMemoryLoopFixtureRestart|TestMemoryLoopFixtureCloseAfterRestartIsIdempotent|TestMemoryLoopProcessRestart|TestMemoryLoopCaptureTerminalMatrix|TestMemoryLoopCapturesLongUserSource|TestMemoryLoopLiteralProfilePath|TestMemoryLoopNoChangeAndTriggerPolicy|TestMemoryLoopReconciliationReadsExistingWork|TestMemoryLoopPersonaReviewAndFrozenSessions|TestMemoryLoopMissionChangeFencesPendingEffects|TestMemoryLoopPublicCorrectionAndTombstoneAfterRestart|TestMemoryLoopCaptureReplayAndContentConflict|TestMemoryLoopRecallAfterProcessRestart|TestMemoryLoopRecallNegativeControls|TestMemoryLoopCorrectionAndDeletionInModelInput|TestMemoryLoopRecallDeadlineDegradesSafely|TestCognitiveActions.*|TestCognitiveActionFixtureCapture)$' -count=1
go test -json ./internal/app ./internal/modules/diva-cognitive ./internal/modules/defaults -count=1
go test -race -json ./internal/runtime -run '^(TestContextSourceAdmissionCannotBeForgedByRequest|TestCognitive.*|TestMission.*|TestUnassignedMission.*)$' -count=1
```

For the actual injection race and default-deadline degradation, use package mode and the same overlay with `-race`; their separate runs preserve the distinction between an injected-memory request reaching the model and safe degradation when the production default deadline expires:

```bash
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -race -json ./internal/app -run '^TestMemoryLoopMemoryInjection$' -count=1
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -race -json ./internal/app -run '^TestMemoryLoopRecallDeadlineDegradesSafely$' -count=1
```

Regenerate the diagnostic overlay using its retained raw/diagnostic-helper.go.txt copied temporarily to sdk/internal/cmd/memory-loop-diagnostic/main.go, then run the helper from VIVY with an explicit task output directory and remove that temporary source. The helper invokes official SDK APIs and does not create a final conformance-attested artifact. Baseline source lock stays unchanged.
