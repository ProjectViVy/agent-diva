# 普通聊天 update_plan TODO 清单 — 迭代摘要

## 变更范围

参考 OpenAI Codex 的 `update_plan` 工具，为 agent-diva 的普通聊天（非 Plan 模式）引入 per-turn、内存级的 TODO 清单能力。

## 主要改动

- **类型与事件** (`agent-diva-core`)
  - 新增 `UpdatePlanArgs` / `PlanItem` / `PlanItemStatus`（`Pending` / `InProgress` / `Completed`）。
  - 新增 `AgentEvent::ChatPlanUpdate { args }` 事件变体。
  - 常量限制：`MAX_UPDATE_PLAN_ITEMS = 20`。

- **工具** (`agent-diva-tools`)
  - 新增 `UpdatePlanTool`，`name = "update_plan"`。
  - 参数校验：非空 plan、不超过 20 项、合法 status、step / explanation 长度限制。
  - 工具仅发出事件，不写入任何持久化存储。

- **注册与系统提示词** (`agent-diva-agent`)
  - `tool_assembly.rs` 仅在普通聊天（`plan_phase.is_none()` 且 `execution_session_id.is_none()`）注册 `update_plan`。
  - Plan 模式与 Execution 模式均不注册。
  - `context.rs` 系统提示词新增 `update_plan` 使用说明与状态集合。

- **事件转发** (`agent-diva-manager`)
  - `handlers.rs` 将 `AgentEvent::ChatPlanUpdate` 映射为 SSE `turn_plan_updated`。
  - 同时支持 `chat_handler` 直接事件流和 `events_handler` 总线事件流。

- **客户端渲染**
  - TUI (`agent-diva-cli/src/main.rs`)：新增 `TimelineKind::PlanUpdate`，显示 `[plan]` 标签、explanation 和 `☐/→/✓` 清单。
  - GUI (`agent-diva-gui`)：监听 `agent-turn-plan-updated` 事件，在 `ChatView.vue` 中复用 `TodoCard.vue` 渲染只读 plan 卡片，并新增 `planUpdateCard`  locale 文案。

- **测试**
  - `agent-diva-core` / `agent-diva-tools` / `agent-diva-agent` 单元测试覆盖序列化、校验、注册、handler。
  - `agent-diva-manager` 单元测试覆盖 SSE 转发。
  - `agent-diva-cli/tests/update_plan_e2e.rs` 覆盖 CLI `ApiClient` → Manager → SSE → Client 完整路径。
  - `agent-diva-cli` 与 `agent-diva-gui` 组件回归测试覆盖 TUI 格式化与 GUI 渲染分支。

## 影响范围

- 仅影响普通聊天消息流；Plan 模式、Execution 模式、现有 TODO 持久化流程不变。
- 新增 `BuiltInToolsConfig.update_plan` 配置字段，默认由 CLI 读取 `config.tools.builtin.update_plan`。

## 已排除范围

- 不持久化到 `JsonlTodoStore` / `EphemeralPlanRegistry`。
- 不支持用户手动勾选、删除、新增条目。
- 不支持嵌套子任务。
- 不支持跨 turn / 跨 session 保留。

## 已知限制（已清理）

- ~~`cargo test --workspace update_plan` 因其它 crate 的脏改动无法通过~~ — 已在本轮清理中修复。
- ~~`cargo clippy --workspace -D warnings` 在 `agent-diva-core/src/planning/report.rs` 存在预存 `if_same_then_else` 错误~~ — 已修复。
- 唯一剩余非阻塞项是 `imap-proto v0.10.2` 的 future-incompat 警告，与本项目代码无关。
- 真实 CLI 二进制触发 `update_plan` 仍需要配置 mock provider 或真实 API key；本次验证以集成测试 + CLI `--help` 作为 smoke test 替代。
