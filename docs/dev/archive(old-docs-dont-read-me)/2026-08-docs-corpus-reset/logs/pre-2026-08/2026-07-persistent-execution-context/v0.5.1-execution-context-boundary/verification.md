# Verification

- `cargo test -p agent-diva-core execution_context_is_revision_isolated_and_restart_safe`: passed.
- `cargo test -p agent-diva-manager planning_service::tests::report_projection_uses_canonical_store_for_revision_approval`: passed.
- `cargo test -p agent-diva-cli --test config_commands provider_set_json_updates_model_and_credentials`: passed.
- `just fmt-check`: passed.
- `just check`: passed.
- `pnpm build` in `agent-diva-gui`: passed.
- `just test`: reached 630 passing Core tests and failed on two pre-existing supervised-executor race assertions. Both are recorded in `TODOLIST.md`; the planning revision regressions exposed during this run were fixed and rechecked.

No external provider or network-backed smoke test was run because it would require user credentials. The local approval/persistence path is covered by Manager and Core tests.
