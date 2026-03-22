# Verification

## Commands

- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/publish-nano-stack.ps1 -Mode package`
- `just package-nano-stack`
- `cargo check -p agent-diva-nano`

## Results

- `cargo check -p agent-diva-nano`: passed
- `scripts/publish-nano-stack.ps1 -Mode package`: passed for `agent-diva-core`, then stopped at `agent-diva-providers` with a clear dependency-order message
- `just package-nano-stack`: same behavior and non-zero exit as expected

## Interpretation

- The helper now correctly models the publish dependency chain.
- The next external prerequisite is not code decoupling, but actual topo-ordered crates.io publication of upstream internal crates.
