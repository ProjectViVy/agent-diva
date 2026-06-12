# Verification

## Commands

- `cargo check -p agent-diva-nano`
- `cargo check -p agent-diva-cli --no-default-features --features nano`
- `cargo check -p agent-diva-cli --features full`
- `cargo test -p agent-diva-nano`
- `cargo package -p agent-diva-core --allow-dirty --no-verify`
- `cargo package -p agent-diva-nano --allow-dirty --no-verify`
- `cargo package -p agent-diva-cli --allow-dirty --no-verify`

## Results

- `cargo check -p agent-diva-nano`: passed
- `cargo check -p agent-diva-cli --no-default-features --features nano`: passed
- `cargo check -p agent-diva-cli --features full`: passed
- `cargo test -p agent-diva-nano`: passed
- `cargo package -p agent-diva-core --allow-dirty --no-verify`: passed
- `cargo package -p agent-diva-nano --allow-dirty --no-verify`: failed because upstream internal crates are not yet published on crates.io
- `cargo package -p agent-diva-cli --allow-dirty --no-verify`: failed for the same reason

## Interpretation

- The remaining `cargo package` failures are expected at this stage.
- They confirm that the next step is release sequencing, not more source decoupling for `agent-diva-nano`.
- Once the internal dependency chain is actually published in topo order, the higher-level crates can be packaged and published normally.
