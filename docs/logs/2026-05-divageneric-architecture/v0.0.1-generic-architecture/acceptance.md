# DivaGeneric 架构文档验收步骤

## 用户/产品视角验收

1. 打开 `docs/dev/genericagent/newedge/architecture.md`。
2. 确认文档明确当前方向：DivaGeneric 以 GenericAgent 化为主目标，Laputa 只作为人格连续性层，Mentle 只作为可选后端。
3. 确认文档逐项绑定当前代码接缝，而不是只写抽象设计。
4. 确认文档包含 L0-L4 到 agent-diva 的映射。
5. 确认文档明确在线路径和离线路径的职责差异。
6. 确认文档要求所有 provider failure degrade，不阻断主对话。
7. 确认文档没有要求绕过 `MemoryProvider` 新建并行记忆管线。
8. 确认文档没有让 Mentle full tool set 进入日常聊天上下文。

## 工程视角验收

- 后续实现者可以直接根据 P0-P7 路线拆分任务。
- 后续接口设计有推荐类型和建议 crate 位置。
- 后续测试要求已覆盖 ContextBuilder、AgentLoop、consolidation、subagent、Mentle、Laputa、daily rhythm 等关键路径。

