# Verification

- Planned commands:
  - `cargo test -p agent-diva-core config::loader::tests::test_load_allows_invalid_enabled_channel_config`
  - `cargo test -p agent-diva-channels manager::tests`
  - `cargo test -p agent-diva-manager runtime::task_runtime::tests`
  - `cargo fmt --all -- --check`
- Expected result:
  - Invalid enabled channel configs no longer fail config loading.
  - Runnable channel selection skips invalid enabled channels.
  - Formatting remains clean.

# Notes

- Full workspace validation can be run afterward with `just fmt-check && just check && just test` if broader confirmation is needed.
