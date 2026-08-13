# 运行时审批边界

- 状态：`Current Runtime Boundary`
- 日期：2026-08-13
- 适用范围：危险工具执行、运行时能力授权和 Chat Approval Center

## 唯一职责

Chat Approval Center 负责需要人类确认的危险运行时动作，例如 shell、外部写入和其他
受策略约束的工具执行。它是运行时风险授权入口，不是跨领域内容治理中心。

## 不进入审批中心的动作

| 动作 | 处理方式 |
| --- | --- |
| BML Memory 增、删、改、查 | 直接作用于 BML 权威；使用精确目标、历史、软删除、撤销或恢复保护 |
| STM 自动更新与用户修正 | 直接更新 STM 权威；不创建 Proposal/Approval/Governance |
| Persona 用户直接保存 | 直接保存 Markdown，追加历史和审计 |
| Persona Agent 变更审查 | Persona 工作区内的专用内容审查；不映射为通用治理状态机 |
| 五份用户侧权威首次初始化（不含 `DREAM.MD`） | 一次原子直写；不经过 submit/approve/apply |
| Evolution 页面管理 | 以未来研究确认的 Skill 领域模型为准；不复用旧 Memory 治理链路 |

## 运行时约束

- 工具审批必须绑定请求、能力、资源范围、策略版本和内容摘要，不能跨版本复用授权。
- 审批状态、消费、过期和恢复由运行时安全实现负责；领域文档不得把 Memory 或 Persona
  内容塞进通用审批 payload。
- GUI 展示是审批状态的投影，不是授权真源。
- M3 三模式审批的实现与验证证据见 `docs/logs/2026-08-code-review-residuals/`；该日志
  目录保持原位，不复制到当前架构目录。

## 与旧治理文档的关系

旧 `governance-core-contract.md`、`governance-policy-evaluator.md` 和
`governance-approval-ledger.md` 的通用合同细节已归档。只有与危险工具运行时审批相容的
部分继续有效；它们不再授权 Memory、Persona、STM 或 Evolution 使用通用 Proposal/治理
生命周期。
