# Release — GA-MEM-PARITY Wave 2（分层与工作记忆）

## 发布方式

代码变更随常规 crate 构建发布（core/laputa/agent/tools/manager），无独立部署物。

## 行为变更提示

- **启动注入改为 L1 索引**（B2/B10）：system prompt 不再注入记忆全文，
  只注入有界索引（默认 ≤30 行、每条 ≤80 字符预览 + 取详情指引）。需要
  细节时模型应调用 `memory_search` / `memory_list` 按 id 取全文。可通过
  `memory.l1_index_lines` 配置（0 = 不注入索引）。
- **新增 `update_working_checkpoint` 工具**：写入当前会话的易失工作记忆
  （key_info / related_sops / content）；每轮对话自动注入 `## Working
  Memory` 块；会话结束（loop 退出）自动清理。工作记忆**不是长期权威**，
  需要固化时用 `memory_distill`（可带 `evidence` 参数，写入
  `skills/<name>/EVIDENCE.md`）。
- **system prompt 新增 `## Memory Management Policy` 段**（L0 policy 三条）。
- 配置面：`memory.l1_index_lines`（默认 30）；tools builtin
  `working_memory` gate（默认 true；minimal/none/for_subagent 关闭）。

## GUI/CLI 影响

- 无 wire/SSE/Tauri 协议变更（working 契约与 L1 渲染均为进程内）。
- prompt 变化会体现在所有 agent 对话的 system prompt 中（L1 索引 + policy
  段 + working memory 块），模型行为预期：按指针取详情而非依赖全文注入。
