# S10 developer checkpoint: C04 partial effect batch

This increment exercises a three-effect memory batch in the actual App and
canonical backend. Three distinct user turns are durably captured while
cognition is disabled; only after all three sources exist is the loop
enabled. A test-only, nil-by-default Domain wrapper waits after the second
memory effect has committed its canonical mutation receipt and effect-ledger
row, but before that receipt returns to the strategy caller. The first effect
has already returned to the caller, and the third effect has not been
attempted.

The ten-sample race command used a fresh data root and real child-process
pair for every repetition:

```sh
source /workspace/work/memory-loop/tools/environment.sh
mapfile -t sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -race -timeout 20m \
  -overlay /workspace/work/memory-loop/tools/activity-diagnostic-overlay/overlay.json \
  -json "${sources[@]}" \
  -run '^TestMemoryLoopCrashC04PartialEffectBatchStopsAtUnknown$' -count=10
```

It exited 0 after 939.868 seconds: 10 passes, 0 failures, 0 skips. Each
sample confirmed the actual reflection request contained all three source
entries. The effect ledger and public atomic receipt exposed unchanged e0/e1
receipts at revision 1; e2 had no result row and canonical storage contained
only the three captured sources plus the first two effects. After killing
the process and waiting for its real lease to expire, a new process classified
the original workflow as `recovery_required`, preserved both committed
receipts and canonical targets, kept the result ledger free of e2, and made no
model requests. Unique operation IDs, targets and process pairs for every
sample are preserved in `raw/c04-partial-effect-race10.jsonl`.

The effect-index selector was also checked against C03's index-0 cutpoint:
one race sample passed after the selector change. `go build ./internal/app`
and `git diff --check` exited 0. VIVY is committed at `70cc109c` (`test(memory):
verify partial effect batch recovery`). DIVA `checkpoint.json` hashes both
the C04 matrix and C03 seam-regression log and records the paired revisions.
This remains a developer integration-overlay checkpoint; `candidate_id` is
null and formal acceptance is false.

C04 now has ten local race samples. S10 remains Planned: C05–C06, complete
unknown-effect and partial-batch recovery, and same-candidate acceptance gates
remain open. This checkpoint does not mark the Story Done.
