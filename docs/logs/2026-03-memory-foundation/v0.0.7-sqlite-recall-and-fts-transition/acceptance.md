# v0.0.7 Acceptance

## Acceptance Steps

1. 在一个包含 `memory/diary/rational/*.md` 与 `memory/MEMORY.md` 的 workspace 中初始化 `WorkspaceMemoryService`。
2. 确认 `workspace/memory/brain.db` 已生成，并且内部已有 backfill 后的 diary/compat 记录。
3. 触发 `memory_recall`，确认结果来自 SQLite recall，同时不会因为 file recall 再返回一份重复记录。
4. 追加一条新的 rational diary，确认落文件后可在 SQLite 中找到对应 `diary:` 前缀记录。
5. 继续询问“之前做过什么 / 最近结论”类问题，确认自动 recall 行为不回归。

## Expected Outcomes

- SQLite recall 可命中结构化记录
- diary 与 `MEMORY.md` chunk 会被稳定导入 SQLite
- 混合 recall 不会出现明显的 file/sqlite 双份重复
- tool contract 不变
- 自动 recall policy 不变

## User-Facing Notes

- 本轮用户可见提升主要是 recall 命中质量和稳定性更好。
- 没有新增命令，没有新增配置项，也没有要求手工迁移旧记忆文件。
