# v0.0.7 SQLite Recall And FTS Transition

## Summary

本轮把 `brain.db` 从“仅可存储”推进到了“可检索、可回填、可混合召回”的状态，但没有切成 SQLite-only recall。

## Changes

- 新增 `SqliteRecallEngine`，位置在 `agent-diva-memory/src/sqlite_recall.rs`。
- `SqliteMemoryStore` 升级到 schema `v2`，新增：
  - `tags_text`
  - `source_paths_text`
  - `memory_records_fts`
  - 对应 FTS triggers
- `SqliteMemoryStore::recall()` 现在优先使用 FTS5 做候选检索，再结合 query/filter/source rank 做排序。
- 新增 backfill/sync helper，位置在 `agent-diva-memory/src/sync.rs`：
  - `backfill_workspace_sources`
  - `stored_diary_record`
  - `stored_compat_record`
  - `sync_diary_entry_to_sqlite`
- `WorkspaceMemoryService` 改成混合 recall 编排器：
  - 继续持有 `FileRecallEngine`
  - 新增 `SqliteRecallEngine`
  - `memory_recall()` 与 `recall_records_for_context()` 改为合并 sqlite/file recall 结果
- diary 新写入现在会在落文件后同步 upsert 到 SQLite。

## Behavior Impact

- `brain.db` 中的 `memory_records` 现在可以承接 diary 与 `MEMORY.md` chunk 的结构化副本。
- `WorkspaceMemoryService::new()` 会执行幂等 backfill，把现有 diary 与 `MEMORY.md` 导入 SQLite。
- recall 现在默认走“SQLite 为主 + file recall 补漏”的混合过渡模式。
- 去重优先保留 SQLite 记录，避免 diary/file 与 sqlite 双份重复返回。

## Non-Goals

- 未引入 embedding
- 未引入 hybrid semantic merge
- 未引入 Qdrant 或其他外部向量后端
- 未修改 memory tool 的公开参数 schema
- 未改变自动 recall policy 的触发规则
