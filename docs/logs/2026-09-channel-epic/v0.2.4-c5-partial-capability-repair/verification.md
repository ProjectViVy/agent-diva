# Verification

## Planned gates

The following commands are required after the channel agents and Lead integration finish:

```text
cargo test -p agent-diva-channels --all-targets
cargo clippy -p agent-diva-channels --all-targets -- -D warnings
just msrv-probe check -p agent-diva-channels
just fmt-check
just check
just test
git diff --check
```

The ignored QQ live harness must be attempted only with credentials supplied outside the repository. Missing credentials or platform permission are recorded as an external block, not simulated as success.

## Evidence checks

- Capability snapshot remains equal to the frozen matrix.
- Every unsupported capability is rejected before transport invocation.
- Admission precedes deduplication and media transport.
- Attachment references are validated, stored, read back, and corruption-checked.
- Retry-After, cancellation, stop, reconnect, health, partial failure, and sensitive-log redaction are covered by deterministic tests where the platform supports them.

## Result

Pending implementation and execution. Failures, pre-existing debt, and external QQ blocks will be recorded here before release.
