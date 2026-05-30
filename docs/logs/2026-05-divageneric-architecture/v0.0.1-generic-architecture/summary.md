# DivaGeneric 架构文档固化总结

## 版本信息

- 版本：`v0.0.1-generic-architecture`
- 日期：2026-05-30
- 范围：`docs/dev/genericagent/newedge/architecture.md`

## 变更内容

本轮创建 DivaGeneric 单一详细架构文档，固化当前审批方向：

- 以 `divageneric` 分支和 `origin/vrm-memory-test` 为基线。
- 将 DivaGeneric 定义为 GenericAgent 记忆纪律、agent-diva 模块化运行时、Laputa 人格连续性、Mentle 可选后端的组合。
- 明确 GenericAgent 化是主目标，Laputa 不是主架构中心。
- 逐项绑定当前代码接缝：`MemoryProvider`、`ContextBuilder`、`AgentLoop`、`ToolAssembly`、`MentleRuntime`、`HybridMemoryProvider`、`consolidation`、SOUL 注入机制。
- 给出 `agent-diva-generic` 推荐 crate 边界、L0-L4 映射、在线/离线路径、接口草案、分阶段实施路线和验收标准。
- 根据后续确认，修订 L0-L4 章节：沿用此前对话中已经形成的 Diva/Laputa 文件名体系，并定稿 `.laputa/SOUL.md`、`.laputa/expectations.md`、`.laputa/index.md`、`.laputa/MEMORY.md`、`.laputa/relationships.md`、`.laputa/sop/`、`.laputa/rhythm/`、`.laputa/inbox/` 等文件/目录落点。
- 根据后续确认，新增 Plan Mode 架构章节：固化 `.agent-diva/plans/<plan-id>/`、`plan.md`、`exploration_findings.md`、`state.json`、`events.jsonl`、`verification.md`、`evidence/` 等文件名，并定义 Explore -> Plan -> Execute -> Verify 四阶段、approval gate、verification verdict、PlanOrchestrator 边界和实施路线。后续又明确 Plan Mode 是 Diva 原生运行时能力，不是 Laputa 能力；Laputa 只提供人格/期望输入和沉淀目标。

## 影响范围

本轮仅新增设计文档和迭代日志，不修改 Rust 代码、配置、README 或运行时行为。
