# S10 developer checkpoint: C03 canonical effect before caller receipt

This increment adds C03 to the actual App crash fixture. A package-private,
nil-by-default composition wrapper preserves the bound run domain and pauses
only after the real `Domain.Apply` has returned an applied receipt. The parent
then queries the public memory receipt action and reads the canonical row
before killing the owned process. The strategy caller is still blocked and
has not received the `Apply` result.

The race command used a fresh data root and an actual child-process pair for
each repetition:

```sh
source /workspace/work/memory-loop/tools/environment.sh
mapfile -t sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -race -overlay /workspace/work/memory-loop/tools/activity-diagnostic-overlay/overlay.json \
  -json "${sources[@]}" \
  -run '^TestMemoryLoopCrashC03CanonicalBeforeCallerReceipt$' -count=10
```

The command exited 0 after 590.294 seconds: 10 passes, 0 failures, 0 skips.
All ten samples used distinct operation IDs, canonical targets, and old/new
process IDs. Before each crash, the public atomic receipt matched the actual
`Apply` receipt, the canonical row had revision 1, and the database contained
the source plus exactly one reflected memory. Each new process classified the
interrupted workflow as `recovery_required`; the same public receipt remained
queryable with identical operation, target, status, and revision. The
canonical count remained 2 and the restarted process made no model requests
over two automatic-trigger intervals. Per-sample PID, operation, target,
workflow status and request counts are preserved in
`raw/c03-effect-receipt-race10.jsonl`.

`go build ./internal/app` and `git diff --check` also exited 0. The test and
package-private seam are committed in VIVY at `03bfe793` (`test(memory): verify
effect receipt recovery boundary`). DIVA `checkpoint.json` hashes the raw
race log and records the paired source revisions. This is a developer
integration-overlay checkpoint: `candidate_id` remains null and formal
acceptance is false.

C03 now has ten local race samples. S10 remains Planned: C04–C06, unknown and
partial-batch recovery, and the same-candidate gates remain open. Continue
with the actual partial-success boundary; do not mark the Story Done from this
checkpoint.
