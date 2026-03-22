# Summary

- 本轮将 `agent-diva-manager` 的运行期控制面按职责分成两层：正式 CLI 默认依赖的 runtime 控制面，以及 GUI/HTTP companion 管理面。
- `agent-diva-manager/src/manager.rs` 的主循环改为轻量分发，具体逻辑下沉到 `src/manager/runtime_control.rs` 和 `src/manager/companion_admin.rs`。
- `agent-diva-manager/src/handlers.rs` 将 provider 写路径收口到 `ManagerCommand`，不再直接负责 provider 配置写盘。
- `agent-diva-manager/src/server.rs` 保持现有路由不变，但按 runtime routes、companion routes、misc routes 分组构建。

# Impact

- 保持 `/api/chat`、`/api/chat/stop`、`/api/events`、`sessions`、`config`、`channels`、`tools`、`cron`、`providers`、`skills`、`mcps` 的既有路径与响应契约不变。
- 为第三轮继续处理 GUI 编译依赖与发布闭包准备出更清晰的内部边界。
