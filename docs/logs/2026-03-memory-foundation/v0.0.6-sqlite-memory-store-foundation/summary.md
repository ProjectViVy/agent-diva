# v0.0.6 SQLite Memory Store Foundation

## Summary

本轮在 `agent-diva-memory` 内为长期结构化记忆补上了 SQLite 主存储基础，但没有切换现有 recall 主链路。

## Changes

- 新增 `SqliteMemoryStore`，位置在 `agent-diva-memory/src/store/sqlite.rs`。
- 新增 `store` 模块导出，并在 `agent-diva-memory/src/lib.rs` 暴露 `SqliteMemoryStore`。
- 新增 `brain_db_path()` 路径 helper，固定数据文件为 `workspace/memory/brain.db`。
- `SqliteMemoryStore` 实现了 `MemoryStore` 的最小可用能力：
  - `store_record`
  - `get_record`
  - `list_records`
  - `forget_record`
  - `recall`
- SQLite schema 采用 `memory_records + schema_migrations` 的收敛版设计。
- `tags` 与 `source_refs` 先以 JSON 文本形式持久化。
- 开启 WAL，并保留 schema 初始化的幂等行为。
- `WorkspaceMemoryService` 新增对 SQLite store 的组合，但 `memory_recall` 仍继续走 `FileRecallEngine`。

## Behavior Impact

- `memory/brain.db` 现在会在初始化 `WorkspaceMemoryService` 或显式创建 `SqliteMemoryStore` 时生成。
- 现有 file diary、`MEMORY.md` chunk recall、自动 recall policy、memory tools 的对外行为保持不变。
- 这次新增的是结构化主存储基础，不是 recall 主路径切换。

## Non-Goals

- 未实现 FTS5 / BM25
- 未实现 embedding / hybrid recall
- 未实现 Qdrant 或其他外部向量后端
- 未做 diary / `MEMORY.md` 自动回填 SQLite
- 未变更 `agent-diva-core::memory` 最小兼容层
