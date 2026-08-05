# Release — ask_user 表面闭环（Phase 2）

- 版本：`v0.2.0-ask-user-surface`
- 日期：2026-08-05

## 发布方式

随 `agent-diva-pro` 分支 4 个聚焦 commit 提交（manager/cli/gui 桥/gui 前端）。
无配置迁移；`tools.builtin.ask_user` 默认开启，旧配置向后兼容。

## 行为影响（用户可感知）

- **CLI chat/TUI**：Agent 调用 `ask_user` 时终端出现结构化问题与选项；
  非 TTY 管道自动取消挂起问题，不阻塞。
- **GUI**：聊天内出现问题卡（选项/Other/取消），回答回传后 Agent 继续；
  2s 轮询仅在存在挂起问题时渲染卡片。
- **Manager**：`/api/ask-user/questions` 等 3 端点供 Tauri 桥与外部客户端使用。

## 回滚

- 关闭配置 `tools.builtin.ask_user = false` 即移除工具注册（回到 Phase 1 前行为）。
- 移除对应 commit 即可整体回退；coordinator 为进程内内存，无持久化残留。
