# 性能与成本

## 1. 预算

- 每次反思限定 evidence 条数、字符数和 token 估算。
- 同一 session/digest 只进入一个未完成 batch。
- 设置 per-run、per-day token/cost 上限和 provider 并发上限。
- 默认批量处理，不为每轮会话单独调用 LLM。

## 2. 热点

- journal append 必须轻量，不能阻塞 AgentLoop。
- typed duplicate 检查优先使用 digest/index，不把全量 Memory 送给模型。
- Recall feedback 异步批写，但 authority apply 保持同步确认。
- GUI 使用 SSE 增量事件，不轮询全量 proposals/runs。

## 3. 基线

- 10k typed records 下 Recall p95、FTS 候选数和 prompt token 成本。
- 1k session evidence 批处理的内存峰值。
- 100 并发 run trigger 的去重与队列延迟。
- 断网、限流下不会产生无界 retry。

性能优化不得删除 provenance、审计、幂等或完整性校验。
