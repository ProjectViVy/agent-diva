# v0.0.5 Memory Recall Engine Foundation

## Summary

本轮把 `agent-diva-memory` 从“service 内部私有 recall”提升为可演进的 recall 子系统骨架，但没有进入 SQLite/FTS。

## Changes

- 新增 `FileRecallEngine`，作为 `RecallEngine` 的文件型 concrete implementation，位置在 `agent-diva-memory/src/recall.rs`。
- 新增 `MemoryMdChunkSource`，把 `memory/MEMORY.md` 从整文件 recall 升级为 chunk recall，位置在 `agent-diva-memory/src/compat_source.rs`。
- 新增 `MemoryBackendKind` 作为后续 backend 演进占位，位置在 `agent-diva-memory/src/backend.rs`。
- `WorkspaceMemoryService` 收缩为 facade / adapter，`memory_recall()` 与 `recall_records_for_context()` 改为委托 `FileRecallEngine`。
- 保持 `MemoryToolContract`、`DiaryToolContract`、`MemoryQuery`、`MemoryRecord` 的公开接口不变。
- 保持 `agent-diva-core::memory` 不变，未把增强 memory 类型回流到 `core`。

## Behavior Impact

- diary 记录与 `MEMORY.md` chunk 现在会统一进入 recall 候选集。
- `MEMORY.md` 命中不再只有一条“大块整文件记录”，而是可返回多个标题块/段落块。
- recall 排序改为先按 query 匹配度，再按来源优先级与时间排序；近期 diary 在同分时优先于 compatibility chunk。
- `agent-diva-agent` 的自动 recall policy 行为保持不变，但底层 recall 质量更稳定。

## Zeroclaw Alignment

本轮主要吸收了 `.workspace/zeroclaw` 的三点：

- `Memory` 抽象与 backend 解耦。
- retrieval pipeline 分层，而不是让 service 同时承担全部职责。
- `MEMORY.md` 只是 compatibility source，不是最终主存储。

## Non-Goals

- 未实现 SQLite / `brain.db`
- 未实现 FTS / BM25 / embedding
- 未新增 `memory_search` / `memory_get`
- 未调整自动 recall 的触发策略与用户可见性
