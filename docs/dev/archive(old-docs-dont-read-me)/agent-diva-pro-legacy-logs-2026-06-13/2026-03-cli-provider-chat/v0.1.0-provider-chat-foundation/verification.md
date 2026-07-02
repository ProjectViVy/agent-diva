# Verification

- `just fmt-check`
  - Passed.
- `just check`
  - Passed.
- `just test`
  - Passed.
- `cargo test -p agent-diva-cli`
  - Passed.

Additional coverage added:

- CLI integration tests for `provider list --json`.
- CLI integration tests for `provider set --json`.
- Unit tests for provider default-model fallback behavior when registry metadata is missing.
