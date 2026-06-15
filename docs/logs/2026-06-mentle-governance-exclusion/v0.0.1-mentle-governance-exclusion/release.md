# Story 5.2 Mentle Governance Exclusion Release

## Release Status

Not released.

## Reason

The guardrail changes are implemented, but the story remains `in-progress` because required full validation is blocked by unrelated current AutoDream and Laputa test failures.

## Release Criteria

- Resolve the validation blockers recorded in `TODOLIST.md`.
- Re-run story-required validation:
  - `cargo test -p agent-diva-agent mentle`
  - `cargo test -p agent-diva-autodream`
  - `cargo test -p agent-diva-laputa`
  - `cargo check -p agent-diva-manager`
- Move story status to `review` only after all required validation passes.
