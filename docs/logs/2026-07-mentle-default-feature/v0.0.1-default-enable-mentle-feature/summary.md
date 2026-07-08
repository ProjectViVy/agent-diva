# Summary

- Enabled the `agent-diva-agent` `mentle` feature by default along the primary binary startup path.
- Updated `agent-diva-manager` and `agent-diva-cli` dependency declarations so ordinary CLI and gateway builds now compile Mentle support without requiring an explicit feature flag.
- Preserved the existing optional feature definition inside `agent-diva-agent`; this change only makes the shipped binaries opt into it by default.

# Impact

- `cargo run --package agent-diva-cli -- ...`
- `just run -- ...`
- `just diva-gate`
- `just start`

These entry points now build Mentle-capable binaries by default.
