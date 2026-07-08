# Acceptance

1. Build or run the default CLI path, for example `cargo run --package agent-diva-cli -- gateway run` or `just diva-gate`.
2. Confirm the resulting binary is Mentle-capable without adding `--features mentle` manually.
3. Enable Mentle in `~/.agent-diva/config.json` or the GUI settings and verify the runtime no longer reports the `agent-diva-agent \`mentle\` feature is disabled` fallback.
