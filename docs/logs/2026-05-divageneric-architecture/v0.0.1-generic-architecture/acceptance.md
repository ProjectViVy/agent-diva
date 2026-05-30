# DivaGeneric 架构文档验收步骤

## 用户/产品视角验收

1. 打开 `docs/dev/genericagent/newedge/architecture.md`。
2. 确认文档明确当前方向：DivaGeneric 以 GenericAgent 化为主目标，Laputa 只作为人格连续性层，Mentle 只作为可选后端。
3. 确认文档逐项绑定当前代码接缝，而不是只写抽象设计。
4. 确认文档包含 L0-L4 到 agent-diva 的映射。
5. 确认 L0-L4 章节沿用此前对话中确认过的 Diva/Laputa 文件名，而不是照搬 GenericAgent 原始文件名。
6. 确认文档敲定 `.laputa/index.md`、`.laputa/MEMORY.md`、`.laputa/relationships.md`、`.laputa/sop/*.md`、`.laputa/rhythm/*`、`.laputa/inbox/*.jsonl` 等具体落点。
7. 确认文档明确在线路径和离线路径的职责差异。
8. 确认文档包含 Plan Mode 架构，且沿用 `plan.md`、`exploration_findings.md`、`verification.md` 等既有概念。
9. 确认 Plan Mode 是 Diva 原生能力，不是 Laputa 能力。
10. 确认 Plan Mode 文件落点为 `.diva/plans/<plan-id>/`，计划状态与 session message history 分离，且不混入 `.laputa/`。
11. 确认 Plan Mode 包含 Explore -> Plan -> Execute -> Verify 四阶段、approval gate 和 `PASS/FAIL/PARTIAL` verdict。
12. 确认文档要求所有 provider failure degrade，不阻断主对话。
13. 确认文档没有要求绕过 `MemoryProvider` 新建并行记忆管线。
14. 确认文档没有让 Mentle full tool set 进入日常聊天上下文。

## 工程视角验收

- 后续实现者可以直接根据 P0-P7 路线拆分任务。
- 后续接口设计有推荐类型和建议 crate 位置。
- 后续测试要求已覆盖 ContextBuilder、AgentLoop、consolidation、subagent、Mentle、Laputa、daily rhythm、Plan Mode 等关键路径。
