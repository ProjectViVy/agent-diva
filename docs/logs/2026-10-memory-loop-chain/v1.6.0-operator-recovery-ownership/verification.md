# Verification

Actual generated App, SDK diagnostic overlay, isolated synthetic profile and native memory. The initial chain-operator-recovery-red.jsonl reproduced the defect, observed exit 1. After the fix, chain-operator-recovery-final-race.jsonl: one App test passes, zero skips, observed exit 0, 20.929 seconds. The original Run completes, original canonical ID/revision stays stable, and exactly one model request occurs.

Complete RPC package race regression: chain-operator-recovery-rpc-final-race.jsonl, 220 named tests/subtests pass, zero failures, one platform skip (TestResolveProjectAttachmentsRejectsNativeWindowsADS), observed exit 0, 115.659 seconds. Six exported actual artifacts have verified SHA256 and byte counts.

Commands: source the task environment; go test -race -json ./internal/rpc -count=1; App command uses the existing generated activity diagnostic overlay with the App source list excluding default_generation_test.go and -run ^TestMemoryLoopOperatorRecoveryDoesNotInterruptOwnedRun$ -count=1. Full just ci and same-candidate SDK/native verification are deferred until the local queue is complete; this is development evidence, not formal Story acceptance.
