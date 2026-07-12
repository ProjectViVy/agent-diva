# 验证

- `cargo test -p agent-diva-manager token_stat_routes -- --nocapture`：通过（2 项）。
- `npm test -- --run src/api/tokenStats.test.ts`：通过（2 项）。
- `npm run build`（`agent-diva-gui`）：通过。

已验证 ledger 聚合、非法查询、Tauri DTO 解析和 GUI 生产构建。
