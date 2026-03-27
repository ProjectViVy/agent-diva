# v0.0.9 Structured Memory Domains Minimum Loop

## Summary

本轮完成了 `relationship` / `self_model` / `soul_signal` 的最小写入与召回闭环，但没有新增 tool，也没有把这些 domain 做成特殊 core 结构。

## Changes

- 新增 [derived.rs](/Users/mastwet/agent-diva/agent-diva-memory/src/derived.rs)，提供 `derive_structured_memory_records()`：
  - 从 rational diary 中提炼稳定关系信号
  - 从 rational diary 中提炼 agent 自我工作方式信号
  - 从 rational diary 中提炼高优先级 soul signal
- 新 domain 全部继续使用统一 `MemoryRecord`：
  - `relationship`
  - `self_model`
  - `soul_signal`
- `sync_diary_entry_to_sqlite()` 现在在写入 diary 对应记录后，会继续写入提炼出的结构化 domain record。
- `backfill_workspace_sources()` 现在会在回填 diary 时同步重建这些结构化 record，保证：
  - 冷启动可恢复
  - `brain.db` 丢失后可重建
  - diary 仍是主来源
- `WorkspaceMemoryService` 现有 `memory_recall` / `memory_search` 无需改 schema，就能召回这些新 domain。
- 新增/更新单测，覆盖：
  - diary -> structured memory 提炼
  - backfill 幂等
  - service 层按 domain 召回 `relationship` / `soul_signal`
  - agent diary persist 后结构化 record 真实落库

## Behavior Impact

- `relationship` / `self_model` / `soul_signal` 不再只是枚举类型，而是有真实写入与检索价值。
- 新 domain 不绕开 diary：
  - 先写 diary
  - 再从 diary 提炼
  - backfill 时仍从 diary 重建
- 现有 tool contract 不变：
  - `memory_recall`
  - `memory_search`
  - `memory_get`
  - `diary_read`
  - `diary_list`

## Non-Goals

- 未引入专门的新 tool。
- 未引入复杂 ontology 或 relationship graph。
- 未引入治理权重系统。
- 未接入外部向量后端。
