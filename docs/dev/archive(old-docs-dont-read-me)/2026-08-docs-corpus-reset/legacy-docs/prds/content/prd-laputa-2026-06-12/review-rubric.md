# PRD Quality Review — Laputa (prd-laputa-2026-06-12)

源材料: prd.md v0.0.4 (323 行) + .decision-log.md D-001~D-009 + 3 份 reconcile (laputa-new-architecture / mentle-laputa / auto-evolution)
评审对象: bmad-prd Finalize Step 3 Reviewer Gate
Stakes: launch 级 (per D-001, 与 selfinprove 同, 公开受众)
Shape: internal tool / brownfield / capability spec / chain-top (feeds UX → architecture → stories)

---

## Overall verdict

PRD 的"下半" (FR 层 §5.1-§5.7) 是 launch 级 PRDs 中少见的高强度 — 32 条 FRs 全带 验收 (verifiable 条件 / numeric 阈值 / grep 可查的命名约束 / 4 道治理串联的失败回滚), §6 OQ 把 7 项显式外推给 follow-up PRD 而非 silently deferred, 3 份 reconcile 把差距摊在桌面上而不是埋在 FR 后。但"上半" (framing 层 §0 / §1 / §2 / §3) 多处仍标 "待 Coaching X 步完成填写", §1 Vision 是 state report 不是 vision statement, §4 缺 Non-Goals 段, §4.3 仅有 3 行 out-of-scope 而输入 C §26 列了 9 条 P0 non-goals。后果: 工程师可基于 §5 开工, 但 PM 拿不到一个能在 launch review 上站住的 PRD — 风险是上半需要 polish 阶段补齐才能 ship。

---

## Decision-readiness — adequate

D-001~D-009 决策审计追踪扎实 — 每条带时间戳、原话引用、override 链 (e.g. D-005 勘误 D-002 的 Mentle 现状误判, D-009 修 D-008 发现的 2 真 phase-blocker)。Trade-off 在决策日志层写得清楚: D-005 §4 把 14 section 拆 Laputa-owned/Report-owned/TBD 三类并显式接受 "TBD 不在本 PRD 详写", D-006 "Q2 全 / Q3 全" + 单独发 Sprint Change Proposal 拆 stub, 是真实的取舍不是 smoothing。

但 §1-§3 没把决策映射到 PRD 正文:
- §1 Vision 锁 v0.0.2 (L33) 但内容是 "现状 + Ship 策略 + 跟现有系统边界" 三段 state report, **不是 thesis**。§1 L35-43 没回答 "Laputa 解决什么 user / product problem", 只回答 "Laputa 长什么样"
- §6 OQ #12-20 显式外推 7 项 + 2 项 P1 (L299-309), 这是 decision-readiness 的正向信号 — 但 #1-#11 的关 / 半关 / 开 分布 (L286-297) 显示 **3 项仍开 + 3 项半关** 是 critical density
- §6 OQ #4 (L290) "SOUL.md AI 自写用户怎么干预" 仍开, 而 FR-501 已枚举 `write_authority: "agent_self"|"user_only"|"shared"` 字段但**未给 #1/#3/#4 各归哪类** — 决策悬空在 PRD 正文中

### Findings
- **[critical]** §0 Document Purpose 是占位 (prd.md L27 "待 Coaching 第 0 步完成填写") — launch 级 PRD 顶段不许空。*Fix:* 把 §0 改写为 2-3 句定义 (本 PRD 定义什么 / 不定义什么 / 跟 sibling PRD 的关系), 与决策日志 D-001 L13-19 对齐
- **[critical]** §2 Target User 是占位 (prd.md L47 "待 Coaching 第二节填写") + 仅列 4 候选受众 (L51-53)。D-006 已锁 "Q2 受众 = 全 (实现者 + 架构师 + 文档读者)", 决策已知但 PRD 未落。*Fix:* 把 §2 改写为基于 D-006 的具体 persona 描述 + 每 persona 想要的 PRD 输出形式
- **[high]** §3 Concerns 列出 10 项 P0 候选 (prd.md L60-68) 但**没有 P0 答案** — 全标 P0 等于没标。*Fix:* 把 §6 OQ #1-#3 (已关) 的答案回填到 §3, 剩 7 项半关 / 开 标 "等 polish 阶段"
- **[high]** §1 Vision 是 state report 不是 thesis (prd.md L33-43)。D-007 已锁 1 稀薄层 14 section 但 §1 把它写成 "Laputa 现状 (D-002 + D-005 勘误)" + "Ship 策略 (D-006)" + "Laputa 跟现有系统的边界 (D-005)" 三段, 没有 1 句话统摄 thesis。*Fix:* §1 加 1 段 thesis: "Laputa 把 8 模板 + 3 写消费方 + 2 事件端 收敛到 1 稀薄层 + 4 接口 + 30 天可回滚, 让 agent 的长期身份变更必经 review/changelog"
- **[medium]** §6 OQ #4 (prd.md L290) "SOUL.md AI 自写, 用户怎么干预" 仍开 — 跟 D-005 §4 自治哲学强相关, 应升 P0。*Fix:* §6 OQ #4 升 P0 + 在 §5.5 FR-501 补 1 条子要求: "#1 identity = write_authority='shared' (用户可在 expectation.md 表达, agent 自写仍走 review)" 之类

---

## Substance over theater — strong (on §5) / broken (on framing 层)

§5 FR 层是反 theater 的标杆样本:
- FR-701 (prd.md L255-260) 性能阈值给 ms 数字 (50/100/200/20ms), 不是 "reasonable performance"
- FR-702 (L263-264) 审计验收用 `grep` 命中数, 不是 "system handles X gracefully"
- FR-704 (L271-272) 并发验收给具体数 (100 个并发写 + ≤200ms 总耗时)
- FR-705 (L275-276) 列 7 种错误枚举值, 不是 "should handle errors well"
- FR-103 (L118-122) 4 道治理串联 + 任一失败整体回滚, 验收明确: "模拟 1 道失败, 整体回滚"
- FR-501 (L196-203) schema 字段 5 个全给枚举值 + max_size_kb 给具体上限 (memory_md 30 行 ≤2kb)

但 framing 层 §0 / §2 / §3 是 theater 的另一种形态 — 占位符看起来像节标题, 实际没内容。

### Findings
- **[high]** §0/§2/§3 占位符是反 substance 的 boilerplate, 跟 §5 高密度 FR 形成强对比, 读 PRD 的人会误判 "上半不重要"。*Fix:* 同 Decision-readiness 3 条 fix
- **[medium]** §1 Vision "Laputa 现状 (D-002 + D-005 勘误)" (L35) 引用决策日志的密度过高, 自我指涉而非用户语言。*Fix:* §1 改写为 "Laputa 是 X (user-facing problem), 现有系统 Y (gap), 本 PRD ship Z (fix)" 三段
- **[low]** FR-2xx/3xx/4xx response schema (L140, L162, L179, L184, L189) 全用 markdown 表格 + 示例字段, 形式上齐但缺 JSON schema 锚定, code review 不易查。*Fix:* 后续 polish 可加 inline ` ```json schema 块 ` 锚定

---

## Strategic coherence — adequate

PRD 隐含 thesis (1 稀薄层 + 4 接口 + 30 天回滚 + 取代 8 模板), 32 条 FR 服务于此 thesis (无 thesis-irrelevant FR)。§6 OQ #12-20 (L299-309) 把 7 项显式外推到 follow-up PRD (三轴主体性 / 进阶心跳 / MemoryProvider 4 lifecycle / AutoDream 4 thin / Heartbeat 触发链 / Mentle 5 wing / Authority 4 级), 是 scope discipline 的强信号 — 没硬塞进本 PRD。

但 thesis 缺 3 块锚定:
- §5 4 个接口 (写/读/事件/changelog) 拆得太细, **没有 1 条 FR 把端到端闭环串成 named loop** — reconcile-auto-evolution.md §2 (L12) 指出输入 §1 的 5 步 pipeline (GenericAgent 证据 → Laputa 文件 → AutoDream 提示词 → 报告/提案草稿 → 审查/策略 → 写 authority + changelog) 在 PRD 零覆盖
- 没有 Success Metrics 段 (本 PRD 全无 SM), 对 launch 级 PRD 是缺口
- "GenericAgent as Trunk + 8 owned disciplines" 反向断言 (reconcile-auto-evolution.md §3) 缺 — PRD §1 没声明 "Laputa / AutoDream 不取代 GenericAgent runtime"

### Findings
- **[high]** §5 缺 1 条 named-loop FR 把 4 接口串成端到端闭环 (reconcile-auto-evolution.md actionable #1)。*Fix:* 新增 1 条 FR (e.g. FR-109 "End-to-End P0 Loop"): "Laputa MUST 在 GenericAgent→AutoDream→review→authority+changelog 5 步闭环中扮演 file authority + review gate + changelog 3 角色; FR-101/2xx/3xx/4xx 是闭环切片, 闭环契约见本条"
- **[medium]** 缺 Success Metrics 段, 对 launch 级 PRD 是缺口 (尤其 D-006 说 "完成后单独发 Sprint Change Proposal 拆 stub", 拆 stub 的成功标准未量化)。*Fix:* 加 1 段 Success Metrics, 至少含: (a) stub 消除率 (FR-5xx stub → 实接); (b) 写回滚率 (30 天内 changelog rollback 调用次数); (c) 治理拦截率 (review 拒绝 / conflict needs_attention 次数)
- **[medium]** §1 Vision 缺 "GenericAgent as Trunk" 反向断言 (reconcile-auto-evolution.md actionable #2)。*Fix:* §1 加 1 句: "Laputa 是 subject file layer, 不取代 GenericAgent runtime; 8 项 owned disciplines (AgentLoop / SessionManager / ContextBuilder / tool registry / subagent isolation / event bus / heartbeat+cron / provider routing / MemoryProvider boundary) 归 GenericAgent"
- **[low]** §6 OQ #12-#18 的 7 项 follow-up PRD 用一句 "转 X PRD" 描述 (L300-306), 缺每个 follow-up PRD 的入门 contract 引用, 跨 PRD 引用易断章。*Fix:* §6 加 follow-up PRD 名 + 入口 contract 行 (e.g. "Laputa 架构后续 PRD: 三轴主体性 4 级自主阶梯数据契约")

---

## Done-ness clarity — strong

32 条 FRs 几乎每条都带 "验收:" 段, 给具体 verifiable 条件 — 不依赖形容词。

代表性 sample:
- FR-101 验收 (L100): "stub 接 stub, mock 返回 ChangelogRecord; 实接后 changelog +1"
- FR-102 验收 (L113): "8 种 proposal_type 各路由到正确 section, TBD 类型返 Err"
- FR-104 验收 (L126): "2 个并发写同 target_section, 后者等前者完成; 2 个并发写不同 target_section, 几乎同时完成"
- FR-203 验收 (L149): "写 1 个 section 后, 带 since=写前时间查询, 返该 section 1 个; since=写后时间查询, 返空"
- FR-401 验收 (L180): "mock 100 条 changelog, page=1 page_size=20 返 20 条, has_more=true; filter target_section=identity 返命中条数"
- FR-702 验收 (L264): "grep 整个 agent-diva-pro 仓库, `fs::write(.*\.laputa` 命中 0 处 (除 LaputaWrite 实现内)"

唯一软肋:
- TBD sections (#10-14) 的 acceptance 只到 "返 status=tbd" 级 (FR-205 L155-156), 缺 4 阶段 staged retrieval 能力锚定 (reconcile-mentle-laputa.md actionable #4)
- FR-503 迁移策略 (L218-224) 改 `sync_workspace_templates()` 的 regression 测试未具体化

### Findings
- **[medium]** TBD sections (#10-14) 缺 capability anchor — 仅 L138-141 的 FR-205 标 `status=tbd`, 但 #13 report_indexes + #14 AAAK_summaries 应该有 "4 阶段 staged retrieval" 能力要求 (reconcile-mentle-laputa.md §5 + reconcile-auto-evolution.md §15 同源)。*Fix:* §4.1 给 TBD section 列表加 1 列 "capability anchor", #13/14 至少锁 4 阶段检索路径
- **[medium]** FR-503 (L218-224) 改 `sync_workspace_templates()` 验收仅 "新装 / 旧装升级 启动 1 次, .laputa/state.json 存在", 缺具体 regression 测试用例。*Fix:* §6 OQ #10 (L296 "旧 8 模板 → 1 稀薄层 迁移测试策略") 升 P0 + 在 FR-503 补 acceptance 例子: "旧 8 文件含 N 条 MEMORY 记录, 迁后 14 section #5 memory_md 含 N 条, 旧文件备份在 .laputa/legacy/{ISO8601}/"
- **[low]** FR-704 性能验收 (L272) "≤200ms 总" 对 100 并发写 14 section 是宽松指标, 缺 P99 / P95 分位。*Fix:* 后续 polish 可补 P99 阈值

---

## Scope honesty — adequate

显式信号:
- §4.3 不在本 PRD 范围 (L87-90) 3 条: 旧 8 模板保留兼容 / Mentle 实现本身 / AutoDream/Report System/Selfinprove 内部逻辑
- §6 OQ 20 条 (L286-309) 是诚实信号 — 5 关 / 3 半关 / 3 开 (来自子 agent 调研 1-9) + 12-18 转 follow-up PRD / 19-20 P1 本 PRD 补
- 3 份 reconcile 文件摊在桌面上 (而不是埋在 PRD 后), 这是 scope honesty 的强证据

显式缺位:
- 没有 Non-Goals 段 — §4.3 是 "out of scope" 不是 "non-goals", 区别是 non-goals 含 "本 PRD 故意不做且未来也不做" 的断言
- 0 个 `[ASSUMPTION: …]` tag 内联 + 0 个 `[NOTE FOR PM]` callout (全 PRD grep 0 命中)
- 输入 C §26 的 9 条 P0 non-goals (reconcile-auto-evolution.md §8 actionable #8) 仅 5 条靠 FR 隐含 (不默认 Mentle 注入 / AutoDream 不直写 authority / 复杂共振 / 全图记忆 / context compaction 与 AutoDream 混), 4 条零显式 (AutoDream 不直写 authority / GenericAgent runtime 不直管 Laputa / MEMORY.md 不当 raw archive / 不把全 audit 当 P0 前提)
- §6 OQ #4 (L290) "SOUL.md AI 自写, 用户怎么干预" 仍开, reconcile-mentle-laputa.md actionable #5 + reconcile-auto-evolution.md actionable #8 都指出应升 P0

### Findings
- **[high]** 缺 Non-Goals 段 (reconcile-auto-evolution.md actionable #8.②)。*Fix:* §4 加 §4.4 Non-Goals, 至少列 4 条零显式项: (a) "Laputa MUST NOT replace GenericAgent runtime"; (b) "AutoDream MUST NOT 直写 authority, 仅产 drafts/proposals"; (c) "MEMORY.md MUST NOT 当 raw archive"; (d) "Audit MUST NOT be P0 全 audit 前提 (走 30 天窗口内 changelog + grep 检查)"
- **[medium]** 0 个 `[ASSUMPTION]` / `[NOTE FOR PM]` callout — 跟 32 条高密度 FR 的严谨度不匹配, polish 阶段应回填。*Fix:* 在 §6 OQ 半关 / 开项前补 inline `[ASSUMPTION: …]` tag, polish 阶段做 roundtrip 索引
- **[medium]** §6 OQ #4 (L290) "SOUL.md AI 自写, 用户怎么干预" 应升 P0 (reconcile-mentle-laputa.md actionable #5 + reconcile-auto-evolution.md actionable #8.①)。*Fix:* §6 OQ #4 升 P0 + FR-501 (L196-203) 补 1 条子要求, 把 #1 identity / #3 commitment / #4 preferences 各归 `write_authority` 哪类

---

## Downstream usability — adequate

强信号:
- FR ID 连续: §5.1 FR-1xx (6 条) / §5.2 FR-2xx (5 条) / §5.3 FR-3xx (3 条) / §5.4 FR-4xx (3 条) / §5.5 FR-5xx (6 条) / §5.6 FR-6xx (3 条) / §5.7 FR-7xx (6 条) — 共 32 条无 gap 无重号
- 跨节 cross-ref 解析: FR-204 (L151) 引用 FR-401 / FR-101 (L99) 引用 FR-501 stub / FR-403 (L188) 复用 FR-103 atomic / FR-602 (L246) 复用 FR-2xx
- 既有代码引用精确: agent-diva-core/src/utils/mod.rs:37-66 (L84, L206) / mod.rs:91-105 (L219) / agent-diva-agent/src/mentle_runtime.rs (L241) / agent-diva-agent/src/agent_loop.rs:570 (L242) / agent-diva-manager/src/runtime.rs:359-365 (决策日志 D-002)
- domain noun 跨节一致: Laputa / Mentle / AutoDream / Report System / selfinprove / GenericAgent — 跨 reconcile 文件一致

缺口:
- 无 Glossary 段, "MEMORY.md" 在 §4.1 表格 (L77) 写 "MEMORY.md" 但 §4.1 表格下方 schema 段 (L195) 写 "memory_md" (snake_case) — 同一概念两种写法, 下游 UX/architecture 引用时易混
- TBD sections (#10-14) 在 §4.1 用位置编号, 在 §5 FR-202 (L144) 用 name 枚举 ("journal_reflective" / "proposal_inbox" / "changelog" / "report_indexes" / "aaak_summaries") — 两套命名 (编号 vs snake_case), 跨节引用需手动翻译
- 无 UJ 段 (这是 internal tool capability spec shape, OK 不算 gap), 无 SM 段 (Strategic coherence 已标)

### Findings
- **[medium]** Glossary 缺失 + "MEMORY.md" vs "memory_md" 术语 drift (L77 vs L195)。*Fix:* §0 之后或 §4 之前加 Glossary 段, 至少含 14 section 的 section_id ↔ display_name ↔ file_path 三列映射
- **[medium]** TBD section 命名两套 (位置编号 #10-14 vs snake_case journal_reflective/proposal_inbox/changelog/report_indexes/aaak_summaries, FR-202 L144)。*Fix:* §4.1 表格加 1 列 "section_id (snake_case)", FR-202 用 section_id 枚举
- **[low]** "changelog" 同时是 #12 TBD section name (FR-202 L144) 跟 §5.4 FR-4xx "changelog" API name, 同名词不同义。*Fix:* Glossary 显式标 "section_id=`changelog` 是 14 section 的 #12; FR-4xx changelog API 是 audit log, 不冲突但易混"

---

## Shape fit — adequate (with mismatches)

PRD 是 internal tool / brownfield / capability spec / chain-top shape, 跟 bmad-prd 模板对齐良好:
- §4 写成 capability spec (无 UJ / 无 persona) — 对 internal tool OK
- §5 FR 详尽到实现级 (file:line 引用 + 验收) — 对 launch 级 + chain-top shape 必要
- §6 OQ 显式留 20 条 — 对 chain-top shape (后续 UX / architecture / stories) 必要
- Brownfield 引用全部精确 (mod.rs:37-66 / mod.rs:91-105 / agent_loop.rs:570 / provider.rs:125-139 / runtime.rs:359-365), 可信赖

Shape 不匹配点:
- §0/§2/§3 占位符跟 launch 级 stakes 不匹配 — D-001 L18 明示 "stakes = launch 级", 但 §0-§3 是 "Coaching 阶段未填" 占位
- §1 Vision 不是 vision statement 是 state report — 跟 launch 级 PRD 的 vision 节形状不匹配
- §4 缺 Non-Goals 段 — 跟 launch 级 PRD 的 scope honesty 形状不匹配 (虽然 §4.3 有 3 条 out-of-scope 但 out-of-scope ≠ non-goals)

### Findings
- **[critical]** launch 级 stakes (D-001 L18) 跟 framing 层占位符不匹配 — PRD 当前 shape 是 "draft / Coaching 协作中" (L21-22), 但 launch review gate 期望 ready-to-ship shape。*Fix:* 在 §0 顶部加 "Status: ready-for-reviewer-gate" 替代 "draft (Coaching path)", 同时填补 §0/§1/§2/§3 (同 Decision-readiness 3 条 fix)
- **[medium]** §4 缺 Non-Goals 段, 当前 §4.3 仅 3 条 out-of-scope 跟 launch 级 PRD 的 scope honesty 形状不匹配。*Fix:* 同 Scope honesty [high] fix
- **[low]** Addendum / Open Questions / Revision Notes (L310-322) 3 段是 bmad-prd 模板结构, 但 Addendum 显式标 "(空 — 材料汇总后由子 agent extract 落入)", 是 shape noise。*Fix:* Addendum 段删空标, 或留 1 句 "本 PRD 不需 addendum"

---

## Mechanical notes

(轻量, 不驱动 overall verdict)

- **Glossary drift**: "MEMORY.md" (§4.1 L77) vs "memory_md" (§5.5 L195); section 命名两套 (位置编号 #10-14 vs snake_case); "changelog" 一词二义 (section #12 vs FR-4xx audit log)
- **ID continuity**: 32 条 FRs 无 gap 无重号, FR-204 ↔ FR-401 cross-ref 解析, FR-602 ↔ FR-2xx 复用解析
- **Assumptions Index roundtrip**: 缺 — 0 个 `[ASSUMPTION]` 内联 tag, 0 个 Assumptions Index 段, polish 阶段应回填
- **UJ protagonist naming**: N/A — internal tool capability spec shape, 无 UJ 段是合理的
- **Required sections presence**: bmad-prd 模板节基本齐 (§0/§1/§2/§3/§4/§5/§6/Addendum/Revision Notes), 但 §0/§2/§3 是占位, §4 缺 Non-Goals 段

---

## Summary scorecard

| 维度 | 判定 | 主要驱动 |
|---|---|---|
| Decision-readiness | adequate | 决策日志扎实, §0-§3 占位 + §1 不是 thesis |
| Substance over theater | strong (§5) / broken (framing) | §5 反 theater 标杆, framing 层占位 |
| Strategic coherence | adequate | 隐含 thesis 强, 缺端到端 loop FR + GenericAgent 反向断言 |
| Done-ness clarity | strong | 32 条 FRs 全带验收, TBD section 缺 capability anchor |
| Scope honesty | adequate | §4.3 + §6 OQ + reconcile 强, 缺 Non-Goals 段 + 0 个 [ASSUMPTION] |
| Downstream usability | adequate | FR ID 连续 + cross-ref 解析 + brownfield 引用精确, Glossary 缺 |
| Shape fit | adequate | capability spec shape OK, launch 级 stakes 跟 framing 占位不匹配 |

**总判定**: adequate — §5 强 (§5 FRs 可直接驱动工程师开工), framing 层弱 (§0-§3 占位 + §1 不是 thesis + §4 缺 Non-Goals), polish 阶段补 4-5 条 fix 可升 strong。
