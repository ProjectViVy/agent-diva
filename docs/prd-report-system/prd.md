---
title: "Agent-Diva Pro 报表系统 & Session 历史检索 PRD"
created: 2026-06-08
updated: 2026-06-12
version: 1.2
revision_note: |
  v1.2 — 2026-06-12 P1 修复批:
  - §4.0 引入 AC 约定 + FR-1~10 全部补充 G/W/T 验收标准
  - §4.4 新增报表输出 Schema (frontmatter + markdown 模板 + 演进规则)
  - FR-6/7 补充 SOP/Skill 具体格式引用 (项目内范例文件路径)
  - §4.2 显式声明固化依赖关系
  - §6.1 N1/N2/N3 修复 (错误处理 + 并发控制 + 时区)
  - §10.4 LLM 选型 + 成本 Guardrail (5 候选模型 + 4 决策方案, 待大湿拍板)
  - §10.5 搜索演进路线 (v1→v2 SQLite FTS5→v3 向量检索, 切换条件明确)
  - §6.1 GUI 验证状态诚实更新 (P0-2 仅源码验证, 测试/覆盖率/评审待补)
  v1.1 — 2026-06-12 scope-merge 边界分工修订（方案 C 批准）。详见 docs/architecture/scope-merge-decision.md。
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

### 4.0 Acceptance Criteria 约定 (v1.2 新增)

所有 FR 均补充 Given-When-Then 格式的验收标准。下游工程师、QA 可据此:
- 写单元测试 (`#[test]` Given/When 桩, Then 断言)
- 写集成测试 (e2e 端到端 Given/When)
- 写验收清单 (checklist 形式)

**模板**:
```markdown
**Acceptance Criteria**:
- **AC-1**: Given <前置条件>, When <触发动作>, Then <可观察结果>.
- **AC-2**: Given <错误条件>, When <触发动作>, Then <错误处理行为>.
- **AC-3**: <边界条件或非功能约束>.
```

**优先级标注**:
- 🔴 P0: 必须通过 (MVP 阻塞)
- 🟡 P1: 应当通过 (生产阻塞)
- 🟢 P2: 最好通过 (可后续优化)

### 4.1 报表自动生成

**Description**: 基于 session 历史自动生成日报、周报、月报。支持定时自动生成和用户手动触发。Report 以独立 Markdown 文件形式存储。

**Functional Requirements**:

#### FR-1: 自动日报生成 (展示层 — v1.1 修订)

**v1.0 定义**: 系统每日自动基于前 24 小时的 session 历史生成日报。Realizes UJ-1.

**v1.1 修订**: 本 FR 仅负责"展示来自 AutoDream 的日报"和"用户手动触发生成"。Auto-generated 日报的实际生成由 AutoDream PRD FR-12 负责（见 `docs/prds/prd-autodream-2026-06-12/prd.md` §4.7）。边界详见 `docs/architecture/scope-merge-decision.md` §2。

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-1.1**: Given AutoDream 已成功写入 `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`, When 用户打开 Notebook 切到 daily 标签, Then 该日报被列出且详情面板渲染 markdown 内容.
- 🔴 **AC-1.2**: Given 当前日期的日报不存在, When 用户打开 Notebook daily 标签, Then 显示"暂无日报"占位提示.
- 🟡 **AC-1.3**: Given 用户点击"生成日报"按钮, When 按钮被点击, Then 调用 `trigger_autodream({ trigger_type: 'manual', report_only: true })`, 触发成功后刷新列表.
- 🟡 **AC-1.4**: Given Auto-generated 日报写入失败 3 次, When 仍是当前日期, Then GUI 顶部 toast 提示"日报生成失败，请稍后重试", 不阻塞其他标签切换.
- 🟢 **AC-1.5**: Given 同一日报 markdown 文件超过 5MB, When 渲染详情, Then 自动截断至 5000 行并提示"内容过长，已截断".

**Consequences**:
- 日报展示在 GUI 的 Notebook 模块中（与周报/月报统一面板）。
- 日报展示内容来自 `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`（AutoDream 写入）。
- 用户可手动触发"重新生成"——手动触发会调用 AutoDream 的 `trigger_autodream` 并指定 `report_only=true`。
- 日报生成失败时，记录错误日志，不阻塞其他功能。
- **N1 错误处理**: "失败" 定义为: (a) LLM 调用超时 (> 60s), (b) 写入磁盘失败 (磁盘满/权限拒绝), (c) markdown 解析失败. 失败时 GUI toast + 写 `.agent-diva/autodream/events.jsonl`.
- **N2 并发控制**: 同一日期的日报**单写**——若 AutoDream 报告运行中 (lock 文件存在), 手动触发按钮 disabled.
- **N3 时区**: 日报日期边界按**用户系统本地时区**的 00:00 计算 (Windows/Mac/Linux 默认设置). 跨时区用户应在 settings 配置 `report_timezone` (P1 特性, v1 用系统时区).

**Out of Scope**:
- 多语言日报生成（v2 考虑）。

---

#### FR-2: 自动周报生成 (展示层 — v1.1 修订)

**v1.0 定义**: 系统每周一自动基于本周的 session 历史生成周报。Realizes UJ-1.

**v1.1 修订**: 本 FR 仅负责"展示来自 AutoDream 的周报"和"用户手动触发生成"。Auto-generated 周报的实际生成由 AutoDream PRD FR-13 负责（见 `docs/prds/prd-autodream-2026-06-12/prd.md` §4.7）。

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-2.1**: Given AutoDream 已成功写入 `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`, When 用户打开 Notebook 切到 weekly 标签, Then 该周报被列出且详情面板渲染 markdown 内容.
- 🔴 **AC-2.2**: Given 当前周的周报不存在, When 用户打开 Notebook weekly 标签, Then 显示"暂无周报"占位.
- 🟡 **AC-2.3**: Given 周报已存在, When 用户点击"重新生成", Then 弹出确认对话框, 确认后调用 `trigger_autodream` 覆盖写.
- 🟡 **AC-2.4**: Given 周一 00:00 触发时间已过且本周无周报, When 用户手动点击生成, Then 同 FR-1 AC-1.3 路径.
- 🟢 **AC-2.5**: Given 周报跨多份 session (>100 个), When 渲染详情, Then markdown 列表项使用折叠面板 lazy load.

**Consequences**:
- 周报展示在 GUI 的 Notebook 模块中（与日报/月报统一面板）。
- 周报展示内容来自 `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`（AutoDream 写入）。
- 用户可手动触发"重新生成"。
- 周编号格式: ISO 8601 `YYYY-Www`（如 `2026-W24`），周起始日为周一。

---

#### FR-3: 自动月报生成 (本 PRD 全权拥有)

系统每月第一天（周一）自动基于本月的 session 历史生成月报。Realizes UJ-1.

**v1.1 修订**: 月报**不属于** AutoDream 蒸馏产物范围（AutoDream PRD §6.2 明确 P2 才有月报）。本 PRD 使用**独立 cron 调度**生成月报。

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-3.1**: Given 当前日期是某月第一个周一且 00:00 已过, When 调度器触发, Then 生成月报写入 `{workspace}/reports/monthly/{YYYY-MM}.md`.
- 🔴 **AC-3.2**: Given 月报已成功生成, When 用户打开 Notebook monthly 标签, Then 月报按月份倒序排列, 详情面板渲染.
- 🟡 **AC-3.3**: Given 月报生成时 LLM 调用超时 (N1 定义), When 重试 3 次后仍失败, Then 写入 error marker 文件 `{workspace}/reports/monthly/{YYYY-MM}.error.json` 记录失败原因, GUI 显示"月报生成失败"状态.
- 🟡 **AC-3.4**: Given 用户在 GUI 手动触发月报, When 点击"重新生成", Then 走 FR-4 路径并强制覆盖.
- 🟢 **AC-3.5**: Given 月报生成消耗 LLM token > 预算 (见 §10.4), When 超预算, Then 自动降级到摘要模式 (减少 50% 输入) 并记录到 events.jsonl.

**Consequences**:
- 月报包含：月度对话统计、关键成果、知识沉淀、改进建议。
- 月报存储路径：`{workspace}/reports/monthly/{YYYY-MM}.md`（**注意**: 与日/周报不同，月报仍在本 PRD 管理的路径下）
- 月报在每月第一个周一 00:00 触发（若 Diva 处于运行状态）。
- 月报生成依赖本 PRD 自己的 cron 调度器，与 AutoDream 解耦。
- 月报生成可使用 LLM（成本较高，§10.4 定义模型选型与 cost guardrail）。

---

#### FR-4: 手动触发生成

用户可在 GUI 中手动触发任意周期报表的生成。Realizes UJ-1.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-4.1**: Given 用户在 Notebook 任意标签 (daily/weekly/monthly), When 点击"重新生成"按钮, Then 弹出确认对话框显示 "确定重新生成 X 报？现有内容将被覆盖".
- 🔴 **AC-4.2**: Given 用户确认重新生成, When 确认按钮被点击, Then 触发对应调度 (AutoDream for 日/周, 本 PRD cron for 月), 按钮变为"生成中..."且 disabled.
- 🟡 **AC-4.3**: Given 触发已成功, When 报表生成完成, Then 列表自动刷新, 详情面板显示最新内容, toast 提示"X 报已更新".
- 🟡 **AC-4.4**: Given 触发后 30s 内未完成, When 用户继续浏览, Then 后台继续生成, 不阻塞 UI.
- 🟡 **AC-4.5**: Given 报表已存在且无重大修改 (< 1KB), When 用户点击生成, Then 弹出警告"现有 X 报内容较少，确认覆盖？".

**Consequences**:
- 手动生成的报表与自动生成的报表格式一致（见 §4.4 schema）。
- 手动生成支持"强制刷新"，覆盖已有报表。
- 手动生成会经过 LLM 成本 guardrail（见 §10.4）：若当日手动生成次数 > 5 次，弹警告"今日手动生成次数较多，是否继续？"。

---

#### FR-5: 报表查看

用户在 GUI 的 Notebook 模块中查看已生成的报表。Realizes UJ-1.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-5.1**: Given 用户打开 Notebook 视图, When 首次加载, Then 默认显示 daily 标签, 左侧列出最近 30 份日报, 右侧显示选中日报详情.
- 🔴 **AC-5.2**: Given 用户切换标签 (daily/weekly/monthly), When 切换完成, Then 列表与详情同步刷新.
- 🟡 **AC-5.3**: Given 选中 markdown 内容, When 渲染详情, Then 代码块语法高亮 (使用 Shiki 或 highlight.js), 表格/列表正确渲染.
- 🟡 **AC-5.4**: Given 列表项 > 50, When 渲染列表, Then 启用虚拟滚动 (virtual scroller), 仅渲染可视区域.
- 🟢 **AC-5.5**: Given 列表为空, When 加载, Then 显示空状态插画 + "暂无报表"提示 + "立即生成"按钮 (跳转 FR-4).

**Consequences**:
- 支持按 daily/weekly/monthly 过滤。
- 支持报表列表和详情双栏布局（NotebookView 现有实现）。
- 支持 Markdown 渲染（含代码高亮）。
- 双栏布局交互: 列表项 hover 显示"操作菜单"（删除/导出/固化），点击进入详情；详情底部"操作栏"提供"固化为 SOP/Skill/Memory"（FR-6/7/8）。

---

### 4.2 报表固化

**Description**: 支持将报表固化为 SOP、Skill 或更新长期记忆。参考 Hermes 和 GenericAgent 的实现。

**依赖关系 (v1.2 新增)**: 固化功能**仅当关联报表已成功生成且内容非空时可用**。
- FR-6 依赖 FR-1/2/3 (SOP 内容来源于已生成的报表)
- FR-7 依赖 FR-1/2/3 (Skill 内容来源于已生成的报表)
- FR-8 依赖 FR-1/2/3 (记忆更新内容来源于已生成的报表)
- 若源报表生成失败或文件不存在, 固化按钮 disabled 并显示 tooltip "请先生成报表"

**Functional Requirements**:

#### FR-6: 固化为 SOP

用户可将报表中的内容固化为标准操作流程文档。Realizes UJ-1.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-6.1**: Given 报表已生成且详情面板可见, When 用户点击底部"固化为 SOP", Then 弹出对话框, 显示报表内容预览 + SOP 模板.
- 🔴 **AC-6.2**: Given 用户编辑 SOP 内容 (目的/步骤/注意事项/参考链接), When 点击"保存", Then SOP 写入 `{workspace}/sops/{report-id}-{timestamp}.md` 路径.
- 🟡 **AC-6.3**: Given SOP 路径已存在同名文件, When 用户保存, Then 弹冲突对话框 "已存在 X SOP, 是否覆盖/重命名/取消？".
- 🟡 **AC-6.4**: Given SOP 成功保存, When 保存完成, Then toast 提示"已固化为 SOP", GUI 可在 SOP 管理面板中查看.
- 🟢 **AC-6.5**: Given SOP 模板由 LLM 生成, When LLM 不可用, Then 降级到纯复制报表内容 + 简单分节, 不阻塞.

**Consequences**:
- SOP 以 Markdown 文件形式存储：`{workspace}/sops/{report-id}-{timestamp}.md`
- SOP 包含：目的、步骤、注意事项、参考链接。
- **格式参考 (v1.2 具体化)**: SOP 模板结构如下，参考 `docs/prd-report-system/sop-template.md` (本目录新增模板文件):
  ```markdown
  # {SOP 标题}
  
  ## 目的
  {从报表中提取的目的描述}
  
  ## 前置条件
  - {依赖的报表: report-system://monthly/2026-06}
  - {依赖的工具/技能}
  
  ## 步骤
  1. {步骤 1}
  2. {步骤 2}
  
  ## 注意事项
  - {注意事项}
  
  ## 参考链接
  - {源报表 URL/path}
  ```

---

#### FR-7: 固化为 Skill

用户可将报表中的内容固化为 Diva 的 Skill。Realizes UJ-1.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-7.1**: Given 报表已生成, When 用户点击"固化为 Skill", Then 弹出对话框要求填写 skill name (lowercase + hyphens, e.g. `analyze-monthly-trends`).
- 🔴 **AC-7.2**: Given skill name 校验通过, When 用户点击保存, Then 生成 SKILL.md 写入 `{workspace}/skills/{skill-name}/SKILL.md` 路径.
- 🟡 **AC-7.3**: Given skill name 包含非法字符 (大写/空格/下划线), When 校验, Then 输入框红框 + 错误提示"skill name 必须 lowercase + hyphens".
- 🟡 **AC-7.4**: Given skill 已存在同名, When 保存, Then 弹冲突对话框 (同 FR-6 AC-6.3).
- 🟢 **AC-7.5**: Given skill 路径下已有其他文件, When 保存, Then 不影响现有文件, 仅新增/覆盖 SKILL.md.

**Consequences**:
- **Skill 格式规范 (v1.2 具体化)**: SKILL.md 必须包含 YAML frontmatter, 字段 `name` + `description`. 完整模板:
  ```markdown
  ---
  name: {skill-name}
  description: {skill 描述, 含 trigger context}
  ---
  
  # {Skill 标题}
  
  {Skill 主体内容}
  ```
- **格式参考**: 项目内已有 skill 范例 `.workspace/cherry-studio/.agents/skills/create-skill/SKILL.md` (3.2KB 范例), 以及 `.workspace/cherry-studio/.agents/skills/gh-create-pr/SKILL.md` 等 5 个现有 skill.
- **通用参考**: Hermes Agent skill 规范 `~/.hermes/skills/` (e.g. `~/.hermes/skills/bmad-method/2-plan-workflows/bmad-agent-pm/SKILL.md`).
- Skill 存储路径：`{workspace}/skills/{skill-name}/SKILL.md`（注意是**目录**结构, 非单文件）
- Skill 文件创建时由 LLM 根据报表内容生成, 需校验 frontmatter schema (使用 `quick_validate.py` 类似工具).

---

#### FR-8: 更新长期记忆

用户可将报表中的关键信息更新到 Diva 的长期记忆中。Realizes UJ-1.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-8.1**: Given 报表已生成, When 用户点击"更新记忆", Then LLM 提取报表中的关键事实/偏好/决策, 显示候选列表 (含原文引用 + 置信度).
- 🔴 **AC-8.2**: Given 用户选择若干候选 (可全选/部分选/编辑), When 点击"确认写入", Then 写入 `MEMORY.md` 路径 (走 memory provider).
- 🟡 **AC-8.3**: Given 候选信息与现有记忆重复 (相似度 > 0.9), When 提交, Then 提示"X 条候选与现有记忆重复，跳过/合并？".
- 🟡 **AC-8.4**: Given 用户编辑候选内容, When 保存, Then 记录"用户已编辑"到审计日志.
- 🟢 **AC-8.5**: Given memory provider 不可用 (e.g. mask 隔离蒸馏未就绪), When 写入, Then 降级到待处理队列, 稍后重试.

**Consequences**:
- 更新目标 memory provider (由配置决定, 当前正在合并 memory 分支, 完成前降级到本地 JSON 文件 `{workspace}/memory/pending.jsonl`).
- 支持去重：已存在的记忆不重复写入 (使用 embedding 相似度检测, 阈值 0.9, 见 §10.5 搜索演进路线 v2 阶段).
- 记忆更新不阻塞报表展示 (写入操作走异步队列).

---

### 4.3 Session 历史检索

**Description**: 支持 Agent 智能搜索所有历史 session，快速定位过往讨论内容。

**Functional Requirements**:

#### FR-9: Agent 智能搜索

用户可发送自然语言指令，让 Diva 搜索历史对话。Realizes UJ-2.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-9.1**: Given 用户发送自然语言查询, When Diva 解析后判定需要历史检索, When 启动搜索任务, Then 任务作为 session 任务异步执行 (不阻塞当前对话).
- 🔴 **AC-9.2**: Given 搜索任务启动, When 遍历 100 个 session 以内, Then 平均响应时间 < 5s (SM-3).
- 🟡 **AC-9.3**: Given 搜索匹配到结果, When 整理返回, Then 返回按时间倒序, 每条结果含 session key + timestamp + content snippet (前后 200 字符).
- 🟡 **AC-9.4**: Given 搜索范围包含 100-1000 个 session, When 仍走内存遍历, Then 响应时间允许 < 30s, GUI 显示进度条.
- 🟢 **AC-9.5**: Given 用户输入的查询无匹配, When 搜索完成, Then 明确返回"未找到相关讨论" + 建议用户换关键词.

**Consequences**:
- 搜索任务作为 session 任务异步执行。
- 搜索范围：所有历史 session。
- 搜索方式：**v1 = 内存遍历 + 正则匹配** (短期方案, 性能瓶颈见 §10.5 演进路线).
- 返回结果：匹配的 message 列表（含 session key、timestamp、content）。

---

#### FR-10: 搜索结果返回

Diva 将搜索结果返回给用户。Realizes UJ-2.

**Acceptance Criteria (v1.2 新增)**:
- 🔴 **AC-10.1**: Given 搜索完成, When 返回结果, Then 以 **结构化 JSON** 格式 (见下) 返回给 GUI.
- 🔴 **AC-10.2**: Given 搜索结果 >= 1 条, When 渲染, Then 在当前对话上下文中以消息列表形式展示.
- 🟡 **AC-10.3**: Given 搜索结果 0 条, When 返回, Then 显式告知"未找到相关讨论", 建议尝试近义词.
- 🟢 **AC-10.4**: Given 搜索任务超时 (>30s), When 超时, Then 降级为"搜索耗时较长，正在后台继续..."提示.

**Consequences**:
- **返回 JSON Schema (v1.2 具体化)**:
  ```json
  {
    "query": "用户原始查询",
    "searched_at": "ISO8601 timestamp",
    "total_matches": 5,
    "elapsed_ms": 1234,
    "results": [
      {
        "session_key": "{session uuid}",
        "session_date": "2026-06-10T14:23:00Z",
        "message_index": 42,
        "role": "user|assistant",
        "content_snippet": "前后各 200 字符的摘要",
        "match_type": "exact|regex|fuzzy",
        "relevance_score": 0.87
      }
    ]
  }
  ```
- 不要求可视化展示（可通过 API/命令行返回）。
- 未找到时明确告知"未找到相关讨论"。

### 4.4 报表输出 Schema (v1.2 新增)

**Markdown Frontmatter (强制, v1 schema)**:
```yaml
---
period: daily | weekly | monthly          # 必填
date: YYYY-MM-DD                          # daily 必填
week: YYYY-Www                            # weekly 必填 (ISO 8601)
month: YYYY-MM                            # monthly 必填
generated_at: ISO8601 timestamp           # 必填
generated_by: agent-diva-report-system    # 必填 (固定值)
source: auto | manual                     # 必填, 区分自动/手动触发
session_count: integer                    # 必填, 涵盖的 session 数
token_used: integer                       # 必填, LLM token 消耗
schema_version: 1                         # 必填, 当前为 1
---
```

**Markdown Body 模板 (v1)**:
```markdown
# {period_label} {date_or_week_or_month}

## 概览
- **覆盖时段**: {start_time} → {end_time}
- **Session 数**: {session_count}
- **LLM 消耗**: {token_used} tokens

## 关键主题
1. {主题 1} (出现 {N} 次)
2. {主题 2} (出现 {N} 次)
3. ...

## 关键决策
- {决策 1}
- {决策 2}

## 完成的任务
- [x] {任务 1}
- [x] {任务 2}

## 待跟进事项
- [ ] {事项 1} (优先级: {P0/P1/P2})
- [ ] {事项 2}

## 情绪趋势
（仅日报/周报包含）
- 平均情绪: {positive/neutral/negative}
- 峰值: {峰值时间} {情绪}

## 知识沉淀
（仅周报/月报包含）
- {新 Skill: skill-name}
- {新 SOP: sop-name}
- {新记忆: memory-id}

## 改进建议
（仅月报包含）
- {建议 1}
- {建议 2}

---
*Generated by agent-diva-report-system at {generated_at}*
*Source: {source} | Schema: v{schema_version}*
```

**Schema 演进规则 (v1.2 新增)**:
- schema_version 字段强制, 任何破坏性变更必须 bump major
- 新增可选字段允许 minor bump, 必须向后兼容
- 老 schema 报表**只读**, 不自动迁移
- 迁移工具: `agent-diva-report-system migrate --from v1 --to v2` (P1 实现)

---

## 5. Non-Goals (Explicit)

- **多语言报表生成**（v2 考虑）。
- **报表可视化图表**（如对话量趋势图）。
- **语义搜索**（v2 考虑，当前仅支持关键词/正则匹配）。
- **跨设备报表同步**（当前仅本地存储）。

---

## 6. MVP Scope

### 6.1 In Scope (v1.1 修订, v1.2 增补)

- **月报的自动生成**（基于 session 历史，独立 cron 调度，本 PRD 全权拥有）。
- **日/周报的展示与手动触发**（实际生成由 AutoDream 负责，详见 `prd-autodream-2026-06-12/prd.md` FR-12/FR-13）。
- 手动触发生成。
- 报表查看（GUI 已就绪，commit `fcf768d`，NotebookView.vue 17607 bytes / 725 行）。
- 报表固化为 SOP/Skill/Memory。
- Agent 智能搜索（内存遍历 + 正则匹配）。
- Session 原子写入修复（当前 `SessionManager::save` 为非原子写入，直接覆写 `.jsonl` 文件；MVP 需改为临时文件写入后 rename，避免进程崩溃导致 session 数据丢失。见 `agent-diva-core/src/session/manager.rs:124`）。
- 与 AutoDream 的数据流契约（见 §13）。
- **GUI 验证状态 (v1.2 P0-2 状态更新)**: 现有验证证据:
  - **源码存在**: `agent-diva-gui/src/components/NotebookView.vue` 17607 bytes / ~725 行, commit `fcf768d` ✅
  - **测试覆盖**: ⚠️ **待补充** —— `agent-diva-gui/tests/notebook_view.spec.ts` 等测试文件**尚未创建**, 需在 MVP 开发前补齐
  - **覆盖率**: ⚠️ **待补充** —— CI 覆盖率报告未生成, MVP 验收前需达到 > 80%
  - **设计评审**: ⚠️ **待补充** —— 需走 ux-review 流程 (`docs/dev/ux-review-2026-06-XX.md`)
  - **P0-2 状态**: v2 时仅"源码 + commit hash"已验证, 测试/覆盖率/评审待补. MVP 阻塞: 测试文件必须存在, 覆盖率需 > 80%
- **N2 并发控制 (v1.2 增补)**: 报表生成的并发控制策略:
  - **同一报表类型同时刻仅 1 个生成任务** (auto + manual 互斥, 用 lock 文件 `{workspace}/reports/.lock` 实现)
  - **不同报表类型可并行** (e.g. 日报 + 周报不冲突)
  - **跨 PRD 互斥**: 同一日期的日报, AutoDream 自动生成 与 本 PRD 手动触发 互斥 (走 AutoDream lock 路径)
- **N3 时区 (v1.2 增补)**: 见 FR-1 Consequences N3, 统一使用系统本地时区.

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
3. ~~SOP/Skill 格式~~ → **已确认**: 参考 GenericAgent（`.workspace/` 目录）→ **v1.2 升级**: SOP 模板见 §4.2 FR-6, Skill 格式见 §4.2 FR-7.
4. ~~Memory Provider~~ → **已确认**: 正在合并 memory 分支，完成后统一配置。
5. ~~搜索权限~~ → **已确认**: 暂时不考虑权限控制。
6. ⚠️ **LLM 选型** → 见 §10.4 候选 + 4 方案, 大湿需 06-16 前拍板 (L1 推荐).
7. ⚠️ **搜索演进路线** → 见 §10.5 v1/v2/v3 切换条件, 大湿需 06-16 前确认 v1→v2 触发阈值.
8. ⚠️ **GUI 测试覆盖** → P0-2 阻塞, 需补 `agent-diva-gui/tests/notebook_view.spec.ts` + 覆盖率 > 80%.

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

### 10.4 LLM 选型 & 成本 Guardrail (v1.2 占位 — 决策待用户拍板)

> **状态**: ⚠️ **PENDING** — 此节为占位, 大湿需在 06-16 前确认 LLM 选型 + 成本预算. 详见 Sprint Change Proposal 2026-06-12 Action Items.

**待决策项**:

#### 1. LLM 模型选型

下表为本 PRD 候选模型对比 (基于 `agent-diva-providers` 现状 + 团队常用模型):

| 候选 | 优势 | 劣势 | 单次日报成本估算 (输入 10K + 输出 2K) | 适用场景 |
|------|------|------|--------------------------------------|----------|
| **Claude Sonnet 4** | 质量高、长文连贯好、团队熟悉 | 成本中、API 限流严 | $0.045 / 报 | 默认推荐 (质量优先) |
| **GPT-4o mini** | 速度快、成本低 | 中文质量一般 | $0.005 / 报 | 大批量日报 (成本优先) |
| **Gemini 2.5 Flash** | 长 context、价格低 | 中文报告风格需调优 | $0.008 / 报 | 备选 |
| **DeepSeek V3** | 中文好、价格极低 | API 稳定性待验证 | $0.003 / 报 | 中文场景备选 |
| **本地 Ollama (qwen2.5-7b)** | 零 API 成本、隐私好 | 硬件要求高、质量参差 | $0 (电费) | 高频手动生成 |

**默认推荐**: Claude Sonnet 4 (主) + DeepSeek V3 (降级回退, 成本低 15x).

#### 2. 成本 Guardrail

待大湿确认的预算:
- **日预算**: $0.5 / 天 (约 11 份 Sonnet 4 日报 + DeepSeek 降级)
- **周预算**: $3.5 / 周
- **月预算**: $15 / 月 (含月报 LLM 调用, 月报较贵)
- **手动生成软上限**: 5 次/日 (见 FR-4 AC-4.5)
- **超预算行为**: 降级到 DeepSeek V3 (成本 -93%) + 记录到 events.jsonl

#### 3. 决策点

大湿请在以下方案中选 1:
- **方案 L1**: Sonnet 4 主 + DeepSeek 降级 + 默认预算 $0.5/天 (推荐)
- **方案 L2**: GPT-4o mini 全程 (成本优先, 中文质量妥协)
- **方案 L3**: 本地 Ollama (零 API 成本, 需硬件)
- **方案 L4**: 用户自定 (提供具体配置)

### 10.5 搜索方案演进路线 (v1.2 占位)

> **状态**: ⚠️ **PENDING** — 大湿需确认短期/中期/长期方案切换条件.

| 阶段 | 时机 | 方案 | 性能 (1000 sessions) | 实施成本 |
|------|------|------|----------------------|---------|
| **v1 (短期)** | 当前 MVP | 内存遍历 + 正则匹配 | 30-60s (用户感知慢) | 0 (已就绪) |
| **v2 (中期)** | session 数 > 500 或 P95 > 10s | **SQLite FTS5** 全文索引 | 100-500ms (快 100x) | Medium (需 schema 迁移) |
| **v3 (长期)** | 需要语义搜索 (Non-Goal 提到) | Embedding + 向量检索 (sqlite-vec) | 200ms + 语义匹配 | High (需 embedding 模型) |

**v1 → v2 切换触发条件 (任一)**:
- 总 session 数 > 500
- P95 搜索响应时间 > 10s
- 用户反馈"搜索太慢" (>= 3 次/月)

**v2 → v3 切换触发条件**:
- 用户要求"模糊搜索"或"语义搜索" (基于用户访谈)
- 团队决定把"语义搜索"移出 Non-Goals

**v2 阶段 schema 迁移** (草案):
- 新增表 `session_messages_fts` (FTS5 虚拟表)
- 同步触发器: 写入 `.jsonl` 时同步更新 FTS
- 查询接口: `SELECT ... FROM session_messages_fts WHERE content MATCH ?`
- 迁移脚本: `agent-diva-report-system migrate-search --to v2` (P1 实现)

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
- **v1.2 详细成本控制**: 见 §10.4 LLM 选型 & 成本 Guardrail. 核心 guardrail:
  - 日预算 $0.5 (默认)
  - 超预算降级到低成本模型
  - 手动生成软上限 5 次/日
  - 所有 LLM 调用 token 计入 `events.jsonl` 供审计

---

## 12. References

- `docs/research/bmad-info-set-report-session.md` — BMad 信息集
- `docs/research/report-session-research-info-set.md` — 技术调研报告
- `agent-diva-gui/src/components/NotebookView.vue` — GUI 实现 (commit `fcf768d`)
- `.workspace/genericagent/` — GenericAgent 参考项目 (SOP/Skill 格式历史来源, **v1.2 升级**: 具体格式见 `sop-template.md` 与 cherry-studio skill 范例)
- `docs/prd-report-system/sop-template.md` — **v1.2 新增**: SOP 模板 (FR-6 具体格式)
- `.workspace/cherry-studio/.agents/skills/create-skill/SKILL.md` — **v1.2 新增**: Skill 格式范例 (FR-7)
- `docs/architecture/scope-merge-decision.md` — **v1.1 新增**: 边界分工决策
- `docs/architecture/autodream-architecture-2026-06-12.md` — **v1.1 新增**: AutoDream 架构 + ADR-008 写入路径契约
- `_bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md` — Sprint Change Proposal

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

- **2026-06-12 v1.2**: P1 修复批。AC 约定、Schema 模板、依赖关系、N1/N2/N3、LLM 选型占位、搜索演进路线、GUI 验证状态诚实化。
- **2026-06-12 v1.1**: 边界分工修订（方案 C 批准）。新增 §1.1 范围边界表、FR-1/2/3 owner 标注、§6.1 In Scope 更新、§13 Cross-PRD Interface。
- **2026-06-08 v1.0**: 初版。10 FRs + 2 轮 validation (Fair+)。
