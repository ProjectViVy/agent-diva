# Acceptance

## Acceptance Steps

1. 在包含 `memory/MEMORY.md` 与 `memory/diary/rational/*.md` 的 workspace 中启动 agent。
2. 询问历史/进度类问题，例如“之前我们对 memory 拆分做了什么结论”。
3. 确认 agent 首轮回答前会自动拿到 recall 上下文，而不是只依赖 `MEMORY.md` 全量注入。
4. 确认命中结果可同时来自 diary 和 `MEMORY.md` chunk，而不是只返回一条整文件 compatibility record。
5. 确认 `memory_recall`、`diary_read`、`diary_list` 仍可正常使用。

## Expected Result

- 自动 recall 行为仍是保守触发、默认静默。
- `MEMORY.md` recall 质量优于整文件单条记录。
- `WorkspaceMemoryService` 已退回 facade 角色，engine/store 分层更清晰。
