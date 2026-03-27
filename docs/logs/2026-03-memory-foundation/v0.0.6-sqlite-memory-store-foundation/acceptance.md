# v0.0.6 Acceptance

## Acceptance Steps

1. 在一个空 workspace 下初始化 `WorkspaceMemoryService`。
2. 确认 `workspace/memory/brain.db` 被创建。
3. 使用 `SqliteMemoryStore` 写入一条 `MemoryRecord`，再读取、列举并删除它。
4. 确认现有 `memory_recall`、`diary_read`、`diary_list` 仍然工作。
5. 确认 agent 对“之前做过什么 / 最近结论”类问题的自动 recall 行为未变化。

## Expected Outcomes

- `brain.db` 创建成功
- SQLite schema 初始化成功且可重复执行
- `MemoryStore` 基础 CRUD 正常
- 现有 recall policy 不回归
- 现有工具契约不回归
- `MEMORY.md` / diary 文件布局保持不变

## User-Facing Notes

- 本轮新增的是底层结构化主存储能力，用户可见行为基本不变。
- 没有新增配置项，没有新增工具命令，也没有要求迁移旧记忆文件。
