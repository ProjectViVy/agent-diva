# Verification

Actual target lifecycle1/replay1 and final selected composition32 all pass, zero skips, exit0. Default App133pass/22conditional skips, exit0. Default skips are not acceptance. checkpoint.json retains ten raw logs with exact SHA256, counts and observed exits, including compile/type and incorrect fixture assertions.

Initial failures were test assumptions about public failure envelopes, native create ID allocation, and deleted evidence codes, not product REDs. The first replay used an unsupported empty model mode; corrected to explicitly reflection mode with policy disabled. Native product code remained unchanged.

Use VIVY named root internal/app files excluding default_generation_test.go with task environment.sh and tools/diva-source-bound-overlay/overlay.json. Passing selection omits the preserved S05 Pulse/Recap actual RED; it does not attest full S04/S09. This controlled cursor rewind is not process-crash six-cut recovery. Exact source/commands and limitations are in VIVY docs/logs/2026-10-10-memory-loop-lifecycle-replay/.

Full just ci, new-source conformance reproduction, SDK pack/Inspect, native identity and fresh whole-phase review remain due after local freeze. Previous generated hashes do not attest new source.

## Reproduction commands

From the VIVY task checkout (loopback permission required):

```bash
source /workspace/work/memory-loop/tools/environment.sh
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-lifecycle-composition-fixture.json
mapfile -t memory_loop_sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -overlay /workspace/work/memory-loop/tools/diva-source-bound-overlay/overlay.json -json "${memory_loop_sources[@]}" -run '^(TestMemoryLoopFixtureUsesRealComposition|TestMemoryLoopAutomaticReflection|TestMemoryLoopAutomaticReflectionLargeSource|TestMemoryLoopReflectionProvenanceAfterRestart|TestMemoryLoopRejectedEffectRemainsVisibleAndFenced|TestMemoryLoopFixtureRestart|TestMemoryLoopFixtureCloseAfterRestartIsIdempotent|TestMemoryLoopProcessRestart|TestMemoryLoopCaptureTerminalMatrix|TestMemoryLoopCapturesLongUserSource|TestMemoryLoopLiteralProfilePath|TestMemoryLoopNoChangeAndTriggerPolicy|TestMemoryLoopReconciliationReadsExistingWork|TestMemoryLoopPersonaReviewAndFrozenSessions|TestMemoryLoopMissionChangeFencesPendingEffects|TestMemoryLoopPublicCorrectionAndTombstoneAfterRestart|TestMemoryLoopCaptureReplayAndContentConflict|TestCognitiveActions.*|TestCognitiveActionFixtureCapture)$' -count=1
go test -json ./internal/app -count=1
```

Both target probes use the same named files/overlay, replacing -run with their exact individual test name. Raw final target logs: chain-public-revision-sixth.jsonl and chain-capture-replay-second.jsonl.
