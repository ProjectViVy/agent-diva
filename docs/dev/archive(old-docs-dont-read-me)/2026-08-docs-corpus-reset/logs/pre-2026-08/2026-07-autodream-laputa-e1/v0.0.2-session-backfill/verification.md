# 验证

- `cargo test -p agent-diva-migration`：覆盖 dry-run 无写入、apply、幂等 replay、
  payload-free 文件、缺失 tool identity 拒绝、回滚和保留预存 evidence。
- `cargo clippy -p agent-diva-core -p agent-diva-migration -- -D warnings`：通过。
- `cargo run -q -p agent-diva-migration -- experience --help`：真实 CLI smoke 通过，
  显示三个显式操作。
- `just fmt-check`：通过。
- `just check`：通过。

完整工作区门沿用 E1A/E0 已记录的 Windows 桌面二进制锁与 GUI PDB 链接限制，E7
必须在发布环境重新跑通。无人工桌面步骤。
