# S10 developer checkpoint: C01 terminal before Capture

This increment adds the C01 cutpoint to the actual App integration fixture.
A test-only sink decorator signals immediately before calling the bound
Capture sink, after the real terminal event is already in the Journal. The
parent verifies the terminal event and confirms that the corresponding
ingestion row is absent before killing the owned process. On restart, the
same durable event is replayed into the real sink. The file handshake and
gate live in test code; the App composition seams are package-private,
nil-by-default, and expose no product setting or control command.

The C01 race command used a fresh data root and actual child-process pair for
each repetition:

```sh
source /workspace/work/memory-loop/tools/environment.sh
mapfile -t sources < <(rg --files --maxdepth 1 internal/app -g '*.go' -g '!default_generation_test.go' | sort)
go test -race -timeout 12m \
  -overlay /workspace/work/memory-loop/tools/activity-diagnostic-overlay/overlay.json \
  -json "${sources[@]}" \
  -run '^TestMemoryLoopCrashC01TerminalBeforeCapture$' -count=10
```

The command exited 0 after 454.728 seconds: 10 passes, 0 failures, 0 skips.
All ten samples used unique receipt IDs and twenty unique process IDs. Each
sample reached the same durable terminal event at sequence 7 with cursor 6;
the sink had zero ingestion rows before the crash. After restart, replay made
one ingestion row, one canonical record at revision 1, and advanced the
observer cursor to 7. Every original user fact and source role survived. The
sample PID, event, receipt, sequence, record, revision and cursor observations
are preserved in `raw/chain-s10-c01-race-ten-samples-green.jsonl`.

The existing C02 cutpoint was rerun after introducing the new sink decorator:
1 pass, 0 skip, exit 0. Its durable receipt remained unchanged and cursor
advanced from 6 to 7. The regular actual capture replay/content-conflict race
regression passed as well. The package built with `go build ./internal/app`.
The test-first compile-red, single C01 sample, 10-run matrix, C02 compatibility
run, and replay regression are all preserved in `raw/`.

VIVY is committed at `160349ca` (`test(memory): verify terminal replay before
capture`). `checkpoint.json` hashes the preserved logs and records DIVA
`0b0add4e`, VIVY `160349ca`, and Laputa `4da565e` at handoff. This is a
developer integration-overlay checkpoint, not a final artifact/source
attestation; `candidate_id` remains null.

C01 and C02 now each have ten local race samples. S10 remains Planned: C03–C06,
unknown effect and partial-batch recovery, all required cross-cutpoint cases,
and same-candidate acceptance remain open. This checkpoint does not mark the
Story Done.
