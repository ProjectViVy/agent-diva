# PRD Quality Review — Self-Improve (prd-selfinprove-2026-06-12 v0.0.4)

## Overall verdict

PRD v0.0.4 在主战场 (FR-1xx Inbox 批处理 9 条全本 + §1 Vision capability-first + §2 Target User 具体刻画 + decision log D-001~D-011 全档含原话捕获) 达到可审稿 substance, decision-readiness 与 strategic thesis (D-006 全 user 审作为核心命题) 清晰。但 §5 FR-2xx~6xx 全部 outline 状态, §6 Open Questions 空白, §3 Glossary 缺失, §0/Addendum/§6/§7/§8 多个 stub section, 加上 3 份 reconcile 文件累计 32 处 gap 集中指向"假设下游会接 + 假设上游已就绪"型盲点, 落稿前必须先把 Laputa (D-011 stub 实质化 / D-010 stale 关闭) / AutoDream (trigger 同步性 / changelog 共享 / evidence 锚点 / lock 行为) / Report System (UI 4 表面基线 / 14 路由契约 / 9 路径 storage mapping / NotebookView P0-2 阻塞前置) 三方 back-link 闭环, 否则下游 UX / architecture / story creation 三条流水线都会返工。

## Decision-readiness — adequate

决策档案做得扎实。`.decision-log.md` (L8-209) 落档 D-001~D-011 共 11 条决策, 包含 6 处用户原话捕获 per P3 (D-001 L19-21 / D-002 L29-36 / D-006 L84 / D-009 L124 / D-010 L141-144 / D-010.r1 L172-174 / D-011 L193), Trade-off 显式给出 (e.g. D-005 L74-76 写"引入实时外部控制面, 需另立安全评审...与主题正交" / D-006 L86-90 写"硬约束进入 PRD §6.3" + 隐含影响), `[D-006 硬约束]` `[D-009]` `[D-010.r1]` `[LAPUTA-STUB]` 这类 inline 标记在 §5 FR 处反复出现, 可溯源。FR ID 编号稳定全局连续 (1xx/2xx/3xx/4xx/5xx/6xx)。

但有几处 smooth 过 / silent deferral:
- D-010 在 L138-166 标记"P0 待决"+"依赖关系 P0 待决", 但 D-010.r1 (L169-187) 已实质关闭 ("稀薄文档层"), reconcile-laputa G8 标 stale, 而 PRD §1 Vision L38-39 "Laputa 现状 (D-010.r1)" 已锁定 — 但 §3 Concerns 表 (L55-68) 没回写, 仍按 stale P0 风险登记。
- D-006 L92-93 明写"待追问: low-risk auto 一刀切切掉...是否要在 Settings 里加'高级模式'开关...留 §6.4 Open Question", 但 §6 (PRD L177-179) 整段空。
- D-011 L206-209 三条未决事项 "→ Laputa PRD 决定", 没有 [NOTE FOR PM] callout, 是 silent deferral, 没有标"如果 Laputa PRD 6/16 拍板, selfinprove 需哪些 changes"。

### Findings

- **[high]** D-010 实质已关闭 (PRD §1 L38-39 锁 D-010.r1), 但 §3 Concerns (L55-68) 仍按 P0 风险登记, 没回写 "Laputa = 稀薄文档层 + D-011 一锅端" 后 concerns 变化 (§3 L57 "依据...P0 决策 (D-002 + D-005~D-008)" 缺 D-009~D-011)。*Fix:* Concerns 表加 C-9 (stub 实质化为接 Laputa FR-1xx~7xx) + C-10 (Report System §6.1 P0-2 阻塞前置) + 更新 L57 决策清单。
- **[high]** D-006 L92-93 显式留 §6.4 Open Question 但 §6 (L177-179) 整段空。*Fix:* §6 至少加 OQ-D006-1 "low-risk auto 高级模式开关是否进 Settings, 进 v1 还是 v2"。
- **[medium]** D-011 L206-209 "未决 → Laputa PRD 决定" 三条, 没标 [NOTE FOR PM] + 也没给 selfinprove 落稿前置条件 (e.g. "如果 Laputa PRD 6/16 前未拍板写权限契约, selfinprove 落稿延后")。*Fix:* D-011 末加 [NOTE FOR PM] 三行, 标前置依赖。
- **[low]** Decision log 缺 D-012 (reconcile-autodream actionable #8 推荐: 写权限归属澄清, FR-103 接管写权限跟 AutoDream FR-11 / Non-Goals §5 消解)。

## Substance over theater — strong

§1 Vision (PRD L26-39) 含 capability-first 落地向 + 节律双模 (D-009) + Laputa 现状 (D-010.r1) + Ship 策略 (D-011), back-link 到 `docs/vision/04-进化路线图.md` L886-889, 不是模板化的"empower users"空话。§2 Target User (L42-53) 6 字段具体刻画 (角色/场景/期望/技术能力/痛点/价值主张), 每字段都有可验证内容 (e.g. 痛点 "每天 agent 跑了什么 / 记住了什么 / 改了 Laputa 什么, 全是黑盒" 直击 D-006 scope 放大器根源), 没有 "Power user / Casual user" 这种 persona theater。NFR 段 (FR-6xx L164-171) 6 条 outline 但每条带 bound (FR-601 ≤500ms / FR-605 30 天保留期), 不是 "must be scalable / secure" boilerplate。

FR-1xx 9 条全本 (L97-133) 是真正 earned content: 每条 MUST 含字段约束 (e.g. FR-101 "evidence_excerpt ≤120 字符, risk_level badge") + 验收 ("启动后 Inbox 看到 ≥1 条 mock proposal, 过滤条件可改可重排")。FR-103 状态机 5 状态 (approved/rejected/edited/applied/reverted) 与 FR-106 回滚 + FR-105 已读交叉引用形成完整闭环。

### Findings

- **[low]** §2 Target User L48-50 提到"开发者/技术尝鲜者, 已安装 agent-diva-pro 开源版, 跑至少 1 周" — "至少 1 周" 是 critical assumption, 没标 [ASSUMPTION]。*Fix:* 加 [ASSUMPTION: 用户已跑 ≥1 周才有足够 Laputa 数据] 标 + Addendum/§3 索引。
- **[low]** §1 Vision L32 "30 天后...有了只属于这位用户的身份定义和有机用户模型" — "30 天" 是 thesis 周期, 但 §7 Success Metrics 缺 (reconcile-laputa L15 指出), 没法验证 30 天是否达成。*Fix:* §7 加 SM-D30 "30 天后 ≥70% 用户在 Laputa 看到 ≥1 条 identity 类 identity 变更被 review 过"。

## Strategic coherence — adequate

Thesis 清晰: §1 Vision L30-32 "让用户**看见并参与**自己 agent 的人格如何随时间生长" + "用户每天 ≤5 分钟, 干完这件事" — 这是 engagement quality thesis, 不是 efficiency thesis。Feature 跟随 thesis: FR-1xx (Inbox 批处理, D-006 scope 放大器) + FR-2xx (固化流程) + FR-3xx (节律 + manual) + FR-4xx (4 表面导航 + i18n) 全部服务"看见 + 参与"主弧, 不是 capability backlog。

MVP scope kind: capability-first experience MVP, scope logic 匹配 thesis (D-006 "全 user 审" 隐含 engagement over efficiency, D-009 "默认 out-of-session + manual fallback" 隐含 trust over speed)。

但缺:
- §7 Success Metrics 缺失 (PRD §1~§6 后直接到 §6 Open Questions, §7/§8 不存在)
- 没有 counter-metrics (e.g. "用户每天 ≤5 分钟" → counter: "不是 ≤1 分钟" 防止误读)
- 30 天价值主张 (Vision L32) 没有 measurement plan

### Findings

- **[critical]** §7 Success Metrics 完全缺失 (PRD §6 后直接 Revision Notes, L177-186 间隔 9 行空)。*Fix:* §7 加 SM-D30 (30 天 identity review 完成率) + SM-Daily (≤5 分钟用户每天实际用时 P50) + CM-NotAuto (auto-mode 默认 off, cron 路径不被偷偷开启) 三个 metric。
- **[medium]** Counter-metrics 缺失 (用户每天 ≤5 分钟 → CM-NotMicro: 不优化到 ≤1 分钟以免引入 auto-apply)。*Fix:* §7 加 2 条 counter。
- **[low]** Vision L32 "30 天" 是 magic number, 没解释为什么 30 天 (per D-010.r1 Laputa 稀薄文档层, 是否对应 .relationships.md 周度更新 + .identity.md 月度固化?)。

## Done-ness clarity — adequate

FR-1xx 9 条全本验收具体:
- FR-101 (PRD L100-101) "启动后 Inbox 看到 ≥1 条 mock proposal, 过滤条件可改可重排" — 可测, 明确 mock 阶段
- FR-103 (L107-109) "4 个动作在 stub 模式下都能跑通, 状态正确流转, 应用时 changelog +1" — 状态机 5 状态明确, changelog +1 可测
- FR-106 (L119-121) "1 条已应用 proposal 回滚后, target_file 内容回到原状, changelog +1 revert 条目" — 回滚验证可测
- FR-109 (L131-133) "按 J/A/R/E 各能触发对应动作, 搜索框 focus 时快捷键不响应" — 5 快捷键 + 1 禁用条件全列

FR-2xx~6xx 全是 outline (L135-171), v0.0.4 显式标 [outline], 不是 silent gap。但 reconcile 三份累计 32 处 actionable 直接对应这些 outline 的扩写需求, "outline" 状态跟 "launch 级" stakes (D-001 L15) 不匹配。

NFR 段有 bound: FR-601 ≤500ms for 100 proposals / FR-605 30 天保留期。但缺:
- changelog 路径契约 (reconcile-autodream G5): "走 changelog" 反复出现但路径 `.agent-diva/memory/changelog.jsonl` + 与 AutoDream 共享策略没声明
- Laputa 写权限分配 (G6): FR-501 "POST /api/laputa/proposals/{id}/apply" 走统一 API, 没声明与 AutoDream 写 `.laputa/inbox/learning-candidates.jsonl` 分工
- evidence 锚点字段 (G7): FR-107 "经 evidence_excerpt 锚定的 session_id" 用摘要做锚点, 无法精确回滚
- lock 行为契约 (reconcile-autodream G11): FR-302 manual 撞到 lock 时排队/报错/清 stale 没写

### Findings

- **[critical]** FR-2xx (固化流程, L135-141) outline 5 条, 跟 D-006 "scope 放大器" 标定主战场不匹配 — reconcile-laputa G1 + reconcile-report-system G9 共 2 条 actionable 指 FR-201/202 缺 SOP 4 节模板 + SKILL YAML frontmatter 引用 + 现有 skill 范例路径。*Fix:* FR-201/202/203/204/205 至少扩到 FR-1xx 同级 MUST+验收, FR-201 加 SOP 4 节格式契约 (引用 Report System §4.2 FR-6), FR-202 加 YAML frontmatter `name`+`description` (引用 Report System §4.2 FR-7)。
- **[critical]** FR-3xx (节律/触发/诊断, L143-148) outline, reconcile-autodream G3+G4+G10+G11 共 4 条 actionable 指 FR-303 字段从 5 扩到 8 (加 trigger_type 5 枚举 / auto_mode_enabled 默认 false / degraded 布尔+reason)。*Fix:* FR-303 字段扩到 8 个, FR-301 加 "auto_mode 默认关闭, 关闭时仅响应 manual" 1 行。
- **[critical]** FR-4xx~5xx outline (L150-162), reconcile-report-system G1+G2+G3+G4+G5+G6+G7+G8+G11+G12 共 10 条 actionable 集中指向 — 4 表面导航 / UiCard 11 字段 / 5 核心流程端到端 / 14 路由 + 10 SSE / 9 路径 storage mapping / 写权限降级 / lock 行为契约 / non-goal back-link 全缺。*Fix:* 按 reconcile-report-system actionable #1~8 逐条扩写, FR-406 [outline] "Backend/UI Contract 基线对齐" 引用 UI research §10 14 路由 + 10 SSE。
- **[high]** FR-501 `[LAPUTA-STUB]` (L160) 缺降级契约 — reconcile-report-system G8 指 FR-8 pending.jsonl 降级契约没引用。*Fix:* FR-501 改写为 "调统一 Laputa API, 不可用时降级到 `{workspace}/memory/pending.jsonl`"。
- **[high]** FR-107 (L124) evidence 锚点用 evidence_excerpt 做锚定, 不精确 (reconcile-autodream G7)。*Fix:* FR-107 改写为 "经 evidence_refs[].session_id + turn_index 锚定原始 session turn, evidence_excerpt 仅作预览"; 新增 FR-110 "evidence 回溯面板"。
- **[high]** FR-106 / FR-204 / FR-602 "走 changelog" (L120 / L140 / L168) 缺路径契约 + 共享策略 (reconcile-autodream G5)。*Fix:* 三处加 ".agent-diva/memory/changelog.jsonl, 与 AutoDream design §10.4 共享, 保留期 ≥30 天"。
- **[medium]** FR-603 i18n 完整性 (L168) 缺具体 namespace 不破坏已有 selfEvolution.* / notebook.* 的回归测试范围。*Fix:* 加 AC "回归 selfEvolution.* / notebook.* / core.* 共 N 个 key 字符串不变"。
- **[medium]** FR-302 manual trigger 撞到 lock UI 行为契约缺 (reconcile-autodream G11 + reconcile-report-system G11)。*Fix:* FR-302 加 "lock 活进程占 → 卡片显示 PID + 启动分钟 / stale >60min → 自动回收并提示 / 与 Report System N2 + AutoDream design §6.2 对齐"。

## Scope honesty — thin

§4 Scope Boundaries (PRD L70-91) 做得好, 16 行 × 3 列 (能力/IN-REF-OUT/备注) 显式列出: IN 9 行 (记忆候选/固化/Journal 批处理/节律/manual trigger/cron 异步/Laputa 写 UI 入口/diagnostics/changelog 顶级/全 user 审/MEMORY 三层/MemoryProvider 契约), REF 2 行 (AutoDream / Report System), OUT 4 行 (文件干预机制/知识图谱/技能生态/Kanban/Plan mode)。D-005 L74-76 给 OUT 理由 ("与主题正交"), D-002 L37-42 给 OUT/REF/IN 三档划分依据 (大湿原话捕获)。

但 Open Items 密度异常低 (launch 级 stakes 下):
- §6 Open Questions (L177-179) 整段空
- §0 Document Purpose (L20-22) "待 Discovery 完成后填写" stub
- Addendum (L173-175) "(空 — 材料汇总后由子 agent extract 落入)" stub
- §7/§8 完全缺失 (L172 直接到 §6, L176 后直接 Addendum)
- 没有 [ASSUMPTION] tag 索引 (reconcile 三份都提)
- 没有 [NON-GOAL for MVP] callout 在 §5 FR (reconcile-report-system G12 指 UI research §13 7 条 non-goal 没 back-link)
- [NOTE FOR PM] 仅出现在 PRD header L18 (工作模式声明), 不出现在 FR 处

### Findings

- **[critical]** §6 Open Questions 空白 (PRD L177-179 "(空 — 走 Decision log)") 跟 reconcile-autodream 12 gap + reconcile-report-system 12 gap 不匹配 — reconcile-autodream G1 (trigger 同步性) / G2 (cron phase 错位) / G6 (写权限分配) / G7 (evidence 锚点) / G11 (lock 行为) / G12 (FR-11 vs Non-Goals 矛盾) 共 6 条 high/medium 应进 §6。*Fix:* §6 populate 至少 11-15 条, 每条标 "Laputa 后已关 / 转 follow-up / 待 polish" 三档状态。
- **[high]** §0 Document Purpose stub (L20-22) — v0.0.4 还在 draft OK, 但落稿前必须填: 文档意图 / 读者 (UX/arch/stories 3 个下游) / 适用范围 (Self-Improve 用户可见层, Laputa/AutoDream/Report System 各自 PRD 边界声明)。*Fix:* §0 3 段, 引用 related_prds (PRD L9-11) 三方 PRD 边界。
- **[high]** Addendum stub (L173-175) — 子 agent extract 任务没完成。*Fix:* 落稿前补: (a) Laputa stub 实质化追踪 (D-011 三条未决); (b) AutoDream reconcile 8 actionable 进度; (c) Report System reconcile 8 actionable 进度。
- **[high]** 没有 [NON-GOAL for MVP] callout 在 §5 — reconcile-report-system G12 指 UI research §13 non-goal #2/#5/#6 (不让 GUI 直接改 MEMORY.md / 不在 chat 暴露完整 Mentle / 无 auto-apply identity) 没显式 back-link 到 selfinprove 边界。*Fix:* §5 加 3 句 back-link, FR-5xx 走 stub API 满足 #2 / FR-5xx 不含 Mentle 满足 #5 / D-006 全 user 审 满足 #6。
- **[medium]** 没有 [ASSUMPTION] tag — reconcile 三份至少 6 处显式假设 (e.g. "用户已跑 ≥1 周" / "auto_mode 默认 off" / "NotebookView P0-2 阻塞前置" / "event bus 已就位") 没标。*Fix:* §3 末尾加 Assumptions Index 表, inline 标 [ASSUMPTION: …]。
- **[medium]** [NOTE FOR PM] 仅出现在 PRD header, 不出现在 §5 FR 处 — D-011 三条未决 (PRD §5.5 L160-162 隐含) 应有 callout。*Fix:* §5.5 FR-501 末加 [NOTE FOR PM: Laputa PRD 6/16 前未拍板写权限契约, FR-501 stub 模式延后实接]。

## Downstream usability — thin

PRD 是 chain-top (feeds UX → architecture → stories per UI research §12 P0/P1/P2 mapping + Report System §13 Cross-PRD Interface)。下游三条流水线都需要 source-extract。

ID continuity OK: FR ID 1xx~6xx 全本连续, [D-006 硬约束] / [D-009] / [D-010.r1] / [LAPUTA-STUB] inline 标记反复出现, 可溯源。Cross-ref §1 Vision back-link 到 `docs/vision/04-进化路线图.md` L886-889 (PRD L28) + related_prds (L9-11) autodream/report-system 路径引用都准确。

但缺:
- §3 Glossary 完全缺失 (PRD L55-68 Concerns 表前没 Glossary section, 直接 §3 跳 Concerns; reconcile-report-system G2/G5 显式指 UiCard 11 字段 / Storage Mapping 9 路径 / MemoryChangelog / 4 表面 / Mentle 5 词条需进 Glossary; reconcile-autodream G8 也指 Mentle 词条缺)
- FR-2xx~6xx outline 状态直接限制下游 source-extract (UX 拿不到固化流程交互细节 / arch 拿不到 Laputa 契约细节 / stories 拿不到验收场景)
- 没有 P0/P1/P2 mapping 表 (reconcile-report-system 附录指 UI research §12 6 项 P0 + selfinprove FR-401/404/405 对应关系缺位)
- 每 FR 没标 "下游用这条做什么" (e.g. FR-109 快捷键给 UX 直接拿去做 Figma, FR-302 manual 给 arch 做 trigger 适配器)

### Findings

- **[critical]** §3 Glossary 完全缺失 (PRD L55 直接 §3 Concerns) — reconcile-report-system G2/G5 + reconcile-autodream G8 共 5 词条需进 Glossary: (a) UiCard 11 字段 (id/kind/status/title/summary/body_markdown/actions/links/badges/evidence_preview/created_at/updated_at) + kind 6 枚举; (b) Storage Mapping 9 路径契约 (`.agent-diva/{autodream,audit,plans,compact}` + `.laputa/{rhythm,inbox}` + `memory/{MEMORY.md,HISTORY.md,changelog.jsonl}`); (c) MemoryChangelog 路径 + 与 AutoDream/Report System 共享策略; (d) 4 表面 (Chat/Journal/Inbox/Settings) + "每表面回答 1 个用户问题" 分法; (e) Mentle 词条 + v1 不集成 back-link。*Fix:* §3 加 5 词条, 引用 UI research §3/§8/§11 + design spec §10.4 + Report System FR-8。
- **[high]** 缺 P0/P1/P2 mapping 表 (reconcile-report-system 附录指 UI research §12 6 项 P0 + selfinprove FR-401/404/405 对应关系缺位)。*Fix:* §5 末尾或 Addendum 加 1 张表, 6 行 (UI research P0 #1~#6) × 3 列 (UI research 描述 / selfinprove FR ID / 状态)。
- **[high]** 每 FR 没标下游用这条做什么 — UX 拿 FR-109 快捷键去做 Figma / arch 拿 FR-302 manual 做 trigger 适配器 / stories 拿 FR-103 状态机做验收场景, 这些映射关系没显式。*Fix:* 每 FR 模板加 1 行 "Downstream: UX(…) / Arch(…) / Stories(…)"。
- **[medium]** FR-2xx~6xx outline 限制 source-extract — UX 拿不到 FR-201 SOP 模板细节 / FR-401 UiCard 11 字段 / FR-404 sidebar 顺序。*Fix:* 跟 Done-ness critical findings 同源扩写。
- **[medium]** §3 Concerns 表 (C-1~C-8, L55-68) 没标优先级 + 来源强度 (除 "硬/中/低" 三档外没标下游影响)。*Fix:* Concerns 表加 "下游影响 (UX/Arch/Stories)" 列。

## Shape fit — adequate

PRD 是 brownfield + chain-top + launch 级 stakes (D-001 L15 "open-source 公开受众")。

形状匹配: capability spec shape + 局部已细化 (FR-1xx 9 条全本)。这是单 primary user 群 (尝鲜版 diva 用户, D-001 L17) + 4 表面 + 内部 tool 偏向 (chat/journal/inbox/settings), UJ 不是 load-bearing, Persona 不要 5 个 (D-001 "无所谓, 越多越好" 是反向信号, 但 §2 1 个 primary user 刻画做对了)。Brownfield: existing-code references (NotebookView.vue / SelfEvolutionSettings.vue / InboxView.vue / MemoryChangelogView / SidebarSection) 准确 (PRD §5.4 L152-156 + §5.6 L170 FR-604 引用 EvolutionRunDiagnostics.vue 新增)。

不匹配 / 缺:
- 没强行套 6 字段 UiCard 模型 — FR-401 (L152) 仅列 kind+status 枚举, reconcile G2 指 UI research §8 11 字段全列缺位, kind 枚举也只列 4 种缺 plan/option 2 种。
- Chain-top 缺 back-link 严重 (下游 UX/arch/stories 拿到 PRD 后还要去 3 份 reconcile + UI research + design spec 才能拼全, 这是 UJ/Ux hygiene 问题)。
- D-006 "scope 放大器" (D-006 L88-90 写 "Journal 表面从展示列表升级为批处理工作台") 是 capability spec → experience spec 的扩型信号, PRD 没显式承认 (Journal 升级后是"批处理工作台", 原 capability spec 不够装, 应进 experience shape)。

### Findings

- **[high]** FR-401 UiCard (L152) 仅列 kind+status 枚举, 缺顶层 11 字段 + kind 枚举缺 plan/option (reconcile-report-system G2)。*Fix:* FR-401 改写为列 11 字段 + kind 6 枚举 (per UI research §8)。
- **[high]** D-006 "Journal 升级为批处理工作台" (decision log L88-90) 是 shape shift signal (capability → experience), PRD §3 C-1 L62 (硬约束) 提了但 §0 没说 "本 PRD 是 capability + experience 混合 spec, experience 部分主要在 Journal 批处理工作台"。*Fix:* §0 / §1 加 1 句 shape 声明。
- **[medium]** §5.4 FR-4xx (L150-156) 把 4 表面导航散落 (Vision 提 Journal/Inbox/Chat, FR-1xx 全 Inbox, FR-3xx Settings+Chat), 没用 UI research "4 question model" (reconcile-report-system G1) 串起来。*Fix:* §5.4 加 FR-400 "Sidebar 顺序对齐 UI research P0 推荐 Chat → Journal → Inbox → Pet → Settings → Console → Neuro → Cron"。
- **[low]** D-001 L15-21 "无所谓, 越多越好" "你看着来" 是 user 在 stakes 上的 dismiss, 不是 stakes 真的低 — D-001 L15 已 capture 为 launch 级, 但 §0 没标 launch 级阅读对象 (公开开源软件受众, 阅读者可能是 contributor / 下游 PRD owner / 内部 reviewer 三类, 每类需要不同 section)。

## Mechanical notes

- **ID continuity**: FR 1xx~6xx 全本连续, OK。Inline 决策引用 [D-006 硬约束] / [D-009] / [D-010.r1] / [LAPUTA-STUB] 全可溯源, OK。
- **Cross-refs**: §1 Vision L28 back-link 到 `docs/vision/04-进化路线图.md` L886-889, OK; related_prds (L9-11) autodream/report-system 路径引用 OK; 但 D-004 (decision log L55-64) 提到的 `docs/dev/genericagent/*` 断链修复 commit 是否落档未追踪。
- **Glossary drift**: §3 缺失 (见 Downstream usability critical finding)。
- **Assumptions Index roundtrip**: 无 inline `[ASSUMPTION]` 标 + 无 index, 双向都缺。
- **UJ protagonist naming**: PRD 无 UJ section, capability spec shape 下不必要 (见 Shape fit)。
- **Required sections**: 缺 §0 实质内容 (stub) / §3 Glossary / §7 Success Metrics / §8 Open Items Index — 4 个 section 不存在或 stub。launch 级 stakes 下, 4 个 section 都应实质化。

---

## Compact Summary (≤200 字)

PRD v0.0.4 FR-1xx 9 条全本写得扎实 (§5.1 L97-133), decision log D-001~D-011 含原话捕获, Vision capability-first 不 theater — **3 维度 strong/adequate** (Decision-readiness / Strategic coherence / Shape fit adequate; Substance strong)。但 FR-2xx~6xx 全部 outline 状态 (§5.2~5.6 L135-171), §6 Open Questions 空白 (L177-179), §3 Glossary 缺失, §0/Addendum/§7/§8 多个 stub, 加上 3 份 reconcile 共 32 gap 集中指向"上游已就绪 / 下游会接"型盲点 — **3 维度 thin/broken** (Scope honesty / Downstream usability thin, 部分子项 broken)。

**Verdict**: 不能 ship, 落稿前必须 (a) 扩 FR-2xx~6xx 至 full FRs (3 份 reconcile actionable #1~8), (b) populate §6 OQ ≥11 条, (c) §3 加 5 词条 Glossary, (d) §7 SM + §0 实质内容, (e) 关 D-010 stale + 回写 Concerns C-9/C-10。文件: `C:\Users\Administrator\Desktop\morediva\agent-diva-pro\docs\prds\prd-selfinprove-2026-06-12\review-rubric.md`
