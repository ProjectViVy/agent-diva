# 验证记录

- `cargo fmt --all`：通过。
- `cargo test -p agent-diva-core session::store::tests::test_chat_message_plan_metadata_roundtrips_and_is_optional`：通过。
- `cargo test -p agent-diva-agent --lib`：335 个测试通过。
- `cargo check -p agent-diva-manager -p agent-diva-cli`：通过。
- `pnpm exec vue-tsc --noEmit`（在 `agent-diva-gui`）：通过。
- `pnpm test`（在 `agent-diva-gui`）：43 个测试文件、388 个测试通过。
- `PlanHistoryCard` 与 `PlanApprovalCard` 定向测试：3 个测试通过。
