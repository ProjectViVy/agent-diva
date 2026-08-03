# Verification

## Focused automated evidence

- `cargo test -p agent-diva-cli` — passed.
- `cargo clippy -p agent-diva-cli --all-targets -- -D warnings` — passed.
- CLI library coverage proves all three domains parse and resolve through the
  same version/idempotency contract, empty input is never interpreted as allow,
  explicit Plan queue returns promptly, high-risk Memory defaults to cancel,
  and Command queue fails because its payload is intentionally non-durable.
- Binary smoke coverage proves local queue-unavailable and remote transport
  failure emit stable JSON with a non-zero exit status; normal direct-agent
  behavior and existing CLI commands remain green.

## Final gates

- `just fmt-check` — passed.
- `just check` — passed; only the existing `imap-proto v0.10.2`
  future-incompatibility advisory was emitted.
- `just test` — passed; the two previously recorded unused-variable warnings in
  test-only channel/tool fixtures remain unchanged.

The single human CLI/desktop smoke remains intentionally deferred until the
complete M3 release candidate is ready.
