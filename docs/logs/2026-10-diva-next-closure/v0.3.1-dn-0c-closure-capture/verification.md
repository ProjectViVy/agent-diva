# Verification — v0.3.1 DN-0C

## Commands run (Linux, Go 1.26.4)

```bash
cd agent-vivy
DN0_CAPTURE_OUT=/tmp/diva-closure-chat-obs.json \
  go test ./internal/app -run TestDN0CaptureClosureTranscript -count=1 -v
# => PASS, 110 request/response records captured, 25/25 assertions ok

go test ./internal/app ./internal/rpc -count=1
# => ok agent-vivy/internal/app 8.0s; ok agent-vivy/internal/rpc 19.9s

python3 -m json.tool docs/plans/diva-next/fixtures/closure-chat-obs.json
# => valid JSON
```

## Assertions embedded in the fixture (all `ok: true`)

- attachment: bad-base64 → -32602; unsupported MIME → -32602;
  bytes round-trip via `data_url` byte-identical to sent PNG.
- permission: preset enum enforced (-32602 on `bogus`).
- turn: malformed params (-32602).
- rewind: busy → -32009; inclusive cutoff leaves `remaining_count: 0`;
  invalid cutoff → -32004.
- cancel: terminal run → -32004.
- edit: run admitted (`status: accepted`), suffix replaced, missing message
  → -32603.
- fork: inclusive copy (`copied_count: 1`).
- stats: `projection_version == 2`, coverage shape, invalid period → -32602.
- trajectory: `projection_version == 2`, safe-integer watermarks, unknown
  session → empty projection (no error).
- diagnostics: gui round-trip, invalid source → -32602, oversized batch
  (>500) → -32602.
- children: `child/list` shape, missing run → -32004.

## Redaction check

`grep -i "facehost-test-key|api_key|authorization|bearer|reasoning|thinking|audio|sk-"`
→ only `total_reasoning: 0` (a stats counter). No credentials, audio bytes,
or raw-thinking sentinels present.

## Environment note

`ui/dist` is gitignored; a fresh checkout needs `ui/dist/.keep` (or the Vite
build) for `go:embed all:dist`. Created locally, not committed.
