# Acceptance — GA-MEM-PARITY Wave 2（分层与工作记忆）

## 验收步骤

1. **L1 预算注入**：`cargo test -p agent-diva-laputa` 中
   `startup_injects_bounded_l1_index_not_full_content` 与
   `zero_l1_budget_renders_no_index` 通过——启动注入只含 ≤N 行索引与
   取详情指引，不含全文；`memory.l1_index_lines = 0` 时不注入。
2. **Legacy 同步**：`cargo test -p agent-diva-core` 中
   `l1_block_caps_lines_and_never_injects_full_content` 通过；MemoryManager
   启动块为 L1 索引格式。
3. **checkpoint 写读**：`cargo test -p agent-diva-laputa` 中
   `checkpoint_write_read_roundtrip_and_excluded_from_startup` 通过——
   `update_working_checkpoint` 后 working_memory_block 含 key_info/
   related_sops/content，且不出现在启动注入中。
4. **回合注入**：`cargo test -p agent-diva-agent` 中
   `working_memory_block_injects_after_system_prompt` 通过——working
   memory 块位于 system prompt 之后；空块不注入。
5. **会话清理（U5）**：`session_end_clears_checkpoint` 通过——session end
   后 checkpoint 不再可见；agent loop 退出按会话枚举清理。
6. **L0 policy**：`prompt_injects_l0_memory_management_policy` 通过——
   system prompt 含 `## Memory Management Policy` 与三条原则。
7. **工具接线**：`cargo test -p agent-diva-agent` 中
   `working_checkpoint_tool_registers_with_session_binding` 与
   `working_checkpoint_tool_gated_by_working_memory_flag` 通过；无
   session 时工具返回 failed。
8. **distill evidence（G1）**：`distill_fresh_writes_evidence_file` 通过——
   `memory_distill` 带 evidence 时新建 skill 目录含 `EVIDENCE.md`。

## 验收结果

- [ ] 用户确认 8 项验收通过
- [ ] 用户确认可启动 Wave 3（读侧闭环：prefetch 生产注入 + 启动一致）排期

（由用户填写）
