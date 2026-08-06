# Acceptance — GA-MEM-PARITY Wave 0（诚实与契约）

## 验收步骤

1. **sync_turn 诚实**：跑 `cargo test -p agent-diva-laputa`——测试
   `turn_sync_creates_pending_proposals_without_mutating_authority` 断言
   `ProposalCreated`（而非 `Persisted`）；consolidation 回归测试通过。
2. **配置默认**：`cargo test -p agent-diva-core` 中 3 个 authority_mode 用例
   （缺失段 → Typed；段存在字段缺失 → Typed；显式 legacy 生效）通过。
3. **prompt 诚实**：`cargo test -p agent-diva-agent --lib context` 中负面测试
   `prompt_does_not_promise_unavailable_memory_tools` 通过；人工抽查
   `build_system_prompt` 输出不再含 "available memory tools"。
4. **分工文档**：`docs/architecture/memory-write-paths-contract.md` 存在且与
   inventory §10.1–§10.3 一致。

## 验收结果

- [ ] 用户确认 4 项验收通过
- [ ] 用户确认可启动 Wave 1 排期

（由用户填写）
