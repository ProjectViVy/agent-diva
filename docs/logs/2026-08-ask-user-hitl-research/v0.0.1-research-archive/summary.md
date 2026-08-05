# Summary — ask-user / Clarify HITL 研究归档

- 版本：`v0.0.1-research-archive`
- 日期：2026-08-05
- 类型：文档-only（无产品代码）

## 做了什么

1. 只读勘察确认：Agent 主动「询问工具」在当前主干**不存在**；`MessageTool` 未装配；system prompt 压制工具式沟通。
2. 与 M3 **审批** HITL（GMH-30..33）划界：本缺口为 **Conversational Clarify HITL**。
3. 对照 Hermes `clarify`、Claude Code `AskUserQuestion`、OpenHarness `ask_user_question`。
4. 写出分阶段提案（运行时 → GUI/CLI → 策略硬化），推荐工具名 `ask_user`。
5. 归档到 `docs/research/ask-user-clarify-hitl-proposal.md`，索引 `docs/research/README.md`，`TODOLIST.md` 新增 `CLARIFY-HITL` 并标注 M3 不含本能力。

## 影响范围

- 文档：`docs/research/*`、`TODOLIST.md`、本日志目录
- 运行时 / GUI / 工具：**无代码变更**

## 未做

- 任何 `ask_user` / Coordinator 实现
- GUI QuestionCard
- 历史 archive gap 文档正文勘误（仅在提案中标注误判）
