# 验证

已通过：

- `cargo test -p agent-diva-laputa`
  - 84 个测试通过，2 个显式性能测试保持 ignored。
  - 新覆盖 suppression 持久化、无 payload、精确匹配、显著变化、过期、物理容量、
    损坏文件 fail closed，以及 proposal 编辑后的旧授权失效。
- `cargo test -p agent-diva-autodream`
  - 48 个测试通过。
  - 新覆盖拒绝候选不会在下一 run 原样生成，且 rejection reason 可审计。
- `cargo test -p agent-diva-manager proposal_decision_retry_finishes_transition_after_decision_crash_window --lib`
  - 验证 Ledger 决策后、proposal transition 前崩溃的补偿与 replay。
- `cargo clippy -p agent-diva-core -p agent-diva-laputa -p agent-diva-autodream -p agent-diva-manager -- -D warnings`
  - 通过。

提交前继续执行 `just fmt-check` 与 `just check`。完整 `just test` 保留给 E7 解决既有
Windows GUI 链接门。本切片没有真实外部 API、桌面密钥或人工测试。
