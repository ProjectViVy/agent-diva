# Verification

chain-activity-fault-first.jsonl: observed exit1 from a false assumption that a sealed deleting Session permits public history reconciliation. Source prefix zero and actual failed ACTMEM read were already observed. Production leaves admission sealed after failed archive; direct read of the actual durable Session/message Store is the appropriate preservation observation.

chain-activity-fault-final-race.jsonl: one actual generated App test passes, zero skip, observed exit0, 21.504 seconds. The real Service delete error wraps context.DeadlineExceeded; original primary/source identities stay stable, native worker recovers, and public archive retry succeeds with an actual capsule. Two complete developer observations retain native fault and restoration; artifact SHA256 and byte counts are verified by the checkpoint export.

Command: task environment, existing SDK-generated activity diagnostic overlay and App source list excluding default_generation_test.go, go test -race -json <App sources> -run ^TestMemoryLoopActivityFailurePreservesSourceAndArchiveRetry$ -count=1. Final just ci, SDK/Inspect/source identity and formal S05/S11 matrix remain pending.
