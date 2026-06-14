# Reconcile: mentle-laputa-memory-role-decision.md vs Laputa PRD (prd-laputa-2026-06-12)

> 来源输入: docs/dev/genericagent/mentle-laputa-memory-role-decision.md (2026-05-31, 295 行, 状态: accepted architecture direction)
> 比对对象: docs/prds/prd-laputa-2026-06-12/prd.md (v0.0.3, 311 行)
> 辅助参考: .decision-log.md (D-002/D-005/D-007) + 已存在的 reconcile-laputa-new-architecture.md
> 性质: 输入文档是"Mentle/Laputa/AutoDream/Harness 边界 + work_memory schema + recall/write/propose/approve 时机"的 accepted architecture decision。
>       PRD 已锁 FR-6xx Laputa↔Mentle 写权限边界 + FR-603 recall 注入策略。比对聚焦"PRD 是否漏掉输入的设计意图 / 约束 / 未决", 不展开实现细节。

## Summary (5–8 句)

1. 输入是 2026-05-31 的"Mentle↔Laputa↔AutoDream↔Harness 边界决策 + work memory schema + 时机矩阵"已落档架构方向, PRD 在 2026-06-12 经 FR-6xx 锁了 Laputa↔Mentle 写权限边界 + FR-603 锁 recall 注入策略, 主体方向一致, **不冲突**。
2. **PRD 完全漏掉输入 §4 的"Mentle 5 个 CRUD wing surface (search/read/write/update/delete_or_archive, 可选 link)"契约** — mentle_runtime.rs 已实现这 5 个 wing, 但 Laputa PRD FR-6xx 不引用具体工具面, 工具面契约散落在 mentle 集成文档 + CI 脚本, 无 Laputa PRD FR 锚定, 后续若有 wing 改名/删减无 PRD 留痕。
3. **PRD 完全漏掉输入 §5 的"Mentle 该写 vs 不该写"内容白黑名单** — 9 项高优先级写候选 + 8 项不该写 (含 casual chat / 不稳定情绪 / 未确认人格推断 / 敏感隐私 / 原始模型臆测 / 项目文件已有事实 / Laputa-authority 内容 / 静默变长期身份记忆) + 1 条硬规则, PRD FR-6xx 只处理 recall 注入策略, **Mentle 写策略在 PRD 零覆盖**, 是 governance 缺位。
4. **PRD 完全漏掉输入 §10 的"Mentle temporary work memory"能力级 schema** — `work_memory/<project>/{current_goal, constraints, decisions, open_questions, evidence, next_actions, useful_links}` 7 字段 + task start recall / task progress update / task end archive 3 阶段生命周期, PRD 无任何 FR 描述 work memory, 也未定 work memory 归属 (Mentle 暂存 vs Laputa 持久)。
5. **PRD 完全漏掉输入 §9 的"4 阶段 staged retrieval 路径"** — index → AAAK summary → full report → source sessions; 同时未定义 Mentle index 该含什么字段 (titles / dates / participants / AAAK summaries / keywords / links to full reports / links to source sessions); 这直接影响 #13 report_indexes + #14 AAAK summaries TBD section 的 schema 设计能力级。
6. **PRD 部分覆盖输入 §12 的 4 级 authority ordering + 5 条 MUST NOT 否定规则** — FR-601 提 principle 但未列 4 级顺序 (Laputa authority > changelog/review > reports/journals > Mentle indexes/notes/work-memory); 否定规则 (must NOT silently rewrite MEMORY.md / must NOT silently rewrite identity files / must NOT 默认进 context) 仅靠 review/changelog 间接保护, 无显式否定契约。
7. **PRD §6 OQ 没有把"casual 聊天 → 长期身份记忆 反向保护"列入 P0** — 输入 §5+§8+§12 三处强调"不可静默变长期身份记忆", 跟 D-005 §4 自治哲学一致, 应在 Laputa PRD 留 1 条 FR 显式拒绝此通道, 而不是只靠 review 流程间接拦截。
8. **PRD FR-603 `mentle_recall_policy="always"` 选项跟输入 §3 + §12 "notebook on the desk, 不可默认进 context" 设计哲学有 soft drift** — 输入原则是 "默认 on_demand, 不入 prompt", "always" 留配置项等于开了一条违背原则的通道, 应在 PRD 注明 "always = escape hatch, 非推荐值", 避免后人误读为 "PRD 已支持 always 注入, 故可作默认"。

## Gap Table

| PRD 段 | 输入对应段 | 状态 | 说明 |
|---|---|---|---|
| FR-601 (Laputa 写 authority, Mentle 仅 recall) | §1 / §3 / §6 / §12 Mentle 不背 authority | **covered** | 主体方向一致, OK |
| FR-602 (Mentle recall 走 LaputaReadAdapter HTTP) | §3 + §11 (Laputa 路径, 不直读物理文件) | **covered** | 方向一致, OK |
| FR-603 `mentle_recall_policy: off/on_demand/always` (默认 on_demand) | §3 + §8 + §12 (默认不开, 笔记本隐喻, harness 决定) | **partial / soft drift** | "always" 选项违反输入原则 "不可默认注入"; 缺 harness 5 触发条件枚举 (见下方专项) |
| FR-1xx 写治理 4 道 (review/changelog/audit/rollback) | (输入无对应) | **PRD 自主补齐** | 输入未提治理, PRD 详写, 合理 |
| FR-2xx~5xx 读 / 事件 / changelog / schema / 迁移 | (输入无对应) | **PRD 自主补齐** | 输入未提, PRD 自主补齐, 合理 |
| FR-7xx NFR (性能 / 审计完备 / 撤销 / 并发 / 错误 / 可观测) | (输入无对应) | **PRD 自主补齐** | 输入未提, PRD 自主补齐, 合理 |
| (无对应 PRD FR) | **§4 Mentle 5 个 CRUD wing surface (search/read/write/update/delete_or_archive, 可选 link)** | **missing in PRD** | 输入明确工具面 = 5+1 wing; mentle_runtime.rs 真实实现这 5 wing; PRD FR-6xx 仅写 boundary, **不引用具体 wing 名字**, 工具面契约散落 mentle 集成文档, 无 Laputa PRD FR 锚定 |
| (无对应 PRD FR) | **§5 Mentle 写白名单 (9 项高优) + 黑名单 (8 项不该写) + 硬规则 "不可静默变长期身份记忆"** | **missing in PRD** | 9+8 内容候选清单 + 1 关键规则; PRD 无任何 FR 覆盖 Mentle 写策略; §6 OQ 不含; governance 缺位 |
| (无对应 PRD FR) | **§7 AutoDream → Laputa 8 步 flow (sessions → Laputa → optional Mentle recall → reports / proposals → Laputa inbox → MEMORY/HISTORY 升级 after review)** | **missing in PRD / scope drift** | FR-1xx 写入口是 generic apply_proposal; **AutoDream 接入契约 (何时调 apply_proposal, 走哪些 proposal_type, 跟 review 的衔接) 无对应 FR**; 属 autodream PRD 范围, 但 Laputa PRD 应留 1 条"AutoDream 写必经 Laputa 入口"反向断言 |
| (无对应 PRD FR) | **§8 Harness / Prompt 5 条决策规则 (何时调 recall, 何时 write, 何时 review, 何时拒, 何时不入 context)** | **missing in PRD** | 输入给 5 条触发器; PRD FR-603 仅覆盖 recall 注入 1 条; write 触发器 / review 触发器 / 拒绝触发器均无 FR; 是 harness policy 缺位 |
| (无对应 PRD FR) | **§9 4 阶段 staged retrieval (index → AAAK summary → full report → source sessions)** | **missing in PRD** | 输入明确 4 阶段能力级路径; PRD #13 report_indexes + #14 AAAK 标 TBD 但**未锁 4 阶段能力要求**, 也未定义 Mentle index 字段列表 (titles/dates/participants/AAAK/keywords/links) |
| (无对应 PRD FR) | **§10 Mentle temporary work memory schema (work_memory/<project>/7 字段) + 3 阶段生命周期** | **missing in PRD** | 7 字段 schema + start recall / progress update / end archive 3 阶段; PRD 无 FR 描述 work memory; 也未定 work memory 归属 (Mentle 暂存 vs Laputa 持久) |
| (无对应 PRD FR) | **§12 Authority 4 级 ordering (Laputa>changelog/review>reports/journals>Mentle) + 5 条 MUST NOT 否定规则** | **partial** | FR-601 提 principle, 未列 4 级顺序; 否定规则 (must NOT silently rewrite MEMORY.md / identity files / must NOT 默认进 context / must NOT silently become long-term identity / must NOT 绑死 physical layout) 无显式否定契约, 仅靠 review/changelog 间接保护 |
| §6 OQ#4 (开) SOUL.md AI 自写用户怎么干预 | (输入未提用户干预) | **out of input scope** | 输入是 boundary 决策不涉及 user intervention, 留开合理 |
| (无对应 PRD 段) | **§5+§8+§12 "casual 聊天 → 长期身份记忆" 反向保护** | **missing in PRD (governance gap)** | 输入三处强调"不可静默变长期身份记忆"; PRD FR-103 review 流程仅间接拦截, **无显式反向拒绝 FR**; §6 OQ 不含此 P0 |
| (无对应 PRD 段) | **§11 Obsidian 集成方向 (Laputa=authority / Obsidian=human-facing / Mentle=machine-facing 三向 split)** | **drift / silently deferred** | 输入明确未来 Obsidian 走同样 split; PRD 完全不提 Obsidian, **无 deferred 声明也无接口预留**, 后人若查 PRD 找 Obsidian 入口无着落 |
| (无对应 PRD 段) | **§13 7 步实施顺序 (AutoDream 先 → Laputa 写入 → Mentle wing → report index → work memory → AutoDream+Mentle 联通 → Obsidian 最后)** | **drift** | 输入给 7 步 phasing; PRD 仅 D-006 锁"实接深度 = 全 (写+读+事件+changelog 4 接口)", **无 phasing 段落**, 一次 ship 全部 stub 消除风险高, 跟输入"先核心后可选"建议违背 |
| §4.1 14 section 分配表 | §6 Laputa-owned 文件清单 (identity/relationship/commitment/reports/journal/proposal/MEMORY/HISTORY/changelog/patches) | **partial** | 输入提"GenericAgent 不应直散这些文件, 应调 Laputa-facing boundary"; PRD FR-101 统一入口 OK, **但"reviewable long-term memory patches"在 14 section 分配表中无显式归属** (可能并入 #1 identity 或 #4 preferences 但未定) |

## 关键 actionable 提示 (给主 agent)

1. **PRD §5 FR-6xx 应显式引用 Mentle 5 wing surface** (search/read/write/update/delete_or_archive, optional link) — 即使 wing 实现在 mentle_runtime.rs, Laputa PRD 也应留 1 条 FR 锚定工具面契约, 避免 wing 改名/删减无 PRD 留痕。
2. **PRD §5 应新增 FR 关于 Mentle 写策略**: 9 项高优写候选 + 8 项不该写 + "不可静默变长期身份记忆" 硬规则 (输入 §5+§12) — 这是 governance 缺位, 必须落到 FR 级才能被 code review 检查。
3. **PRD §5 应新增 FR 关于 Mentle temporary work memory**: 7 字段 schema + 3 阶段生命周期 + 归属 (建议 Mentle 暂存, Laputa 不背 work memory)。
4. **PRD #13 report_indexes + #14 AAAK summaries TBD 段应升级**: 即使内容 schema 仍 TBD, 也应锁"4 阶段 staged retrieval"能力级 + Mentle index 字段列表 (titles/dates/participants/AAAK/keywords/links) — 否则 TBD 永远 TBD, 无能力级锚点。
5. **PRD §6 OQ 应新增 "casual 聊天 → 长期身份记忆 反向保护" 列为 P0** — 输入 §5+§8+§12 三处强调, 跟 D-005 §4 自治哲学一致, 应在 Laputa PRD 留 1 条 FR 显式拒绝 "casual→identity" 通道, 不只靠 review 流程间接拦截。
6. **PRD FR-603 `mentle_recall_policy="always"` 应注明 "escape hatch, 非推荐值"** — 输入 §3+§12 原则是"notebook on the desk, 不默认注入"; 当前 PRD 把 "always" 跟 "off/on_demand" 平级, 易被误读为推荐值。
7. **PRD §4.1 14 section 分配表应显式声明 "Obsidian 集成 deferred, 当前无预留接口"** 或给 1 个 deferred section placeholder — 避免后人查 PRD 找 Obsidian 入口无着落。
8. **PRD 应新增 phasing 段落或附录**: 输入 §13 给 7 步实施顺序 (先核心 Laputa 写 + report index → 后可选 Mentle wing + work memory + Obsidian), PRD 仅 D-006 "实接深度=全" 无 phasing, 一次 ship 全部 stub 消除风险高, 违反输入"先核心后可选"建议。