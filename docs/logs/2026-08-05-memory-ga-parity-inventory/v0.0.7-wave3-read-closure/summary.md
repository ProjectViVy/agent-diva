# Summary — GA-MEM-PARITY Wave 3（读侧闭环）

- 版本：`v0.0.7-wave3-read-closure`
- 日期：2026-08-06
- 类型：读侧闭环（prefetch 生产注入 + apply→typed 一致性 + 延期条目化）
- 前置：`v0.0.6-wave2-layers-working-memory`

## 做了什么

Wave 3 聚焦**读侧闭环**：证明 Typed 权威写入后，下次启动/回合召回真正可见；
Legacy 降级可解释；U1/U2 跨会话记忆联通具备自动化证据。契约基线：inventory
§6 Wave 3 + §10.3 G2（读侧闭环缺失修补）+ TODOLIST WAVE3-MEMORY-READ-CLOSURE。

范围决策（已与用户确认）：**最小闭环**（D2/D3/D4 + F5/H3 + U1/U2 自动化，
真机延后到 G2D+）；F3/F4/F6/F7 条目化延后到 Wave 5 / GMH-52。

### S1 laputa：读侧集成测试 + supersedes-target 修复 — `3ae7975d`

- 新增 `mod wave3_tests` 到 `typed_provider.rs`（8 个测试）：
  - `recall_returns_prompt_block_with_recalled_content`（D4）：memory_add →
    prefetch → prompt_block 含关键字。
  - `memory_add_visible_in_next_startup_rendering`（D4+H3）：memory_add →
    drop provider → 新 provider 实例 startup markdown 含记录 id 与预览。
  - `supersedes_tombstone_filtered_from_next_startup_rendering`（G3/U3）：
    直接对 store 写入规范 supersedes tombstone（独立记录 + tombstone.target），
    新 provider 实例 startup markdown 不含目标 id 与内容。
  - `supersedes_tombstone_filtered_from_search`（G3/U3）：supersedes 目标
    不出现在 memory_search 结果中（即使 FTS 命中）。
  - `memory_remove_creates_governed_proposal`（G3 proposal-first）：typed
    memory_remove 返回 ProposalCreated 而非 Applied。
  - `startup_respects_l1_budget_ceiling`（B2）：L1 预算 2 行时只渲染 2 条。
  - `apply_then_search_remains_consistent_across_reopens`（F5/H3）：
    memory_add → drop provider → 新 provider 实例 memory_search 命中。
  - `checkpoint_invisible_to_search_and_subsequent_startup`（F5/H3）：
    checkpoint → on_session_end → 新 provider 实例 search 空 + startup 不
    含 checkpoint 内容。

- **生产侧 bug 修复（测试暴露）**：supersedes tombstone 此前只过滤「自身是
  tombstone 的记录」，不处理「被 tombstone 指向的目标记录」。新增
  `TypedMemoryStore::superseded_target_ids()` 方法（JOIN memory_supersedes
  与 memory_records WHERE tombstone=1，返回 HashSet），并在以下两处接线：
  - `open_with_l1_budget`（启动渲染 L1 index 过滤）；
  - `memory_search`（search_visible 后过滤）。

### S2 agent：prefetch 降级 + typed 注入顺序测试 — `e66cfa9b`

- 新增 `mod wave3_tests` 到 `agent_loop::turn::context`（3 个测试）：
  - `legacy_prefetch_failure_leaves_messages_untouched`（D2）：PrefetchStatus
    ::Failed 时 messages 保持 [system, user]。
  - `typed_prefetch_inserts_after_working_memory_block`（D4 端到端）：
    工作记忆 + prefetch 都注入时顺序为 [system, working_memory, prefetch,
    user]。
  - `no_injection_when_both_prefetch_and_working_memory_are_absent`
    （D2+D4）：无工作记忆 + Skipped prefetch 时不产生空 system 注入。

### S3 docs：TODOLIST + v0.0.7 四件套 — 本次提交

- TODOLIST WAVE3 条目所有子项勾选；F4 同会话热注入改为「延期（Wave 5 或
  独立 slice）」。
- 新增延期条目组 `GA-MEM-PARITY Wave 3 延期项`：F3（GUI/CLI 审批端到端，
  归 GMH-52/G2D+）、F4（同会话热注入）、F6（Rollback 端到端验收）、
  F7（tombstone U3 完整路径）。

## 验证

- S1：laputa 35 全绿（lib 35 + 集成 9）；clippy `-D warnings` 干净。
- S2：agent 386 全绿（lib 386 + 集成 15）；clippy 干净。
- 全 workspace（除 CLI/GUI）：core 692 / laputa 35 / agent 386 / tools 109 /
  manager 全绿。CLI 6 个既有 wiremock 502 失败
  （`CLI-WIREMOCK-502-PREEXISTING`）与本迭代无关。

## 影响范围

- laputa：TypedMemoryStore 新增 `superseded_target_ids()` 公开方法；
  TypedLaputaMemoryProvider 启动渲染与 memory_search 接线；测试 +8。
- agent：turn/context.rs 测试 +3；无生产侧代码变更。
- docs：TODOLIST 延期条目组；v0.0.7 四件套。
