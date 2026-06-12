# Acceptance

1. `agent-diva-manager/src/manager.rs` 主循环不再内联全部控制面逻辑，而是把 runtime 与 companion 管理能力分发到私有子模块。
2. `agent-diva-manager/src/handlers.rs` 中 provider 的新增、更新、删除、model CRUD、resolve 不再直接写配置，而是通过 `ManagerCommand` 转给 manager。
3. `agent-diva-manager/src/server.rs` 明确区分 runtime routes 与 companion routes，但所有既有 `/api/*` 路径保持不变。
4. `cargo check -p agent-diva-manager`、`cargo check -p agent-diva-cli`、`cargo check -p agent-diva-cli --no-default-features --features nano`、`just test` 均通过。
