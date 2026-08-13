# 普通聊天 update_plan TODO 清单 — 发布记录

## 发布状态

**未发布** — 本次变更仍位于工作树中，未生成独立 release 或 tag。

## 部署方式

- 合并到 `agent-diva-pro` 主线后，用户通过 `cargo build --all` / `just build` 重新编译即可使用。
- GUI 用户需额外在 `agent-diva-gui` 目录执行 `pnpm install && pnpm build`（或 `pnpm tauri dev`）以获取前端变更。

## 无独立发布的原因

- 本特性属于功能增量，不涉及版本号升级或发布产物。
- 合并后由主仓库版本统一发布（当前主版本 `0.5.0`）。
- 合并前已确认：
  1. `cargo test --workspace update_plan` 通过。
  2. `cargo clippy --workspace -D warnings` 通过。
  3. `cargo fmt --check` 通过。
- 合并后建议：在真实或 mock provider 环境下完成一次端到端 CLI/GUI smoke test。

## 回滚策略

- 本次改动均为新增代码，未修改现有 Plan 模式 / TODO 持久化逻辑。
- 如需回滚，可移除 `agent-diva-core/src/planning/update_plan.rs`、`agent-diva-tools/src/update_plan.rs` 中的新增类型与工具，并恢复 `tool_assembly.rs`、`context.rs`、`handlers.rs`、`main.rs`、`App.vue`、`ChatView.vue`、`TodoCard.vue` 中的相关片段。
- 建议以独立 commit 保存本次变更，便于 revert。
