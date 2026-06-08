---
title: "Agent-Diva Pro 报表系统 & Session 历史检索 PRD"
created: 2026-06-08
updated: 2026-06-08
status: draft
---

# PRD: Agent-Diva Pro 报表系统 & Session 历史检索

## 0. Document Purpose

本文档面向 PM、架构师及下游实现团队，定义 Agent-Diva Pro 分支中**报表系统（Notebook）**与 **Session 历史检索**两大功能的完整需求。文档采用 Glossary 锚定词汇、Features 分组嵌套 FR、Assumptions 内联标注并索引的结构。本 PRD 建立在已有调研信息集（`docs/research/bmad-info-set-report-session.md`）之上，不重复其技术审计内容。

---

## 1. Vision

Agent-Diva 作为用户的全天候 AI 助手，每日产生大量对话与交互记录。用户需要一个自动化的**回顾与沉淀机制**：让 Diva 能够基于 session 历史自动生成日报、周报、月报，并支持将报告固化为可复用的知识资产（SOP、Skill、Memory）。同时，用户应能指令 Diva 主动搜索所有历史对话，快速定位过往讨论内容。

本功能让 Diva 从"用完即走"的对话工具，进化为**具备自我回顾、知识沉淀、历史检索能力的智能体**。

---

## 2. Target User

### 2.1 Jobs To Be Done

- **JT-1**: 作为用户，我希望 Diva 能自动总结我们的互动，让我快速回顾每日/每周/每月的进展。
- **JT-2**: 作为用户，我希望将 Diva 生成的报告沉淀为可复用的知识（SOP、Skill、Memory），避免重复劳动。
- **JT-3**: 作为用户，我希望能指令 Diva 搜索历史对话，快速找到之前的讨论内容。

### 2.2 Non-Users (v1)

- 不需要报表功能的用户（可通过配置关闭）。
- 不需要历史检索功能的用户（基础 session 管理已满足需求）。

### 2.3 Key User Journeys

#### UJ-1. 小明早晨查看 Diva 生成的日报

**Persona + context**: 小明是开发者，每天与 Diva 有大量技术对话。他希望快速了解昨天的讨论重点。

**Entry state**: 已登录 GUI，Notebook 标签页可见。

**Path**:
1. 小明打开 Notebook 标签页
2. 系统自动显示当日日报（若已生成）
3. 小明阅读日报摘要，点击感兴趣的部分查看详情
4. 小明觉得某段总结很有价值，点击"固化为 SOP"

**Climax**: 日报内容准确反映了昨日的技术讨论重点。

**Resolution**: 小明了解了昨日进展，并将有价值的知识沉淀为 SOP。

**Edge case**: 若日报尚未生成，显示"日报生成中"或"点击生成"。

---

#### UJ-2. 小红让 Diva 搜索上周关于"数据库优化"的讨论

**Persona + context**: 小红是技术负责人，需要回顾之前与 Diva 讨论的数据库优化方案。

**Entry state**: 在任意 session 中。

**Path**:
1. 小红发送指令："搜索之前关于数据库优化的讨论"
2. Diva 启动 Agent 智能搜索任务
3. Diva 遍历所有历史 session，正则匹配关键词
4. Diva 返回匹配的对话片段，标注 session 和时间

**Climax**: Diva 准确找到了之前的讨论内容。

**Resolution**: 小红回顾了数据库优化方案，无需重新讨论。

**Edge case**: 若未找到匹配内容，Diva 应明确告知"未找到相关讨论"。

---

## 3. Glossary

- **Report / 报表** — 基于 session 历史自动生成的总结性文档，分为日报（Daily）、周报（Weekly）、月报（Monthly）。
- **Notebook** — GUI 中用于查看和管理报表的模块。
- **Session** — 一次完整的对话记录，以 JSONL 文件形式持久化存储。
- **SOP (Standard Operating Procedure)** — 标准操作流程，将报告中的经验固化为可执行步骤。
- **Skill** — Diva 的可复用能力单元，参考 Hermes/GenericAgent 的 skill 格式。
- **Memory** — Diva 的长期记忆，用于跨 session 保持上下文。
- **Agent 智能搜索** — 将搜索任务作为一个 session 任务执行，由 Diva 主动完成搜索并返回结果。
- **Session 任务** — Diva 在后台执行的异步任务，不阻塞用户当前对话。

---

## 4. Features

### 4.1 报表自动生成

**Description**: 基于 session 历史自动生成日报、周报、月报。支持定时自动生成和用户手动触发。Report 以独立 Markdown 文件形式存储。

**Functional Requirements**:

#### FR-1: 自动日报生成

系统每日自动基于前 24 小时的 session 历史生成日报。Realizes UJ-1.

**Consequences**:
- 日报包含：对话摘要、关键决策、完成的任务、待跟进事项。
- 日报存储路径：`{workspace}/reports/daily/{YYYY-MM-DD}.md`
- 日报生成失败时，记录错误日志，不阻塞其他功能。
- **节律点触发规则**:
  - 若当前时间到达 00:00 且 Diva 处于运行状态，自动触发生成。
  - 若今日日报尚未生成，且昨日聊天记录数 > 0，则触发生成（补生成）。

**Out of Scope**:
- 多语言日报生成（v2 考虑）。

---

#### FR-2: 自动周报生成

系统每周一自动基于本周的 session 历史生成周报。Realizes UJ-1.

**Consequences**:
- 周报包含：本周对话概览、关键成果、问题与风险、下周计划。
- 周报存储路径：`{workspace}/reports/weekly/{YYYY-Www}.md`
- 周报在每周一 00:00 触发（若 Diva 处于运行状态）。

---

#### FR-3: 自动月报生成

系统每月第一天（周一）自动基于本月的 session 历史生成月报。Realizes UJ-1.

**Consequences**:
- 月报包含：月度对话统计、关键成果、知识沉淀、改进建议。
- 月报存储路径：`{workspace}/reports/monthly/{YYYY-MM}.md`
- 月报在每月第一个周一 00:00 触发（若 Diva 处于运行状态）。

---

#### FR-4: 手动触发生成

用户可在 GUI 中手动触发任意周期报表的生成。Realizes UJ-1.

**Consequences**:
- 手动生成的报表与自动生成的报表格式一致。
- 手动生成支持"强制刷新"，覆盖已有报表。

---

#### FR-5: 报表查看

用户在 GUI 的 Notebook 模块中查看已生成的报表。Realizes UJ-1.

**Consequences**:
- 支持按 daily/weekly/monthly 过滤。
- 支持报表列表和详情双栏布局。
- 支持 Markdown 渲染（含代码高亮）。

---

### 4.2 报表固化

**Description**: 支持将报表固化为 SOP、Skill 或更新长期记忆。参考 Hermes 和 GenericAgent 的实现。

**Functional Requirements**:

#### FR-6: 固化为 SOP

用户可将报表中的内容固化为标准操作流程文档。Realizes UJ-1.

**Consequences**:
- SOP 以 Markdown 文件形式存储：`{workspace}/sops/{report-id}.md`
- SOP 包含：目的、步骤、注意事项、参考链接。
- **格式参考**: GenericAgent SOP 格式（见 `.workspace/` 目录）。

---

#### FR-7: 固化为 Skill

用户可将报表中的内容固化为 Diva 的 Skill。Realizes UJ-1.

**Consequences**:
- Skill 格式参考 GenericAgent skill 规范（见 `.workspace/` 目录）。
- Skill 存储路径：`{workspace}/skills/{skill-name}.md`

---

#### FR-8: 更新长期记忆

用户可将报表中的关键信息更新到 Diva 的长期记忆中。Realizes UJ-1.

**Consequences**:
- 更新目标 memory provider（由配置决定，当前正在合并 memory 分支）。
- 支持去重：已存在的记忆不重复写入。

---

### 4.3 Session 历史检索

**Description**: 支持 Agent 智能搜索所有历史 session，快速定位过往讨论内容。

**Functional Requirements**:

#### FR-9: Agent 智能搜索

用户可发送自然语言指令，让 Diva 搜索历史对话。Realizes UJ-2.

**Consequences**:
- 搜索任务作为 session 任务异步执行。
- 搜索范围：所有历史 session。
- 搜索方式：内存遍历 + 正则匹配（短期方案）。
- 返回结果：匹配的 message 列表（含 session key、timestamp、content）。

---

#### FR-10: 搜索结果返回

Diva 将搜索结果返回给用户。Realizes UJ-2.

**Consequences**:
- 返回格式：文本列表或结构化 JSON。
- 不要求可视化展示（可通过 API/命令行返回）。
- 未找到时明确告知"未找到相关讨论"。

---

## 5. Non-Goals (Explicit)

- **多语言报表生成**（v2 考虑）。
- **报表可视化图表**（如对话量趋势图）。
- **语义搜索**（v2 考虑，当前仅支持关键词/正则匹配）。
- **跨设备报表同步**（当前仅本地存储）。

---

## 6. MVP Scope

### 6.1 In Scope

- 日报/周报/月报的自动生成（基于 session 历史）。
- 手动触发生成。
- 报表查看（GUI 已就绪）。
- 报表固化为 SOP/Skill/Memory。
- Agent 智能搜索（内存遍历 + 正则匹配）。
- Session 原子写入修复（cherry-pick main 的 `write_session_atomically`）。

### 6.2 Out of Scope for MVP

| 项目 | 原因 | 计划版本 |
|------|------|---------|
| 多语言报表 | 用户量不足 | v2 |
| 语义搜索 | 需要 embedding 模型 | v2 |
| 可视化图表 | 非核心需求 | v2 |
| 跨设备同步 | 需要云端存储 | v3 |

---

## 7. Success Metrics

**Primary**:
- **SM-1**: 日报生成成功率 > 95%。Validates FR-1.
- **SM-2**: 用户每周至少查看 1 次报表。Validates FR-5.
- **SM-3**: 历史搜索平均响应时间 < 5s（100 个 session 以内）。Validates FR-9.

**Secondary**:
- **SM-4**: 报表固化功能使用率 > 30%。Validates FR-6, FR-7, FR-8.

**Counter-metrics**:
- **SM-C1**: 避免为追求生成速度而牺牲报表质量。

---

## 8. Open Questions

1. ~~日报生成时间~~ → **已确认**: 00:00 自动触发，若未生成且昨日聊天记录 > 0 则补生成。
2. ~~周报起始日~~ → **已确认**: 每周一。
3. ~~SOP/Skill 格式~~ → **已确认**: 参考 GenericAgent（`.workspace/` 目录）。
4. ~~Memory Provider~~ → **已确认**: 正在合并 memory 分支，完成后统一配置。
5. ~~搜索权限~~ → **已确认**: 暂时不考虑权限控制。

---

## 9. Assumptions Index

- **ASSUMPTION-1**: Session 历史数据完整且可读取（依赖 session 持久化修复）。
- **ASSUMPTION-2**: LLM 调用能力可用（用于报表生成）。
- **ASSUMPTION-3**: 用户对报表内容的质量要求以"可用"为标准，而非"完美"。
- **ASSUMPTION-4**: 历史搜索的性能瓶颈在可接受范围内（短期方案）。

---

## 10. Cross-Cutting NFRs

### 10.1 性能

- 报表生成应在后台异步完成，不阻塞用户当前对话。
- 历史搜索应在 5s 内返回结果（100 个 session 以内）。

### 10.2 可靠性

- 报表生成失败应记录日志，不影响其他功能。
- Session 写入应采用原子写入，避免数据丢失。

### 10.3 安全性

- 报表文件应存储在用户本地，不自动上传云端。
- 历史搜索应仅搜索当前用户的 session。

---

## 11. Constraints and Guardrails

### 11.1 安全

- 报表内容可能包含敏感信息，应存储在用户可控的本地目录。
- SOP/Skill 固化时应避免覆盖用户已有文件。

### 11.2 隐私

- 历史搜索仅搜索当前用户的 session，不跨用户。
- 报表生成不将数据发送到外部服务（除 LLM API 外）。

### 11.3 成本

- 报表生成依赖 LLM 调用，应控制调用频率（每日/每周/每月一次）。
- 历史搜索应避免全量加载所有 session（未来优化方向）。

---

## 12. References

- `docs/research/bmad-info-set-report-session.md` — BMad 信息集
- `docs/research/report-session-research-info-set.md` — 技术调研报告
- `agent-diva-gui/src/components/NotebookView.vue` — GUI 实现
- `.workspace/genericagent/` — GenericAgent 参考项目（SOP/Skill 格式）
