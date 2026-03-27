# v0.0.9 Acceptance

## Acceptance Steps

1. 确认 [derived.rs](/Users/mastwet/agent-diva/agent-diva-memory/src/derived.rs) 已提供 diary -> `MemoryRecord` 提炼逻辑。
2. 确认 `sync_diary_entry_to_sqlite()` 与 `backfill_workspace_sources()` 都会写入提炼后的新 domain record。
3. 确认 `relationship` / `self_model` / `soul_signal` 继续走统一 `MemoryRecord`，没有引入新的特殊存储结构。
4. 确认 `memory_recall` / `memory_search` 可不改 schema 直接召回这些记录。
5. 确认删除或重建 `brain.db` 后，重新 backfill 仍能从 diary 恢复这些记录。

## Expected Outcome

- 新 domain 已经形成最小可用闭环。
- 下一步可以继续做 governance，而不需要回头重做写入来源或恢复路径。
