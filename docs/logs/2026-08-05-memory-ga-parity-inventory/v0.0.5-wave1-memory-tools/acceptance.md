# Acceptance — GA-MEM-PARITY Wave 1（Agent 记忆工具 CRUD）

## 验收步骤

1. **六工具注册**：`cargo test -p agent-diva-agent --lib tool_assembly` 中
   注册测试（六工具全部装配，mask deny 生效）通过；`cargo test -p
   agent-diva-tools` 六工具单测（applied / proposal_created / failed /
   invalid args）通过。
2. **add 即时权威**：`cargo test -p agent-diva-laputa` 中
   `memory_add` 测试断言记录以 `AppliedAuthority` 落库且 `memory_list`
   可见（启动渲染一致）。
3. **update/remove 走审批**：同 crate 测试断言 `MemoryPatch` /
   `Deprecation` proposal 创建并经 `coordinator.submit`（返回
   `ProposalCreated` 含 id）；审批在 GUI/CLI 经 GMH-31 可见。
4. **G3 tombstone 验收**：删除（deprecation apply）后 `search_visible` /
   `list` 不再出现该记录——S2 测试
   「search 排除 tombstone」通过（工具路径删除后不再出现）。
5. **G1 distill 最小版**：新建 skill 即时写 `<ws>/skills/<name>/SKILL.md`
   并返回 `applied`；覆盖已有 skill 返回 `proposal_created`（SopCreate）。
6. **Legacy proposal-first**：`cargo test -p agent-diva-agent` 中 legacy
   add 测试断言 proposal 落在 `.laputa` 且 MEMORY.md 未被直写。
7. **prompt 指引**：`cargo test -p agent-diva-agent --lib context` 中
   `prompt_guides_memory_tool_usage` 通过；人工抽查 build_system_prompt
   输出含 memory_add 指引与三态语义。
8. **全量验证**：`just fmt-check && just check && just test`——仅 CLI
   6 个既有 wiremock 502 失败（`CLI-WIREMOCK-502-PREEXISTING`）可接受。

## 验收结果

- [ ] 用户确认 8 项验收通过
- [ ] 用户确认可启动 Wave 2（工作记忆/分层）与 Wave 3（读侧闭环）排期

（由用户填写）
