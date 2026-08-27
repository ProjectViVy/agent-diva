# Verification

通过：

- `cargo fmt --all -- --check`
- `cargo check -p agent-diva-manager -p agent-diva-cli`
- `cargo check --manifest-path agent-diva-gui/src-tauri/Cargo.toml`
- `cargo test -p agent-diva-manager handlers::workspace --lib`（4 passed）
- `cargo test -p agent-diva-cli --test effective_workspace`（6 passed）
- `git diff --check`

新增 Manager 测试验证 `ProcessCwd` source 从 `WorkspaceContext` 原样投影，而不是根据 root
路径反推。
