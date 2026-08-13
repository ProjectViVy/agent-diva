# AutoDream 与 Report System 边界决策

- 原始记录：归档批次 `2026-08-docs-corpus-reset/legacy-docs/prds/content/prd-autodream-2026-06-12/.decision-log.md`
- 日期：2026-06-12
- 状态：`Historical Boundary / Superseded by 2026-08 Workspace Reset where applicable`

## 保留的真实决策

- AutoDream 的 MVP 支持手动触发；自动模式必须满足会话门槛（默认至少 20 个 session），且默认关闭、由用户独立开启。
- 产生的记忆候选和演化建议必须经过用户审查；失败必须通知用户，不能静默丢失。
- AutoDream 是数据压缩/候选生成层；Report System 是用户呈现、固化、搜索和历史检索层。两者不因“日报/周报”名称相近而合并为一个权威。
- AutoDream 与 mask 的关系是统一蒸馏流程，而不是复制一套独立的 mask 体系。

## 当前关系

2026-08 的 Evolution Reset 已明确：旧 AutoDream–Evolution 混合链路不再作为当前架构依据，GenericAgent 研究完成前不冻结新的演化链路。本文仅保留手动优先、用户审查、失败可见和生成/呈现分层原则；当前边界以 `docs/architecture/README.md`、`docs/research/evolution-genericagent-reset-2026-08/` 和最新日志为准。
