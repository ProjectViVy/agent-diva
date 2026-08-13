# Verification

## Commands

- `cargo test -p agent-diva-e2e -- --nocapture`
- `just e2e-test`

## Results

- `cargo test -p agent-diva-e2e -- --nocapture` now resolves from the root workspace and executes the crate tests instead of failing with a workspace-membership error.
- Real-provider scenario tests skip cleanly without failure when `DEEPSEEK_API_KEY` / `E2E_API_KEY` is absent.
- `just e2e-test` resolves to the same explicit E2E lane from the workspace root.

## Deferred Validation

- No live provider-backed scenario was executed in this environment because no real E2E API key was present during validation.
