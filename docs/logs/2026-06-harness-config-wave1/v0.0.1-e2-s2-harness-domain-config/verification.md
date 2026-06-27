# Verification

- Ran `cargo test -p agent-diva-core config::loader::tests::test_load_migrates_legacy_config_to_current_version_and_persists -- --nocapture`
- Ran `cargo test -p agent-diva-cli --test config_commands -- --nocapture`
- Ran `cargo check -p agent-diva-manager -p agent-diva-core -p agent-diva-cli`
- Captured CLI evidence:
  - `.omo/ulw-loop/evidence/wave1-e2-s2-c001-cli-config-show.txt`
  - `.omo/ulw-loop/evidence/wave1-e2-s2-c002-invalid-config.txt`
  - `.omo/ulw-loop/evidence/wave1-e2-s2-c003-legacy-config-migration.txt`

# Result

- All targeted Rust tests passed, including the new harness-config CLI checks and legacy migration coverage.
- The manager/core/CLI crates passed `cargo check`.
- CLI evidence proves the happy path, invalid-config rejection, and legacy-config migration/regression behavior on the built binary.
- Full `just fmt-check && just check && just test` was not re-run in this turn; targeted crate-level verification was used for the touched surface.
