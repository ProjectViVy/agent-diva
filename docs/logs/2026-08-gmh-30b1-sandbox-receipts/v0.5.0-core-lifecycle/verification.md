# Verification

- `cargo test -p agent-diva-core governance::ledger::tests::state_pages_use_stable_request_id_cursor_and_replay_time`
- `cargo test -p agent-diva-core governance::coordinator`

Both focused suites passed. Existing unrelated supervised-store test warnings
remain outside this slice.
