# Release

## Status

- No release or publication was performed in this iteration.

## Reason

- This round only adds planning and execution-checklist documentation.
- The document is intended to guide a future crates.io publication flow for the nano dependency closure.

## Future Release Path

1. Validate each publish candidate crate with `cargo package --dry-run`.
2. Publish the current minimum closure in dependency order.
3. Wait for crates.io index visibility between each dependent layer.
4. Switch `agent-diva-nano` from `path` dependencies to version dependencies.
5. Re-validate `agent-diva-nano` from a repository-external environment.
