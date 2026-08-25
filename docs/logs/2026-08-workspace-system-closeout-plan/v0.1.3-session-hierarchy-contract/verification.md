# Verification

通过：

- `cargo fmt --all -- --check`
- `cargo test -p agent-diva-core session::manager --lib`（16 passed）
- `cargo test -p agent-diva-manager handlers::tests::get_sessions_response_includes_title --lib`（1 passed）
- `cargo check -p agent-diva-autodream`

核心测试覆盖新 root 的 workspace identity、显式 branch 的 root/parent/label，以及旧 JSONL
作为独立 legacy root 的兼容投影。
