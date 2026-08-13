# Story 6.2 Release Notes

## Release Method

- Ship as a normal Rust workspace update.
- No data migration is required for existing `.laputa` workspaces.
- No operator configuration changes are required.

## Rollback

- Revert the Story 6.2 commit to restore previous Laputa recovery and error-code behavior.
- Existing `.laputa` data remains file-compatible because the change adds recovery behavior and error codes without changing stored record schemas.
