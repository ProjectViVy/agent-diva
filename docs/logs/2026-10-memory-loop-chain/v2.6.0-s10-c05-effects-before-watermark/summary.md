# S10 developer checkpoint: C05 effects complete before watermark persistence

This increment adds a test-only `SnapshotStore` wrapper to the actual App
composition. It signals immediately before the cognitive state write that
advances `watermark` to `pending_through`. The parent verifies that the
workflow is already `succeeded`, its applied effect result and canonical
revision are durable, while the persisted cognitive state still names the
active workflow at watermark 0. The parent then kills that process before the
store receives the watermark update.

The ten-sample race command used a fresh data root and actual child-process
pair for every repetition:

```sh
source /workspace/work/memory-loop/tools/environment.sh
mapfile -t sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -race -timeout 15m \
  -overlay /workspace/work/memory-loop/tools/activity-diagnostic-overlay/overlay.json \
  -json "${sources[@]}" \
  -run '^TestMemoryLoopCrashC05EffectsDoneBeforeWatermark$' -count=10
```

It exited 0 after 700.302 seconds: 10 passes, 0 failures, 0 skips. Each sample
observed workflow state `succeeded`, watermark 0, one applied effect receipt
at revision 1, and exactly two canonical rows before the crash. After waiting
for the real crashed-process lease to expire and starting a new process, the
original receipt and canonical row remained unchanged; the recovery pass
advanced watermark to 1, cleared the active workflow, and made no model calls.
The handshake, workflow identity, receipt target, old/new watermark, and
process pair for every sample are preserved in
`raw/c05-effects-before-watermark-race10.jsonl`.

`go build ./internal/app` and `git diff --check` also exited 0. VIVY is
committed at `0cd1b8f7` (`test(memory): verify watermark recovery boundary`).
DIVA `checkpoint.json` hashes the race log and records the paired source
revisions. This remains a developer integration-overlay checkpoint:
`candidate_id` is null and formal acceptance is false.

C05 now has ten local race samples. S10 remains Planned: C06, unknown and
partial recovery beyond these cutpoints, and same-candidate acceptance gates
remain open. This checkpoint does not mark the Story Done.
