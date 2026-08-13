# Cognitive Workspace 当前架构边界

- 状态：`Product Boundaries Frozen / Research Pending / Target Architecture Blocked`
- 日期：2026-08-14
- 权威来源：[Cognitive Workspace Reset EPIC](../../research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md)
- Laputa 汇总：[../laputa/architecture.md](../laputa/architecture.md)

## 四个用户工作区

| 工作区 | 唯一职责 | 明确排除 |
| --- | --- | --- |
| Persona | `IDENTITY.MD`（含身体）/ `RELATIONSHIP.MD` / `REDLINE.MD` / `USER.MD` / `DREAM.MD` / `DARK.MD` / `WORLD.MD`、内容审查、完整历史 | 普通 Memory、STM、Skill、通用安全审批、JSON 编辑、AutoDream |
| Memory | BML 长期记忆管理，以及独立的跨会话 STM 入口 | Persona、Evolution、治理提案、文件型长期记忆 |
| Evolution | Skill 的形成、验证、管理与复用；SOP 关系待研究 | Persona 沉淀、普通 Memory、旧 AutoDream 流水线 |
| Chat Approval Center | 危险工具执行等真正需要人类授权的运行时审批 | Memory CRUD、Persona 初始化、STM 日常维护、Evolution 页面治理 |

## 单一权威与生命周期边界

- BML typed SQLite/FTS5 是普通长期 Memory 的唯一生产权威。
- `memory_md` / `MemoryMd` 文件型长期记忆链路属于 clean-break 删除范围。
- STM 是 workspace/profile 级、自动维护、有界、跨 session 的活动工作集；它不是长期
  Memory、transcript、canonical checkpoint 或旧 `working_memory` checkpoint。
  概念上仍称 STM / LTM；权威文件是全局一份 `{config_dir}/actmem/ACTMEM.MD`，
  禁止核心文件叫 `STM.MD` / `MEMORY.MD`。发言即写 Pulse；空闲 10 分钟写胶囊。
  子代理不进。正文不装配。CORE 一个查询；管理工具 DEFER。
  `ACTMEM.MD` 不是 Persona 七文件；七份人格同一目录、一个 Diva 一套。
- Laputa 权威文件为 `IDENTITY.MD`、`RELATIONSHIP.MD`、`REDLINE.MD`、`USER.MD`、
  `DREAM.MD`、`DARK.MD`、`WORLD.MD`。正文以 Markdown 为依据。`IDENTITY.MD` 含当前
  身体/形态。`DARK.MD` 为 FEAR/SHADOW 两展位，不单开 BODY/FEAR/SHADOW 文件。
  `DREAM.MD` Frozen Core 投影严格 10 字。`WORLD.MD` 不进 Frozen Core，不动态装配，
  工具读写。
  v1 种类闭集；实现按登记表可加，用户不能自增权威种类。
- Persona 内容审查是文档领域动作，不是工具风险授权；聊天 Approval Center 不参与其中。
- Memory CRUD、STM 日常维护和 Persona 首次初始化不创建 Proposal、Approval 或
  Governance Ledger 记录。AutoDream 批处理必须整理 STM（直写，与聊天 Agent 日常
  维护同一对象）；人格整理只允许按 P19 提案，不能 apply。

## 当前禁止的推断

以下内容尚未完成研究，不得由实现者自行决定：STM 物理存储、schema、scope、并发合并、
自动更新时机、预算和淘汰、Layer 1 装配、Evolution 的 SOP/Skill 状态机、Persona revision
store 的具体目录和 API、clean-break 删除切片及迁移/备份流程。

## 实施门禁

1. R0–R4 研究完成并经用户评审；
2. D0–D4 架构设计完成并经用户批准；
3. 从已验证提交建立保护性分支；
4. 分域实施、零残留证明、全量自动测试和真实桌面验收完成。

详细问题、完成物和验收条件以 EPIC 为准，本摘要不替代研究记录。
