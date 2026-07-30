# Verification

- `cargo test -p agent-diva-core workspace_identity --lib`: passed.
- `cargo test -p agent-diva-laputa --test typed_store canonical_open_migrates_only_workspace_identity_and_keeps_verified_backup -- --exact`: passed.
- `cargo test -p agent-diva-migration workspace_identity::tests::identity_dry_run_apply_and_rollback_are_explicit_and_reversible -- --exact`: passed.
- `cargo check -p agent-diva-agent -p agent-diva-manager`: passed.
- `cargo run -q -p agent-diva-migration -- memory identity --help`: passed.

The migration test verifies the backup, manifest phases, canonical scope,
unchanged content and digest, unchanged record/store revisions, integrity, and
manifest rollback to the legacy identity.

No desktop key, external API, or real desktop validation was used.
