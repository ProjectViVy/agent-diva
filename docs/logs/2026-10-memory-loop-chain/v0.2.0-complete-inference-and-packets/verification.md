# Verification

| Check | Actual result | Raw log |
|---|---|---|
| Complete request | RED, missing source JSON | raw/chain-cognitive-task-bounds-red.jsonl |
| Native request regression before packet repair | 75 pass / 0 skip / exit 0 | raw/chain-cognitive-task-bounds-regression.jsonl |
| Actual large-source flow before packet repair | RED, reflect failure, unresolved window | raw/chain-reflection-large-source-observation.jsonl |
| Library packet bound | RED, 10,006 bytes | raw/chain-reflection-packet-red.jsonl |
| Full Laputa | 64 pass / 0 skip / exit 0 | raw/chain-laputa-packet-regression.jsonl |
| Actual large-source flow | GREEN / exit 0 | raw/chain-reflection-large-source-green.jsonl |
| Pressure fixture after compaction | RED, old fixture now under its ceiling | raw/chain-cognitive-packet-runtime-observation.jsonl |
| Final focused runtime | 75 pass / 0 skip / exit 0 | raw/chain-cognitive-packet-runtime-regression.jsonl |
| Final selected composition | 11 pass / 0 skip / exit 0 | raw/chain-bounded-composition-regression.jsonl |
| Full Garden | 480 pass / 0 skip / exit 0 | raw/chain-garden-packet-regression.jsonl |

Use the v0.1.0 verification commands with `TestMemoryLoopAutomaticReflectionLargeSource` added to the anchored composition selection and `Test.*OneShot` added to runtime selection. Also run `go test -json ./... -count=1` from the paired Laputa and Garden module roots, using the same task environment and required loopback permission.

The previous overlay selects real generated App composition while compiling current source, but carries old artifact/source attestation. It remains diagnostic. No prior full-CI result, different native generation or historical acceptance record is promoted to attest the new tree. Final frozen-source gates are pending.
