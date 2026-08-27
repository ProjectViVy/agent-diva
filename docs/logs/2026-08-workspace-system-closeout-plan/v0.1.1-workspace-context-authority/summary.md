# WS-00：WorkspaceContext 权威链路

完成 workspace-agents Wave A–D 的提交级重放，并修正运行时到 Manager 的权威上下文传递：

- `GatewayRuntimeConfig`、`GatewayBootstrap` 和生产 `AppState` 持有同一 `WorkspaceContext`。
- CLI 与 Tauri 构造 Gateway 时传入完整 context，而不是只传 root `PathBuf`。
- `/api/workspace` 从 `AppState.workspace_context` 投影 root/source；删除根据路径猜 source 的
  fallback。
- `workspace_root` 暂作为现有 handler 的兼容投影，构造时由 context root 派生。
- 旧隔离分支未被续写；本次工作位于 `feat/workspace-system-closeout`。
