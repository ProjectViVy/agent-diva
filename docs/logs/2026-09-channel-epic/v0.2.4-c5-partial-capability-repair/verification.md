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

### Static evidence

- The pinned Octos checkout is `5ea987813de4fd2afdd1d78f2106ad2868f0d923`.
- `capability-evidence.json` parses with 29 unique rows: `11 verified / 17 partial /
  1 blocked/unsupported`; all 29 IDs are present in `evidence-manifest.md`.
- The frozen capability snapshot test and the zero-transport unsupported-command TCK pass.
- Attachment MIME/size/name/digest/readback/corruption checks and admission-before-dedup/media
  ordering checks pass in the shared and channel suites.

### Commands and results

| Command | Result |
| --- | --- |
| `cargo test -p agent-diva-channels --all-targets` | PASS: 215 library, 12 runtime TCK, 10 shared TCK, 5 characterization, 6 QQ reconnect; 1 live test ignored by design |
| `cargo clippy -p agent-diva-channels --all-targets -- -D warnings` | PASS after `9579e577` removed identical QQ opcode branches and shortened a DingTalk test mutex guard |
| `just msrv-probe check -p agent-diva-channels` | PASS with Rust 1.80.1 |
| `just fmt-check` | PASS |
| `just check` | PASS |
| `just test` | PASS; workspace tests and doctests completed without failure |
| `git diff --check` | PASS |

The ignored live command was executed as:

```text
cargo test -p agent-diva-channels --test qq_live_harness -- --ignored --nocapture
```

It returned the explicit blocked result `C5-Q blocked: missing AGENT_DIVA_LIVE_QQ_APP_ID` and
performed no network access. This is not a live smoke pass. D-013 official QQ event-delivery
proof, D-014 media wire proof, and live/platform/supervisor evidence for the remaining partial
rows remain open. The `imap-proto` future-incompatibility warning and unrelated existing
workspace TODOs are recorded but did not fail the listed gates.
