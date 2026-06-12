# Verification

- No code validation command was required for this iteration because the change only touched Markdown documentation.
- The guide content was cross-checked against:
  - `cargo run -p agent-diva-cli -- --help`
  - `cargo run -p agent-diva-cli -- agent --help`
  - `cargo run -p agent-diva-cli -- chat --help`
  - `cargo run -p agent-diva-cli -- config --help`
  - `cargo run -p agent-diva-cli -- provider --help`
  - `cargo run -p agent-diva-cli -- channels --help`
  - `cargo run -p agent-diva-cli -- cron --help`
  - `cargo run -p agent-diva-cli -- service --help`

Result: documentation examples and command descriptions were aligned with the current CLI surface.
