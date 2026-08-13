# Harness Engineering 下一阶段方向

- 原始记录：`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/root-history/DECISION-v2-next-phase.md`
- 日期：2026-06-18–19
- 状态：`Approved Direction / Historical Runtime Strategy`

## 核心判断

Agent Harness 是包裹模型的确定性运行时：模型提出，Harness 负责验证、授权、执行、记录和
恢复。模型可见提示词不能替代状态机、schema、权限、预算和安全门禁。

## 方向性决策

- 标准化 `decide → act → observe`；
- 强化 Module 生命周期和显式装配；
- 拆分细粒度 Poke/运行时事件链；
- 将工具权限、Provider/Channel schema、安全预算和 Prompt Injection 防御作为运行时合同；
- 行为审计使用结构化日志基础设施；治理审计仍保留在治理领域；
- Context compaction 不重复建设，转由 C1–C5 当前上下文合同承接；
- 插件热重载不提前实现，必须先完成能力和安全研究。

## 当前继承关系

本决策的 Harness 原则仍可作为工程指导；具体 Context、Memory、Persona、STM、Evolution
边界不得从 2026-06 旧文档推断，必须读取当前 `docs/architecture/` 与 `docs/research/`。
