# 验证记录

## 工作区门禁

- `just fmt-check` ✅（先 `just fmt` 后通过）
- `just check`（`cargo clippy --all -- -D warnings`）✅
- `cargo check --workspace --all-targets` ✅

## 受影响 crate 测试

| crate | 结果 |
|---|---|
| agent-diva-sandbox | ✅ 119 passed |
| agent-diva-tools | ✅ 89 passed |
| agent-diva-agent | ✅ 365 passed |
| agent-diva-manager | ✅ 15 passed |
| agent-diva-core | ✅ 132 passed |
| agent-diva-files | ✅ 11 passed |
| agent-diva-channels | ✅ 6 passed |

## 新增回归用例

- `agent_diva_sandbox::orchestrator::tests::should_offer_escalation_covers_execution_failed`：断言 `ExecutionFailed { code: 1, .. }` 进入 escalation 白名单；
- `agent_diva_sandbox::orchestrator::tests::on_request_allows_sandbox_failure_retry`：断言 `OnRequest` 策略允许沙箱失败后重试；
- `agent_diva_manager::handlers::tests::parse_approval_policy_maps_gui_modes_and_canonical_aliases`：覆盖 cautious/smart/trusted/on-request/On_Failure/never/None/""/garbage。

## GUI 烟雾测试（deferred）

完整端到端（启动 Tauri dev → 切谨慎模式 → 触发 exec → 观察 drawer 自动弹出 pending → Allow → 文件夹创建）需要 `pnpm install` + `pnpm tauri dev`，本次未运行。建议在合并前由人工按 `acceptance.md` 步骤执行。

## 未验证 / 已知限制

- Approval coordinator 超时时长仍为默认值，前端未显示倒计时；
- Legacy `command-approval-requested` 与统一 `approval-event` 两套通道并存的技术债未清理（已记录 `TODOLIST.md`）；
- 同命令同会话下第二次执行会命中 `ApprovedForSession` 缓存，谨慎模式下不再询问，符合现有 UX 契约。
