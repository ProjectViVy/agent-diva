# Story 5.2 Mentle Governance Exclusion Acceptance

## 验收步骤

1. 运行 AutoDream 相关测试，确认输出产物、提案和报告写入不会创建 `memory/palace.db` 或 `.mentle`。
2. 运行 Laputa 相关测试，确认 proposal create/apply/audit/rollback/event 路径保持 file-first，且不要求 Mentle 运行态。
3. 运行 `agent-diva-agent` 的 Mentle 相关测试，确认默认治理 prompt 不注入 Mentle recall 或 `memtle_*` 路由。
4. 运行 `cargo check -p agent-diva-manager`，确认管理面兼容层仍可编译。

## 当前结果

已验收，Story 5.2 达到开发完成并进入 `review` 状态。
