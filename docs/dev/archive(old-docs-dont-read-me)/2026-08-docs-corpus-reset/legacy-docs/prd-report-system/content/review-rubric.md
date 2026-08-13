# PRD Quality Review — Agent-Diva Pro 报表系统 & Session 历史检索

## Overall verdict
该 PRD 是一份结构清晰、意图明确的内部工具级需求文档，基本覆盖了报表生成与 Session 检索两大功能域。然而，文档在 Decision-readiness、Done-ness clarity 和 Scope honesty 三个维度存在明显短板：关键决策（如 LLM 选型、生成策略、搜索索引结构）缺乏权衡分析，FR 的可测试性不足，且 Assumptions 与 Open Items 之间存在严重的信息不一致。对于内部工具级 stakes 而言，当前状态处于 "可开工但风险可控" 的边界，建议补充关键决策的 trade-off 分析和 FR 的 acceptance criteria 后再进入开发。

---

## Decision-readiness — thin
文档给出了功能清单，但决策者（Tech Lead / 架构师）无法据此做出关键技术决策，因为核心权衡未被诚实呈现。

### Findings
- **high** 缺少 LLM 选型与成本权衡分析 (§7 Success Metrics, §11.3 成本) — PRD 承认报表生成依赖 LLM 调用且需控制频率，但未说明选用哪个模型、token 预算、成本上限，也未提供 "日报生成成本 > X 时降级" 的决策路径。*Fix:* 增加 LLM 选型决策矩阵（模型 / 成本 / 质量 / 延迟），并定义成本 guardrail。
- **high** "Agent 智能搜索" 的实现路径未做技术方案比选 (§4.3 FR-9) — 文档仅说明 "内存遍历 + 正则匹配（短期方案）"，但未解释为何不用 SQLite FTS、向量检索或简单文件 grep，也未定义 "短期" 的退出条件。*Fix:* 补充搜索方案演进路线图（短期正则 → 中期 SQLite FTS → 长期语义搜索），并标注各阶段的取舍。
- **medium** 报表生成质量的验收标准模糊 (§7 SM-1) — "成功率 > 95%" 未定义 "成功" 的判定标准（是 LLM 调用不报错？还是用户认为内容可用？）。*Fix:* 明确 "成功" 的 operational definition（如：生成过程无异常退出，且输出包含全部 4 个必填 section）。
- **medium** Decision log 与 PRD 正文存在信息冲突 (Decision log vs §8 Open Questions) — Decision log 中 5 项决策均标记为 "已确认"，但 Open Items 表格中对应的 5 项仍显示 "Open"。*Fix:* 统一状态，将 Open Items 中已确认项关闭，或明确标注 "Decision log #N 已确认"。

---

## Substance over theater — adequate
文档整体为实质内容，不存在明显的 persona theater 或 vision theater，但存在两处 "NFR theater" 和一处 "innovation theater"。

### Findings
- **medium** NFR 章节（§10）流于形式，缺乏与 FR 的映射和量化指标 (§10 Cross-Cutting NFRs) — "性能""可靠性""安全性" 三个小节仅列出原则性描述，未说明如何验证（如：报表生成异步完成，如何监控？5s 搜索延迟的测试方法？）。*Fix:* 为每条 NFR 增加验证方法（测试用例或监控指标），并与具体 FR 建立可追溯链接。
- **medium** "Agent 智能搜索" 的命名带有 innovation theater 色彩 (§4.3, Glossary "Agent 智能搜索") — 实际实现仅为 "内存遍历 + 正则匹配"，与 "智能" 一词差距较大，可能误导下游团队对技术复杂度的预期。*Fix:* 将术语更名为 "Session 关键词搜索" 或 "历史对话检索"，在 v2 引入语义搜索后再恢复 "智能搜索"。
- **low** Vision 段落（§1）存在轻微的 vision theater — "进化为具备自我回顾、知识沉淀、历史检索能力的智能体" 表述过于宏大，与内部工具的实际 stakes 不匹配。*Fix:* 缩减 vision  rhetoric，聚焦 "减少用户重复回顾 session 的时间成本"。

---

## Strategic coherence — adequate
文档存在统一的产品弧线（session 历史 → 自动报表 → 知识固化），但功能之间的依赖关系和优先级未清晰表达。

### Findings
- **high** 报表固化（SOP/Skill/Memory）与报表生成之间的依赖关系未明确 (§4.2) — FR-6/7/8 假设报表已生成且内容可用，但未说明如果报表生成失败或内容为空，固化流程如何处理。*Fix:* 在 FR-6/7/8 的 Consequences 中增加前置条件："仅当关联报表已成功生成且内容非空时，固化功能可用"。
- **medium** Session 历史检索与报表生成功能的协同关系未说明 (§4.1 vs §4.3) — 两个功能共享 "session 历史" 数据源，但 PRD 未说明检索结果是否可以被报表生成引用，或报表中是否包含检索统计。*Fix:* 在 §4.1 或 §4.3 中增加跨功能交互说明，或明确标注 "两个功能独立，无直接交互"。
- **low** MVP Scope 中 "Session 原子写入修复" 的纳入理由不足 (§6.1 In Scope) — 该项为技术债务修复，与报表/检索功能无直接业务关联，但被列为 MVP 必选。*Fix:* 增加一行说明："Session 原子写入修复是报表生成和搜索的数据完整性前置依赖"。

---

## Done-ness clarity — thin
工程师无法仅凭此 PRD 判断 "完成" 的精确边界，大量 FR 缺少可测试的 acceptance criteria。

### Findings
- **critical** 所有 FR 均缺少明确的 acceptance criteria (§4.1–§4.3) — 现有的 "Consequences" 仅为行为描述，而非测试用例。例如 FR-1 的 "日报包含：对话摘要、关键决策、完成的任务、待跟进事项" 无法测试（何为 "关键决策"？由谁判定？）。*Fix:* 为每个 FR 增加 Given-When-Then 或验收清单格式的 acceptance criteria。
- **high** 报表生成的输出格式未定义 schema (§4.1 FR-1/2/3) — 仅说明 "Markdown 文件"，但未提供模板、header 结构、必填字段。工程师无法据此实现。*Fix:* 提供日报/周报/月报的 Markdown 模板示例（含 frontmatter schema）。
- **high** "固化为 Skill" 缺少格式规范引用 (§4.2 FR-7) — 文档说 "参考 GenericAgent skill 规范（见 `.workspace/` 目录）"，但未提供具体文件路径或格式示例。下游无法提取。*Fix:* 在 References 中补充具体的文件路径（如 `.workspace/genericagent/skills/example.skill.md`），或内联一个最小示例。
- **medium** "固化为 SOP" 的 "参考 GenericAgent SOP 格式" 同样缺少具体引用 (§4.2 FR-6) — 同 FR-7。*Fix:* 补充具体文件路径或内联示例。
- **medium** 搜索结果的 "结构化 JSON" 格式未定义 schema (§4.3 FR-10) — "文本列表或结构化 JSON" 过于模糊。*Fix:* 提供 JSON schema 示例，包含字段：session_key, timestamp, message_content, match_type, relevance_score（如有）。
- **low** GUI 的 "双栏布局" 未定义交互细节 (§4.1 FR-5) — "报表列表和详情双栏布局" 缺少响应式行为、空状态、加载状态的描述。*Fix:* 补充 GUI 交互状态图或引用已有的 GUI 设计文档。

---

## Scope honesty — adequate
Non-Goals 和 Assumptions 基本完整，但存在隐式遗漏和状态不一致。

### Findings
- **high** Assumptions 与 Open Items / Decision log 之间存在状态不一致 (§9 Assumptions Index, §8 Open Questions, Decision log) — Assumptions 中 ASSUMPTION-1 提到 "依赖 session 持久化修复"，但 Open Questions 中所有项已被 Decision log 标记为 "已确认"，而 Open Items 仍显示 "Open"。这种三角不一致会导致下游对依赖状态的困惑。*Fix:* 统一三个来源的状态，建立单一真相源。
- **medium** 未显式声明的隐式假设：用户 workspace 目录可写 (§4.1 FR-1/2/3, §4.2 FR-6/7) — 所有报表和 SOP/Skill 的存储路径均假设 `{workspace}` 可写，但未在 Assumptions 中列出。*Fix:* 增加 ASSUMPTION-5: "用户 workspace 目录具有写权限，且磁盘空间充足"。
- **medium** "Non-Users (v1)" 的表述方式暗示 v2 会覆盖这些用户，但无承诺 (§2.2) — "不需要报表功能的用户（可通过配置关闭）" 未说明 "配置关闭" 是否在当前 MVP 中实现。*Fix:* 明确 "配置关闭" 是 MVP In Scope 还是 Out of Scope。
- **low** 缺少对 "报表生成失败重试策略" 的 scope 声明 — FR-1 提到 "生成失败时记录错误日志"，但未说明是否重试、重试几次、退避策略。*Fix:* 在 §6 MVP Scope 或 FR-1 Consequences 中补充重试策略声明（如：不重试，仅记录日志并告警）。

---

## Downstream usability — adequate
Glossary 基本一致，ID 连续，但存在术语漂移和交叉引用不足的问题。

### Findings
- **medium** 术语漂移："Report" vs "报表" vs "报告" 混用 (全文) — Glossary 定义了 "Report / 报表"，但正文中 "报告" 也出现多次（如 §1 Vision "自动生成日报、周报、月报"，§4.1 "报表自动生成"）。"报告" 未在 Glossary 中定义。*Fix:* 统一使用 Glossary 定义的 "Report / 报表"，删除或替换所有 "报告" 用法。
- **medium** FR ID 不连续：FR-1 到 FR-10 连续，但缺少对 UJ 和 FR 的交叉引用矩阵 — 下游 UX/架构团队难以快速定位某个 UJ 由哪些 FR 实现。*Fix:* 在 §2.3 Key User Journeys 末尾或 §4 Features 开头增加 UJ-FR 映射表。
- **low** References 中的文件路径未验证存在性 (§12 References) — 如 `agent-diva-gui/src/components/NotebookView.vue` 和 `.workspace/genericagent/` 的路径在当前评审环境中无法验证。*Fix:* 在提交 PRD 前确认所有引用路径有效，或增加备注 "路径以实际仓库结构为准"。
- **low** Glossary 中 "Session 任务" 的定义与 FR-9 中的 "搜索任务作为 session 任务异步执行" 存在轻微语义重叠 — "Session 任务" 被定义为 "Diva 在后台执行的异步任务"，而 FR-9 的 "搜索任务" 也是后台异步任务，但未明确是否属于 "Session 任务" 的一种。*Fix:* 在 FR-9 中明确 "搜索任务是一种 Session 任务"，或调整术语避免歧义。

---

## Shape fit — adequate
PRD 的形状基本匹配内部工具级产品：结构简洁、无过度包装、MVP 范围合理。但存在一处形状错配。

### Findings
- **medium** "固化为 SOP/Skill/Memory" 的功能跨度超出内部工具的典型需求 (§4.2) — 对于内部工具 stakes，"固化" 功能涉及知识管理体系设计（SOP 格式、Skill 规范、Memory Provider），其复杂度接近一个独立子系统。PRD 将其与报表生成并列为一组 FR，但未提供足够的架构上下文。*Fix:* 考虑将 §4.2 拆分为独立的 "知识沉淀子系统 PRD"，或在当前 PRD 中增加架构概览图，说明 SOP/Skill/Memory 与现有 GenericAgent/Hermes 生态的集成方式。
- **low** Success Metrics 中的 "用户每周至少查看 1 次报表" (§7 SM-2) 对于内部工具而言过于产品化 — 内部工具通常不需要强用户 engagement 指标，更应关注 "开发者节省的回顾时间"。*Fix:* 将 SM-2 调整为内部工具导向的指标，如 "开发者平均每周节省 X 分钟用于手动回顾 session"。

---

## Mechanical notes

### 术语漂移
- **Report / 报表 / 报告**: Glossary 定义 "Report / 报表"，但正文中 "报告" 出现多次（§1 Vision "自动生成日报、周报、月报"、§4.1 "报表自动生成" 等）。建议统一。
- **Session 任务**: Glossary 定义为 "Diva 在后台执行的异步任务"，FR-9 中 "搜索任务作为 session 任务异步执行" — 需明确 "搜索任务" 是 "Session 任务" 的实例化。

### ID 连续性
- FR-1 到 FR-10 连续，无跳号。
- UJ-1、UJ-2 连续。
- SM-1 到 SM-C1 连续。
- ASSUMPTION-1 到 ASSUMPTION-4 连续。

### 交叉引用
- FR 与 UJ 的引用通过 "Realizes UJ-N" 实现，但缺少反向索引（UJ 到 FR 的映射）。
- §12 References 中的路径未经验证，存在引用失效风险。
- Decision log 与 §8 Open Questions、§9 Assumptions Index 之间存在状态不一致（见 Decision-readiness 和 Scope honesty  findings）。

### 格式问题
- 文档使用 `---` 作为 section 分隔符，与标准 Markdown 的 horizontal rule 语法冲突，在某些渲染器中可能导致显示异常。建议改用 `##` 级别的 section header 进行分隔。
