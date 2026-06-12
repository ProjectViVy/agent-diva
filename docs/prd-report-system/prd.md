---
title: "Agent-Diva Pro 报表系统 & Session 历史检索 PRD"
created: 2026-06-08
updated: 2026-06-12
version: 1.1
revision_note: "v1.1 — 2026-06-12 scope-merge 边界分工修订（方案 C 批准）。详见 docs/architecture/scope-merge-decision.md。"
status: draft
---

# PRD: Agent-Diva Pro 报表系统 & Session 历史检索

## 0. Document Purpose

本文档面向 PM、架构师及下游实现团队，定义 Agent-Diva Pro 分支中**报表系统（Notebook）**与 **Session 历史检索**两大功能的完整需求。文档采用 Glossary 锚定词汇、Features 分组嵌套 FR、Assumptions 内联标注并索引的结构。本 PRD 建立在已有调研信息集（`docs/research/bmad-info-set-report-session.md`）之上，不重复其技术审计内容。

---

## 1. Vision

Agent-Diva 作为用户的全天候 AI 助手，每日产生大量对话与交互记录。用户需要一个自动化的**回顾与沉淀机制**：让 Diva 能够基于 session 历史自动生成日报、周报、月报，并支持将报告固化为可复用的知识资产（SOP、Skill、Memory）。同时，用户应能指令 Diva 主动搜索所有历史对话，快速定位过往讨论内容。

本功能让 Diva 从"用完即走"的对话工具，进化为**具备自我回顾、知识沉淀、历史检索能力的智能体**。

### 1.1 范围边界（v1.1 修订 — 2026-06-12）

依据 `docs/architecture/scope-merge-decision.md`，本 PRD v1.1 起做如下边界声明：

| 项目 | 归属 |
|------|------|
| **月报生成 (FR-3)** | ✅ 本 PRD 全权拥有 (独立 cron 调度) |
| **日报生成 (FR-1)** | ⚠️ 展示层归本 PRD, 实际生成由 AutoDream PRD FR-12 负责 |
| **周报生成 (FR-2)** | ⚠️ 展示层归本 PRD, 实际生成由 AutoDream PRD FR-13 负责 |
| **报表固化 (FR-6/7/8)** | ✅ 本 PRD 全权拥有 |
| **Session 智能搜索 (FR-9/10)** | ✅ 本 PRD 全权拥有 |
| **NotebookView (GUI)** | ✅ 本 PRD owner (commit `fcf768d`) |

**协作原则**: AutoDream 是数据压缩层 (蒸馏反思), 本 PRD 是用户呈现层 (沉淀消费)。两套子系统通过 markdown 文件路径契约解耦。详见 §13。

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

#### FR-1: 自动日报生成 (展示层 — v1.1 修订)

**v1.0 定义**: 系统每日自动基于前 24 小时的 session 历史生成日报。Realizes UJ-1.

**v1.1 修订**: 本 FR 仅负责"展示来自 AutoDream 的日报"和"用户手动触发生成"。Auto-generated 日报的实际生成由 AutoDream PRD FR-12 负责（见 `docs/prds/prd-autodream-2026-06-12/prd.md` §4.7）。边界详见 `docs/architecture/scope-merge-decision.md` §2。

**Consequences**:
- 日报展示在 GUI 的 Notebook 模块中（与周报/月报统一面板）。
- 日报展示内容来自 `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`（AutoDream 写入）。
- 用户可手动触发"重新生成"——手动触发会调用 AutoDream 的 `trigger_autodream` 并指定 `report_only=true`。
- 日报生成失败时，记录错误日志，不阻塞其他功能。

**Out of Scope**:
- 多语言日报生成（v2 考虑）。

---

#### FR-2: 自动周报生成 (展示层 — v1.1 修订)

**v1.0 定义**: 系统每周一自动基于本周的 session 历史生成周报。Realizes UJ-1.

**v1.1 修订**: 本 FR 仅负责"展示来自 AutoDream 的周报"和"用户手动触发生成"。Auto-generated 周报的实际生成由 AutoDream PRD FR-13 负责（见 `docs/prds/prd-autodream-2026-06-12/prd.md` §4.7）。

**Consequences**:
- 周报展示在 GUI 的 Notebook 模块中（与日报/月报统一面板）。
- 周报展示内容来自 `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`（AutoDream 写入）。
- 用户可手动触发"重新生成"。

---

#### FR-3: 自动月报生成 (本 PRD 全权拥有)

系统每月第一天（周一）自动基于本月的 session 历史生成月报。Realizes UJ-1.

**v1.1 修订**: 月报**不属于** AutoDream 蒸馏产物范围（AutoDream PRD §6.2 明确 P2 才有月报）。本 PRD 使用**独立 cron 调度**生成月报。

**Consequences**:
- 月报包含：月度对话统计、关键成果、知识沉淀、改进建议。
- 月报存储路径：`{workspace}/reports/monthly/{YYYY-MM}.md`（**注意**: 与日/周报不同，月报仍在本 PRD 管理的路径下）
- 月报在每月第一个周一 00:00 触发（若 Diva 处于运行状态）。
- 月报生成依赖本 PRD 自己的 cron 调度器，与 AutoDream 解耦。

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

### 6.1 In Scope (v1.1 修订)

- **月报的自动生成**（基于 session 历史，独立 cron 调度，本 PRD 全权拥有）。
- **日/周报的展示与手动触发**（实际生成由 AutoDream 负责，详见 `prd-autodream-2026-06-12/prd.md` FR-12/FR-13）。
- 手动触发生成。
- 报表查看（GUI 已就绪，commit `fcf768d`，NotebookView.vue 725 行）。
- 报表固化为 SOP/Skill/Memory。
- Agent 智能搜索（内存遍历 + 正则匹配）。
- Session 原子写入修复（当前 `SessionManager::save` 为非原子写入，直接覆写 `.jsonl` 文件；MVP 需改为临时文件写入后 rename，避免进程崩溃导致 session 数据丢失。见 `agent-diva-core/src/session/manager.rs:124`）。
- 与 AutoDream 的数据流契约（见 §13）。

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

---

## 13. Cross-PRD Interface (v1.1 新增)

> 依据 `docs/architecture/scope-merge-decision.md`，本节定义本 PRD 与 AutoDream PRD 的协作契约。

### 13.1 与 AutoDream PRD 的协作

| 方向 | 数据 | 路径 |
|------|------|------|
| AutoDream → 本 PRD | 日报 markdown | `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md` |
| AutoDream → 本 PRD | 周报 markdown | `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md` |
| 本 PRD → AutoDream | 手动触发信号 | `trigger_autodream({ trigger_type: 'manual', report_only: true })` |
| 本 PRD 自有 | 月报 markdown | `{workspace}/reports/monthly/{YYYY-MM}.md` |

### 13.2 调度边界

| 报表 | 调度器 | PRD owner | 触发时机 |
|------|--------|-----------|---------|
| 日报 | AutoDream 时间门 | AutoDream | 蒸馏运行完成后 |
| 周报 | AutoDream 时间门 | AutoDream | 蒸馏运行完成后 |
| 月报 | Report System cron | 本 PRD | 每月第一个周一 00:00 |
| 手动触发 | 任意时刻 | 本 PRD | 用户在 GUI 触发 |

### 13.3 读侧契约 (NotebookView 消费 AutoDream 产物)

- NotebookView **不订阅** AutoDream 内部事件
- NotebookView 切换到 `daily`/`weekly` 标签时，扫描 AutoDream 写入路径
- 扫描失败时降级到"暂无报告"提示，不阻塞 UI
- 写入失败的产物由 AutoDream 侧处理重试（不归本 PRD）

### 13.4 解耦保证

- 两份 PRD **不互相 import**，仅通过文件系统路径通信
- AutoDream 重构不影响本 PRD（只要路径契约不变）
- 本 PRD 重构不影响 AutoDream（只要消费方路径不变）
- 测试可独立进行：AutoDream 可 mock 写入路径，本 PRD 可 mock 读取路径

---

## v1.1 修订历史

- **2026-06-12**: 边界分工修订（方案 C 批准）。新增 §1.1 范围边界表、FR-1/2/3 owner 标注、§6.1 In Scope 更新、§13 Cross-PRD Interface。
- **2026-06-08**: v1.0 初版。10 FRs + 2 轮 validation (Fair+)。
