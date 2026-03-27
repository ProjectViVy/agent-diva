# v0.0.8 Retrieval Layer Extraction

## Summary

本轮把 `WorkspaceMemoryService` 内部的 recall/hybrid/embedding 编排拆成独立 retrieval 层，先完成“可扩展底座”整理，不改变现有 tool contract。

## Changes

- 新增 `agent-diva-memory/src/retrieval.rs`，明确三类职责边界：
  - `KeywordRetriever`
  - `SemanticRetriever`
  - `HybridReranker`
- 新增 `RetrievalEngine` 作为 retrieval facade，统一负责：
  - keyword recall 扩限
  - sqlite/file merge 后的 hybrid rerank
  - semantic 失败时自动降级
- 新增 `MergedKeywordRetriever`，把 `SqliteRecallEngine` 与 `FileRecallEngine` 的合并与去重逻辑从 `service.rs` 下沉。
- 新增 `CachedSemanticRetriever`，把 query/document embedding cache 与 embedding provider 调用边界收敛到 retrieval 层。
- 新增 `EmbeddingCacheStore` 抽象，当前由 `SqliteMemoryStore` 提供实现，为后续向量后端继续抽象预留接口。
- `WorkspaceMemoryService` 现在只保留：
  - 初始化 provider/store/retrieval engine
  - snapshot/hydrate/backfill 编排
  - `memory_recall` / `memory_search` / `memory_get` facade
- `agent-diva-memory/src/lib.rs` 导出 retrieval 相关类型，便于后续 domain/governance/vector backend 继续接入。

## Behavior Impact

- `memory_recall`、`memory_search`、`recall_records_for_context` 的外部行为保持不变。
- `RecallMode::KeywordOnly` / `SemanticDisabled` 仍会直接走 keyword 结果。
- semantic 路径失败时仍会自动降级，不影响现有调用方。
- `service.rs` 明显变薄，后续 governance 和 vector backend 可以挂在 retrieval 层，而不是继续堆进 facade。

## Non-Goals

- 未新增 `relationship` / `self_model` / `soul_signal` 的写入闭环。
- 未修改 diary 治理策略。
- 未接入 LanceDB、Qdrant 或其他外部向量后端。
- 未修改现有 memory/diary tool schema。
