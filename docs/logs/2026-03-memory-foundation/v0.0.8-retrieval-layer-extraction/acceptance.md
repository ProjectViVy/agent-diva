# v0.0.8 Acceptance

## Acceptance Steps

1. 确认 `agent-diva-memory/src/service.rs` 不再承载 merge、embedding cache、hybrid rerank 细节。
2. 确认 `agent-diva-memory/src/retrieval.rs` 已存在 `RetrievalEngine`、`SemanticRetriever`、`HybridReranker` 等核心抽象。
3. 确认 `memory_recall`、`memory_search`、`memory_get` 对外 contract 未变化。
4. 在 semantic 不可用或失败时，确认 recall 仍返回 keyword 结果而不是报错中断。
5. 运行验证命令后，确认 `agent-diva-memory` 单测和全仓测试均通过。

## Expected Outcome

- retrieval 逻辑已经从 facade 下沉成独立底座。
- 下一轮可以继续接 `relationship` / `self_model` / `soul_signal`，而不需要再次重拆 `service.rs`。
