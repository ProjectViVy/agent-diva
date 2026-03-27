# Verification

- `cargo test -p agent-diva-cli --test config_commands provider_list_json_includes_registry_default_model`: passed.
- `cargo test -p agent-diva-cli --test config_commands`: passed.
- `cargo run -p agent-diva-cli -- --config <temp>/instance/config.json provider list --json`: passed.
- `cargo test --all`: passed.

## Additional notes

- The CLI smoke run confirmed `provider list --json` now emits both `name` and `provider` for the same provider identifier.
- Full workspace regression completed successfully after the fix.
