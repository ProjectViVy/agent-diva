# 普通聊天 update_plan TODO 清单 — 验收记录

## 验收目标

普通聊天中，当用户请求复杂/多步骤任务时，agent-diva 能在消息流中展示一个只读的 TODO 清单，并随着模型调用 `update_plan` 更新清单状态。

## 验收步骤（用户/产品视角）

### 1. TUI 路径

1. 启动本地 gateway：`agent-diva gateway run`（或 TUI 自动启动内置 gateway）。
2. 启动 TUI：`agent-diva tui`。
3. 发送消息：`“帮我列出完成这个功能的步骤”`。
4. 观察消息流中是否出现 `[plan]` 标签的清单，包含 `☐` / `→` / `✓` 状态图标。

### 2. GUI 路径

1. 启动 GUI：`cd agent-diva-gui && pnpm tauri dev`。
2. 在聊天窗口发送：`“帮我列出完成这个功能的步骤”`。
3. 观察聊天流中是否出现 `📋 计划更新` 卡片，包含 Pending / InProgress / Completed 状态。
4. 确认卡片为只读，不能手动勾选。

### 3. 回归检查

- 进入 Plan 模式后，确认工具列表中**没有** `update_plan`。
- 确认现有 Plan Execution 的 TODO 卡片功能未被破坏。
- 确认发送普通消息后，不会意外产生持久化的 TODO 条目。

## 实际执行情况

- 已验证 CLI 二进制可编译启动（`--help`、各子命令帮助）。
- 已用 `agent-diva-cli/tests/update_plan_e2e.rs` 验证 CLI `ApiClient` → Manager → SSE → Client 的完整事件流。
- 未在真实 TUI/GUI 中通过真实/mock provider 触发，因为：
  - 当前没有可用的 mock provider。
  - 真实 provider 需要配置 API key，本次环境未提供。
  - GUI/TUI 的渲染分支已通过组件/单元测试覆盖。

## 阻塞项（已清理）

- ~~`cargo test --workspace update_plan` 因其它 crate 脏改动无法通过~~ — 已修复。
- ~~`cargo clippy --workspace -D warnings` 因 `agent-diva-core/src/planning/report.rs` 的预存错误无法通过~~ — 已修复。
- 真实端到端 CLI/GUI smoke test 需 mock provider 或真实 API key — 待后续补充。

## 结论

- 功能实现与核心路径测试已完成。
- `cargo test --workspace update_plan`、`cargo clippy --workspace -D warnings`、`cargo fmt --check` 均已通过。
- 建议：在真实或 mock provider 环境下完成一次 GUI/TUI 端到端验收。
