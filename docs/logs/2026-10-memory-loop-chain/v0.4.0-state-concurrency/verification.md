# Verification

| Check | Result | Log |
|---|---|---|
| Two concurrent policy writers, accepted source preservation | 2 RED failures | raw/chain-cognitive-concurrency-red.jsonl |
| Admitted run/window preservation | 1 RED failure | raw/chain-cognitive-admission-overwrite-red.jsonl |
| Three policy/input/admission cases ×10 |30 pass /0 skip /exit0|raw/chain-cognitive-concurrency-regression.jsonl|
| Four concurrency cases -race ×3 |12 pass /0 skip /exit0|raw/chain-cognitive-concurrency-race-regression.jsonl|
| Focused runtime |85 pass /0 skip /exit0|raw/chain-concurrency-final-runtime.jsonl|
| Actual diagnostic composition |12 pass /0 skip /exit0|raw/chain-concurrency-final-composition.jsonl|

Run from VIVY with the task-local environment and local loopback permission. Race selection: `go test -race -json ./internal/runtime -run '^TestCognitive(PolicyCASHasOneConcurrentWinner|PolicyWritePreservesAcceptedSourceHigh|PolicyWritePreservesAdmittedWindow|ConcurrentManualAndAutomaticWakesCoalesce)$' -count=3`. Runtime and composition commands follow the v0.3 checkpoint.

The 200ms controller scheduling aid is not a crash handshake. All assertions inspect actual SQLite state/native admissions; scripted Domain outcomes do not claim positive backend acceptance. Final required just ci/conformance reproduction/regenerated pack/Inspect/native identity/fresh review remain pending. The old overlay cannot attest this new source.
