# Reproduction commands

From the VIVY task checkout, loopback permission required. Run the ONNX App matrix without competing heavy native/race jobs when reproducing the latest serialized scheduling. Original deadlines remain unchanged; resource-contention observations are preserved and not assumed resolved.

```bash
source /workspace/work/memory-loop/tools/environment.sh
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-recall-serial-fixture.json
mapfile -t memory_loop_sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -overlay /workspace/work/memory-loop/tools/recall-diagnostic-overlay/overlay.json -json "${memory_loop_sources[@]}" -run '^(TestMemoryLoopFixtureUsesRealComposition|TestMemoryLoopAutomaticReflection|TestMemoryLoopAutomaticReflectionLargeSource|TestMemoryLoopReflectionProvenanceAfterRestart|TestMemoryLoopRejectedEffectRemainsVisibleAndFenced|TestMemoryLoopFixtureRestart|TestMemoryLoopFixtureCloseAfterRestartIsIdempotent|TestMemoryLoopProcessRestart|TestMemoryLoopCaptureTerminalMatrix|TestMemoryLoopCapturesLongUserSource|TestMemoryLoopLiteralProfilePath|TestMemoryLoopNoChangeAndTriggerPolicy|TestMemoryLoopReconciliationReadsExistingWork|TestMemoryLoopPersonaReviewAndFrozenSessions|TestMemoryLoopMissionChangeFencesPendingEffects|TestMemoryLoopPublicCorrectionAndTombstoneAfterRestart|TestMemoryLoopCaptureReplayAndContentConflict|TestMemoryLoopRecallAfterProcessRestart|TestMemoryLoopRecallNegativeControls|TestMemoryLoopCorrectionAndDeletionInModelInput|TestMemoryLoopRecallDeadlineDegradesSafely|TestCognitiveActions.*|TestCognitiveActionFixtureCapture)$' -count=1
go test -json ./internal/app ./internal/modules/diva-cognitive ./internal/modules/defaults -count=1
go test -race -json ./internal/runtime -run '^(TestContextSourceAdmissionCannotBeForgedByRequest|TestCognitive.*|TestMission.*|TestUnassignedMission.*)$' -count=1
```

For actual degraded race use the same named files/overlay with -race and -run '^TestMemoryLoopRecallDeadlineDegradesSafely$'. Positive cold recall race failures remain distinct and cannot be converted to passes by selecting the degraded case.

Regenerate the diagnostic overlay using its retained raw/diagnostic-helper.go.txt copied temporarily to sdk/internal/cmd/memory-loop-diagnostic/main.go, then run the helper from VIVY with an explicit task output directory and remove that temporary source. The helper invokes official SDK APIs and does not create a final conformance-attested artifact. Baseline source lock stays unchanged.
