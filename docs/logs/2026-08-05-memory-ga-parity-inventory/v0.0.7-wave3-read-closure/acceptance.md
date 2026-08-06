# Acceptance — GA-MEM-PARITY Wave 3（读侧闭环）

## 验收步骤

1. **D4 生产注入**：`cargo test -p agent-diva-laputa --lib wave3_tests` 中
   `recall_returns_prompt_block_with_recalled_content` 通过——prefetch 返回
   的 prompt_block 含召回关键字（证明 FTS → prompt_block 通路）。
2. **D2/D3 Legacy 降级**：`cargo test -p agent-diva-agent --lib wave3_tests` 中
   `legacy_prefetch_failure_leaves_messages_untouched` 通过——PrefetchStatus
   ::Failed 时 turn messages 保持 [system, user]，Legacy 文案可读。
3. **F5/H3 apply→typed 一致性**：
   - `memory_add_visible_in_next_startup_rendering` 通过——新 provider 实例
     startup markdown 含记录 id 与预览（证"下次会话可见"）。
   - `apply_then_search_remains_consistent_across_reopens` 通过——新 provider
     实例 search 命中（证 FTS 持久化）。
4. **G3/U3 读侧投影过滤**：
   - `supersedes_tombstone_filtered_from_next_startup_rendering` 通过——
     supersedes 目标不出现在启动 markdown。
   - `supersedes_tombstone_filtered_from_search` 通过——supersedes 目标不出
     现在 search 结果中（即使 FTS 命中）。
5. **F5/H3 工作记忆隔离**：`checkpoint_invisible_to_search_and_subsequent_
   startup` 通过——session end 后新 provider 实例 search 空 + startup 不含
   checkpoint 内容（证易失语义落地）。
6. **B2 L1 预算**：`startup_respects_l1_budget_ceiling` 通过——预算 2 行只
   渲染 2 条。
7. **注入顺序（D4 端到端）**：`typed_prefetch_inserts_after_working_memory_
   block` 通过——顺序 [system, working_memory, prefetch, user]。
8. **延期项条目化**：TODOLIST 中新增 `GA-MEM-PARITY Wave 3 延期项` 分组，
   F3/F4/F6/F7 各自有独立 TODO 条目，措辞与归属明确。

## 验收结果

- [ ] 用户确认 8 项验收通过
- [ ] 用户确认可启动 Wave 4（AutoDream 去重，inventory §10.3 G4）排期

（由用户填写）

## 真机联通（G2D+ 桌面验收，本 Wave 不强制）

U1「记住→下次会话还在」与 U2「你还记得吗」的真机 smoke 归 G2D+ 桌面验收
一并执行（规则 `smoke-test-required-for-user-visible-change` 由 G2D+ 阶段
覆盖）。自动化证据已具备（apply→FTS→startup 一致性测试 5 项通过）。
