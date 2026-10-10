# Actual model request and response evidence

This increment records the complete HTTP request/response boundary for the
local memory-loop App harness. It adds status, headers, raw streamed or plain
response bodies, handler errors, finish times, and request association to the
existing provider evidence. A >4 KiB Unicode response was captured byte for
byte. The evidence explicitly distinguishes a completed HTTP handler from
client-side response consumption; consumption is not claimed.

The focused App race run passed three named cases, zero skips, exit 0, in
117.514 seconds:

- interrupted inference and process restart retained the original window;
- an actual large model response was preserved exactly;
- a real protected Tool call and assistant output did not become trusted user
  provenance.

The observation bundle contains 43 files with verified size and SHA-256
records. Its manifests are `developer-observation`, have
`formal_acceptance: false`, and leave `candidate_id` null. This records local
development evidence only; it does not attest a frozen SDK candidate or prove
provider-side/client consumption.

The exact race log is in `raw/chain-response-evidence-first-race.jsonl`; the
five captured observation directories are under `observations/`. The VIVY
change was committed as `ab16de8d` before the next local increment.
