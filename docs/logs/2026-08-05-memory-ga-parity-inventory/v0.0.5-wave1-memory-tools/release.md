# Release — GA-MEM-PARITY Wave 1（Agent 记忆工具 CRUD）

## 发布方式

代码变更随常规 crate 构建发布（core/laputa/agent/tools），无独立部署物。

## 行为变更提示

- **新增六个 memory 工具**（memory_add / memory_list / memory_search /
  memory_update / memory_remove / memory_distill）：ToolAssembly 默认注册
  （mask 默认启用；`builtin.memory = false` 或 minimal/none/for_subagent
  配置下关闭）。工具结果三态 JSON：`applied`（低风险即时权威）、
  `proposal_created`（含 proposal_id，高风险待审批）、`failed`（含 reason）。
- **写路径分级**：memory_add 即时写入权威（AppliedAuthority）；memory_update
  / memory_remove 一律创建 proposal（MemoryPatch / Deprecation）并提交
  治理审批，审批经 GMH-31 GUI/CLI 可见，**不自动生效**。
- **Legacy 模式**：写路径不再直写 MEMORY.md 冒充权威，全部走 Laputa
  proposal（proposal-first）；读路径仍由 MemoryManager 提供。
- **Cutover/Degraded 模式**：memory 工具返回 `failed{unsupported}`（文档化
  降级，不静默）。
- **prompt 恢复工具指引**：Wave 0 的诚实降级文案被工具指引替代（「用
  memory_add 记住…」）；未装配 provider 时工具仍返回 failed，不会假承诺。

## GUI/CLI 影响

- 无 wire/SSE/Tauri 协议变更（MemoryCrudOutcome 为进程内类型）。
- proposal 审批界面无需改动（复用 GMH-31 既有展示路径）。
