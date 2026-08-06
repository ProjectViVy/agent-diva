# Summary — GA-MEM-PARITY Wave 5（巩固与清理）

- 版本：`v0.0.9-wave5-consolidation-gc`
- 日期：2026-08-07
- 类型：巩固与清理（4 切片，每切片独立提交）
- 前置：`v0.0.8-wave4-autodream-dedup`

## 做了什么

Wave 5 按 inventory §6 + §10.1 #4 收口三项：consolidation 条目化兜底（E4/E5）、
working memory GC（B7 子集）、F6/F7 端到端验收。加上 Wave 2 延期到 Wave 5 的
会话异常退出残留 GC。Superseded gate 变体（B）从 S2 独立切片承接 F7 部分路径。

**不吞下**：B9 tool-result 强制校验、F4 同会话热注入、G1–G12 真机端到端。

### S1 laputa：Working memory GC — `865f527d`

- `TypedMemoryStore::gc_session_scoped(ending_session_id)` — 物理 DELETE
  `scope.session_id == ending_session_id` 的记录（非权威，不走 proposal）
- `TypedMemoryStore::gc_stale_session_scoped(active_session_ids)` — 启动
  清理孤儿 session-scoped 记录
- `TypedLaputaMemoryProvider::on_session_end` 调用 `gc_session_scoped`
- `open()` 末尾调用 `gc_stale_session_scoped`；失败仅 `warn!` 不阻断启动
- 3 个 wave5_tests：目标 session 隔离 / 孤儿清理 / 空库 graceful

### S2 autodream：Superseded gate 变体 — `16aa46ed`

- `CandidateRejectionCode::Superseded`（优先级 > Duplicate > Suppressed）
- `BoundedReflectionInput.superseded_memory_digests`（`#[serde(default)]`）
- `LaputaService::superseded_authority_digests()` — 缺库优雅返回空
- worker.rs 双路 digest 扩展（applied + superseded）
- 3 个 service-level + 3 个 gate-level 测试

### S3 laputa tests：F6/F7 端到端验收 — `043beb5b`

- `tests/wave5_acceptance.rs`（新集成测试文件）：
  - `f6_rollback_clears_fts_and_startup` — governed apply → rollback → FTS +
    startup 不含目标
  - `f6_rollback_is_idempotent` — rollback 两次第二次返回 false
  - `f7_tombstone_lifecycle_filters_startup_and_search` — tombstone → 下次
    startup + search 过滤（已知 memory_list 不过滤 superseded 记录）
  - `f7_tombstone_target_missing_stores_tombstone_record` — 不存在 id 的
    tombstone 不崩溃

### S4 agent：Consolidation 条目化兜底 — `3da2bb7b`

- `save_memory` prompt v3：要求 `[{action, id?, content}]` 结构化 JSON
- 逐条 dispatch 到 `memory_add/update/remove`（`MemoryCrudOutcome` 统计）
- 非数组降级到 `sync_turn` proposal + reason "consolidation fallback"
- `agent_diva_tools::distill_guard`：跨 crate thread-local flag
  （`memory_distill` 成功后置位 → `should_consolidate` 跳过）
- 3 个 wave5_tests + prompt 契约测试

## 关键决策

1. **Working memory GC 采用物理 DELETE**（不走 proposal）——工作记忆本就不是
   权威，无 tombstone 需求。
2. **Superseded gate 用 digest 集合**（不用 record_id）——worker 手里只有
   digest 源，与 G4 Duplicate 同模式。
3. **Consolidation flag 用 thread-local**——`agent-diva-tools::distill_guard`
   跨 crate 共享（agent 依赖 tools）；`tokio::test` 多线程场景下每个 test 开头
   `reset_distill_flag()` 隔离。
4. **不引入新 trait/新工具/新配置字段**——纯接线 + 测试 + 文档。
