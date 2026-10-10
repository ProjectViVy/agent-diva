# Verification

Source the task-local tools environment, then run actual App integration through the SDK-produced activity diagnostic overlay, selecting the top-level internal/app Go sources except default_generation_test.go. The overlay is diagnostic and does not attest current source hashes.

Target tests: TestMemoryLoopForegroundBusyDefersOriginalInput, TestMemoryLoopConcurrentWakeKeepsOneOriginalActiveWindow, TestMemoryLoopMinimumIntervalDefersActualNewSource and TestMemoryLoopExcludesDerivedEvidence.

Observed focused rerun: busy + concurrent wake: 2 pass, 0 skip, shell exit 0 (chain-trigger-policy-second.jsonl). Earlier interval and exclusion runs have Go package terminal pass events; their shell process results were not retained after interruption and are not invented. Combined actual App race verification: 4 named tests pass, 0 skip, 0 fail, observed shell exit 0 (chain-trigger-final-race.jsonl, 186.089 seconds).

Preserved first failures: busy test incorrectly mixed manual override with automatic behavior; a later assertion assumed one batch despite independently durable foreground completion/capture; concurrent-wake DTO incorrectly expected nested cognition in the trigger response; derived-source assertion incorrectly expected pending_through to clear after settlement, whereas the native state retains the last admitted bound. These were test-contract defects, not claimed production fixes. Original logs are retained in the DIVA handoff checkpoint.

Full product CI, newly sealed candidate, conformance and native identity are pending for the whole development phase. No old artifact hash is applied to these tests.
