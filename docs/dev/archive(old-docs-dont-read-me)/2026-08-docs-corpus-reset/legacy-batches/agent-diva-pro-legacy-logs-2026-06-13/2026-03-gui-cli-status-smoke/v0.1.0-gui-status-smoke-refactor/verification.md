# Verification

- `cargo check -p agent-diva-cli`
- `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml`
- `npm run build` in `agent-diva-gui`
- `cargo test -p agent-diva-cli --test config_commands`
- `cargo test -p agent-diva-cli --test direct_chat_smoke`
- `just fmt-check`
- `just check`
- `cargo test --all`

Result: all commands completed successfully.
