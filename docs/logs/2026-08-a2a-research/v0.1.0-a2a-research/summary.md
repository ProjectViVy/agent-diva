# A2A 方案调研：交付摘要

## 状态

Research / Proposal；待正式立项，未授权生产实现。

## 本次交付

- 新增 A2A Agent-to-Agent 互操作研究包。
- 对比 OpenFang、ZeroClaw、OpenAkita、OpenHarness、Pi、Codex App Server 和 Claude ACP Link。
- 结合 agent-diva 当前 Manager、AgentLoop、MessageBus、Subagent、Sandbox、Approval、Laputa/BML 边界，给出五种适配方案。
- 在 `TODOLIST.md` 新增 `A2A-INTEROPERABILITY-EPIC`，明确“待正式立项”。
- 未修改生产代码、配置合同或运行时行为。

## 主要建议

采用“内部统一 AgentRun/Task/Policy 模型 + 原生 A2A adapter”；短期需要互操作验证时，先使用独立 Sidecar。
