# G2D Automated E2E Verification

## 定向测试

- `cargo test -p agent-diva-manager --test autodream_laputa_e2e`：6 passed。
- 首轮测试发现 apply 使用审批前 version 的测试契约错误，修正为使用审批后的
  governance version；编辑场景改为验证旧 request 已被撤销。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`：通过，Clippy warnings denied。
- 首次 `just test`：唯一失败为既有
  `agent-diva-laputa::storage::lock_recovers_stale_lock_file` 的 Windows 锁回收时序
  波动；定向重跑通过。
- `cargo test -p agent-diva-laputa --test storage lock_recovers_stale_lock_file -- --nocapture`：通过。
- `just ci`：通过，包含全量测试、feature-gate、BML boundary 与 Laputa clean-break。

## 数据安全观察

- 测试使用 tempfile workspace 与 deterministic reflection engine。
- Recall feedback 断言不包含记忆原文。
- provider unavailable 与 prompt injection 场景均不产生 proposal/apply 副作用。
