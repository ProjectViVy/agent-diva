# 验证

已通过：

- `cargo test -p agent-diva-autodream`
  - 39 个测试通过。
  - 覆盖 queued 记录、stale lock 重排、legacy incomplete fail closed、取消、超时、
    阶段终态、publishing 恢复与确定性 proposal replay。
- `cargo test -p agent-diva-manager autodream --lib`
  - 2 个 HTTP/后台执行测试通过。
  - 覆盖请求立即返回 queued，随后分别进入稳定失败与成功发布终态。
- `cargo fmt --all -- --check`：通过（格式化后复验）。

本切片完成后继续执行 `just fmt-check` 与 `just check`。完整 `just test` 的已知
Windows 桌面二进制占用和 GUI PDB 链接限制保留在 E7 发布门统一复验，不能以
`cargo check` 替代。按用户要求，本阶段不执行真实桌面人工测试。
