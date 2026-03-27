# Release

## Method

- No packaging or deployment was performed in this iteration.
- This change is a source-level foundation update for memory and diary behavior inside the Rust workspace.

## Preconditions For Release

- Merge the source changes into the target branch
- Run the same workspace validation commands in CI or the destination environment
- If the runtime should start using rational diary persistence immediately, deploy the updated binaries normally; no data migration is required

## Rollback

- Revert the source changes in this iteration
- Existing `MEMORY.md` and `HISTORY.md` behavior will continue to work because the new diary path is additive
