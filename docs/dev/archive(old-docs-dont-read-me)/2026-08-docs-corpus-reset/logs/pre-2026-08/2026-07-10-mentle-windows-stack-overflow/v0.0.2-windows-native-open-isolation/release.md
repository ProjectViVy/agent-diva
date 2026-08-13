# Release

No special packaging step. Ship with the normal `agent-diva-cli` binary build.

- Debug/release: `cargo build -p agent-diva-cli` / `cargo build -p agent-diva-cli --release`
- Windows MSVC builds pick up `/STACK:16777216` from `agent-diva-cli/build.rs`
- Operators do not need environment variables for the common path

Rollback: revert this iteration and restore the temporary Windows platform disable only if a new native abort reappears on unsupported hosts.
