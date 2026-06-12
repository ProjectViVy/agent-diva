# Summary

- 本轮进入第三轮正式开发，目标是收口 GUI 对 manager companion 面的消费路径，并把 GUI/CLI 的编译依赖边界表达清楚。
- `agent-diva-gui/src-tauri/src/commands.rs` 中的 provider 管理命令已切换为通过 `/api/providers*` companion API 工作，不再由 GUI 直接读写 provider 配置。
- GUI 仍保留本地宿主管理桥接职责，包括 gateway 进程管理、service 管理、日志读取、本地配置状态和数据清理；这些能力没有迁回 manager。
- `agent-diva-gui/src-tauri/src/app_state.rs` 新增 provider companion API 访问辅助，统一了 GUI 对 `/api/providers` 和 `/api/providers/:name/models` 的调用入口。
- `agent-diva-cli/src/main.rs` 与相关 `Cargo.toml` 补充了 feature/依赖说明，明确 `full` 和 `nano` 仅在本地 `gateway run` 路径上切换运行时实现，GUI 依赖 CLI 仅用于本地 runtime/status 辅助能力。

# Impact

- GUI 当前 companion 面依赖已明确聚焦为 `providers`、`skills`、`mcps`、`events`，并继续复用少量 runtime 面能力如 `chat`、`sessions`、`config`、`channels`、`tools`、`cron`。
- 未修改 `/api/chat`、`/api/chat/stop`、SSE event name/payload、`agent-diva-cli --remote` 契约或 `full/nano` feature 语义。
- 为后续继续裁剪 GUI 对 CLI 的编译依赖准备了边界条件，但本轮没有引入新的共享 crate，也没有改变发布脚本承诺。
