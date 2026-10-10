# S10 developer checkpoint: C02 capture receipt before ObserverHost ACK

This checkpoint adds a deterministic C02 crash handshake to the actual App
integration fixture. The handshake fires after the bound Capture sink returns
its durable receipt and before the observer provider returns its ACK, so the
test kills an owned child process at the specified boundary instead of
guessing with a delay. The product memory policy and receipt implementation
were not changed; the App has a package-private, nil-by-default test callback,
and the handshake control exists only in the Go test process fixture.

The focused race command ran the real generated integration overlay and an
actual child process against an isolated data root:

```sh
source /workspace/work/memory-loop/tools/environment.sh
mapfile -t sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -race -timeout 12m \
  -overlay /workspace/work/memory-loop/tools/activity-diagnostic-overlay/overlay.json \
  -json "${sources[@]}" \
  -run '^TestMemoryLoopCrashC02CaptureBeforeObserverAck$' -count=10
```

The command exited 0 after 442.748 seconds: 10 passes, 0 failures, 0 skips.
Each sample used a distinct data root, ten unique Capture receipt IDs, and
twenty distinct process IDs across crash/restart pairs. Every sample reached
capture sequence 1 at terminal event 7 with the observer cursor at 6 before
the crash; after restart it reused the same ingestion, sequence, canonical
record, revision 1, source hash, and single canonical row, then advanced the
cursor from 6 to 7. The watermark never advanced past the accepted source;
`pending_through` was 0 or 1 at observation time. Per-sample process IDs,
receipts, cursor positions, revision, watermark and pending window are in
`raw/chain-s10-c02-race-ten-samples-green.jsonl`.

Adjacent race regressions also passed: the existing interrupted-inference
test retained its original `[0,1]` window and watermark 0 across a real process
restart, and the real capture replay/content-conflict test passed. The latter
was rerun after sharing the observer-cursor key helper. Logs preserve the
initial compile-red test, the first runtime attempt, the corrected single
sample, all ten C02 samples, and both adjacent regressions. The first runtime
attempt stopped after restart because the fixture had not rebound the user
session; adding the existing `session/get` rebind fixed the test harness. This
was not a product defect.

VIVY code is committed at `fc78d0c6` (`test(memory): verify capture replay
before observer ack`). The `checkpoint.json` manifest hashes the preserved
raw logs and binds the observation to VIVY `fc78d0c6`, DIVA `0b0add4e`, and
Laputa `4da565e` at handoff. This used the diagnostic integration overlay and
is not a final artifact/source attestation; `candidate_id` remains null.

This proves only C02 in the developer environment. C01 and C03–C06, unknown
effect and partial-batch recovery, all other S10 cases, the same-candidate
gate, Windows, and live-model acceptance remain open. S10 therefore remains
Planned; this checkpoint does not mark the Story Done.
