# Release

## 发布方式

- 本轮为工作区内开发迭代，不涉及单独部署步骤。
- 若后续需要正式交付，可直接基于当前 worktree 继续做提交/PR；本轮变更不要求数据迁移，也不改变已有 `MEMORY.md` / `HISTORY.md` / `memory/diary/rational/YYYY-MM-DD.md` 文件布局。

## 发布前检查

- 确认 agent runtime 已使用同一份 `WorkspaceMemoryService` 同时服务工具注册和自动 recall policy。
- 确认历史型问题的首轮 system prompt 中能看到 auto-recalled memory 摘要。
- 确认自动 recall 结果不写入 session history，只在当前 turn 生效。
