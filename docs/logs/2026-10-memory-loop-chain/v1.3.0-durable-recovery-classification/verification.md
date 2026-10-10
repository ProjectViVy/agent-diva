# Verification

Task environment: source /workspace/work/memory-loop/tools/environment.sh. Source pin: existing INOFY v0.0.0-20260930141905-71e2c9bbe47d, Eino v0.9.13 unchanged. Diagnostic activity overlay uses official generated composition and is not current-source artifact attestation.

Real SQLite adapter first product RED: chain-crash-recovery-id-collision-red.jsonl, exit1, idempotency_conflict while classifying a running process using the original admission raw commit ID. Earlier compile/identity fixture mistakes are retained separately.

After repair: go test -race -json ./internal/runtime -run '^(TestINOFYStore|TestCognitiveRecovery|TestCognitive.*Settlement|TestCognitive.*Unknown|TestINOFYWorkflow)' -count=1. chain-crash-recovery-id-final-race.jsonl: 16 named tests/subtests pass, zero skip, observed shell exit0. Verify actual match count; this selector does not claim every settlement test ran.

Actual App diagnostic: TestMemoryLoopInterruptedInferenceKeepsOriginalWindow, chain-crash-inference-classification-green.jsonl: one pass, zero skip, observed exit0, 41.627 seconds. Owned processes 294464 -> 294568; original workflow_9f265811bde03ee4, source1/window[0,1], engine recovery_required, watermark0, post-restart requests0.

Preserved failures: first compile error; startup EOF now explained by a still-valid 30-second organism lease; actual expiry rerun reached startup but kept engine running due to the production commit collision. Full source freeze/CI/conformance/sealed pack/Inspect/native identity/fresh whole-phase review are pending. No push, merge or release.

Final follow-up checks: chain-crash-recovery-id-bounded-final-race.jsonl: 21 named tests/subtests pass, zero skip, observed exit0. Visible-block regression first returns active (chain-crash-visible-block-product-red.jsonl, observed exit1). After repair, chain-crash-visible-block-final-race.jsonl: 45 named tests/subtests pass, zero skip, observed exit0. Actual interrupted-inference plus concurrent-active-window App race: chain-crash-visible-app-final-race.jsonl, two pass, zero skip, observed exit0, 107.893 seconds. Prior actual classification App race passed one test, exit0 (chain-crash-inference-final-race.jsonl). Full final-source gates remain pending.
