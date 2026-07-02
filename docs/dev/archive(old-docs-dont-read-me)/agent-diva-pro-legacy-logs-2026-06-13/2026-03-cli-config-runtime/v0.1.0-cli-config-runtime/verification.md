# Verification

- `cargo fmt --all`
- `just fmt-check`
- `just check`
- `just test`
- `cargo test -p agent-diva-cli --test config_commands -- --nocapture`
- `cargo run -p agent-diva-cli -- --config <temp-config> status --json`
- `cargo run -p agent-diva-cli -- --config <temp-config> config doctor --json`
- `cargo run -p agent-diva-cli -- --help`
- `cargo run -p agent-diva-cli -- config --help`

# Result

- `cargo fmt --all`: passed
- `just fmt-check`: passed
- `just check`: passed after adding the missing `mcp_manager` field in `agent-diva-migration` and removing one unused test import in `agent-diva-agent`.
- `just test`: passed
- `cargo test -p agent-diva-cli --test config_commands -- --nocapture`: passed
- Structured-output commands were verified to emit clean JSON without ASCII logo contamination.
- Explicit `--config` path routing was verified through integration tests.

# Notes

- Existing workspace-wide warnings about future incompatibility in `imap-proto v0.10.2` remain unrelated to this iteration.
