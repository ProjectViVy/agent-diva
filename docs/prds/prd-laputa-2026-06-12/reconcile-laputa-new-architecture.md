# Reconcile: laputa-new-architecture.md vs Laputa PRD (prd-laputa-2026-06-12)

> **Superseded amendment (2026-07-30):** Mentle-retention conclusions below
> no longer define the target. Embedded Laputa owns local storage/retrieval and
> Mentle is removed cleanly under GMH-23A..24.

> 来源输入: docs/dev/laputa-new-architecture.md (2026-05-28, 32 行设计索引)
> 比对对象: docs/prds/prd-laputa-2026-06-12/prd.md (v0.0.3, ~310 行)
> 辅助参考: .decision-log.md (D-002/D-003/D-005/D-007)
> 性质: 输入文档是早期 7 文件原型; PRD 已锁定 1 稀薄层 14 section 布局。比对聚焦
>       "PRD 是否漏掉了输入的设计意图 / 约束 / 未决", 不展开实现细节。

## Summary (5–8 句)

1. 输入文档是 2026-05-28 的"7 文件 + 三轴主体性"原型, PRD 在 2026-06-12 经 D-007 改走"1 稀薄层 14 section", 文件层属合法 drift, 无须回退。
2. **PRD 完全漏掉输入文档的"三轴主体性 (自指 / 自反 / 自主)"整套能力级**: SelfModel + SoulSignal 三分类、5 问结构化自反、4 级自主阶梯均无对应 FR, 决策日志 §6 #2 只把它推到"Laputa 架构后续 PRD"。
3. **PRD 完全漏掉输入文档的"进阶心跳 (每 4h LLM 评估 + 子代理委派)"机制**, 无周期 / 评估器 / 子代理路由任何 FR, 心跳属于本 PRD 边界外。
4. **PRD 把 autodream 从输入的"每日整理产出方"降级为"只读消费方 (FR-7 Orient)"**, 输入描述的"SOP/Skill 产出 / SOUL 演化 / MEMORY 压缩"4 项产物在 PRD 无任何写接口归属, 是显著 drift。
5. **设计哲学 (用户=父母, agent 自主决定成为什么) 未落到能力级**: PRD 有 #3 commitment 与 #4 preferences section, 但没有写权限契约区分"用户可改 vs agent 自改", 仅 §6 OQ#4 留开。
6. **rhythm/*.md / sop/*.md 扩展机制**: 输入明确提 rhythm 与 sop 子目录, PRD 仅有 FR-102 sop_create 隐含并入 #1 sub-section, rhythm 概念完全消失。
7. **输入文档不提的能力 (写治理 4 道 / 三向合并 / 30 天回滚 / SSE+60s 轮询 / schema_version / NFR) PRD 全本**, 属 PRD 自主补齐, 不算 gap。
8. mentle 在输入是"日常 4 工具 vs autodream 30+ 工具集差异", 在 PRD 是"Laputa 写 authority / Mentle 仅 recall 边界", 两者**对象不同**, 输入的工具集使用模式未沉淀到 PRD。

## Gap Table

| PRD 段 | 输入对应段 | 状态 | 说明 |
|---|---|---|---|
| §1 Vision (Laputa = 稀薄层) | 一句话 (极简 7 文件身份管理) | **drift (resolved)** | D-007 已锁 1 稀薄层; 输入 7 文件是早期原型, 已被替换, 不需回退 |
| §4.1 14 content section 分配 (Laputa-owned 6 + Report-owned 3 + TBD 5) | 设计核心 — 7 文件清单 | **drift (resolved)** | D-003 记录 3 套清单之争, D-005+D-007 落档 14 段; 输入 7 文件清单已扩, 7→14 路径走 FR-502 |
| FR-1xx 写接口 4 道治理 (review/changelog/audit/rollback) | (输入无对应) | **covered in PRD, missing in input** | 输入未提写权限契约, PRD 自主补齐 |
| FR-2xx 读接口 (snapshot / per-section / since 增量) | (输入无对应) | **covered in PRD, missing in input** | 输入未提读接口契约 |
| FR-3xx 事件接口 (SSE + 60s 轮询 fallback) | (输入无对应) | **covered in PRD, missing in input** | 输入未提实时订阅契约 |
| FR-4xx changelog (list/detail/rollback 30 天) | (输入无对应) | **covered in PRD, missing in input** | 输入未提变更历史回滚契约 |
| FR-5xx 14 schema + 旧 8 模板迁移 (FR-501 ~ FR-506) | 设计核心 — 7 文件 | **partial** | 输入给 7 文件原型, PRD 给 8→14 映射表 + schema_version; 但**输入的 rhythm/sop/expectations 在 14 section 中无显式归属**, 见下方专项 |
| FR-6xx Mentle 边界 (Laputa 写 authority / Mentle recall) | 设计核心 — mentle 简化 (日常 4 工具 / autodream 30+) | **partial / scope drift** | 输入提 mentle **工具集使用模式差异**, PRD 锁 mentle **写权限边界**; 输入的"日常 4 工具"未沉淀到任何 PRD FR |
| FR-7xx NFR (性能 / 审计完备 / 撤销 / 并发 / 错误 / 可观测) | (输入无对应) | **covered in PRD, missing in input** | 输入未提 NFR, PRD 自主补齐 |
| (无对应 PRD 段) | **三轴 — 自指 (SelfModel + SoulSignal Rule/Identity/Preference)** | **missing in PRD** | 输入明确给出数据结构 + 三分类; PRD 无 SelfModel / SoulSignal 任何字段、无 §1 identity section 内分类路由; 决策日志 §6 #2 关闭但**未落到任何 FR** |
| (无对应 PRD 段) | **三轴 — 自反 (5 问结构化提示词审视行为)** | **missing in PRD** | 输入明确给"5 问审视"作为自反能力; PRD 无触发器 / 调用方 / 产物落点 (写到哪个 section: identity? commitment? preferences?), 决策日志 §6 #2 关闭 |
| (无对应 PRD 段) | **三轴 — 自主 (4 级: 被动→反应式→主动式→涌现式)** | **missing in PRD** | 输入明确 4 级自主阶梯; 决策日志 §6 #2 明确"留 Laputa 架构后续 PRD", **本 PRD 不背**, 但 PRD §1 Vision 也未声明此边界外推 |
| (无对应 PRD 段) | **进阶心跳 (每 4h, LLM 评估 + 委派子代理, 本体不做事)** | **missing in PRD** | 输入明确周期 + 评估器 + 委派机制; PRD 无任何心跳 FR (FR-7xx NFR 不含心跳), 决策日志 §6 #2 关闭并外推 |
| §4.1 #1 identity / §5.1 FR-102 写路由 | **autodream 每日整理 (日报 + SOP/Skill 产出 + SOUL 演化 + MEMORY 压缩)** | **drift** | 输入把 autodream 描述为 **Laputa 写消费方/产出方** (产 SOP, 压 MEMORY, 演 SOUL); PRD 把 autodream 降为**只读消费方 (FR-7 Orient)**, 4 项产出机制在 PRD 无任何写入口归属 |
| §4.1 #3 commitment / #4 preferences | **设计哲学 (用户=父母, expectations.md 表达期望, SOUL.md = agent 自主决定成为什么)** | **partial / intent drift** | PRD 有 commitment / preferences section, 但**未把"用户表达期望 vs agent 自主决定"的边界写进写权限契约**; §6 OQ#4 仍开: SOUL "AI 自写, 用户不可直接编辑" — 用户怎么干预 |
| FR-102 sop_create 路由 (合并入 #1 sub-section) | **rhythm/*.md / sop/*.md 扩展子目录** | **partial** | 输入明确 rhythm 与 sop 是 7 文件的扩展机制; PRD 隐含 sop 但**rhythm 概念完全消失**, FR-501 schema 字段 (content_type / render_layer / max_size_kb) 也不覆盖 rhythm |
| FR-502 旧 8 → 新 14 映射表 | 设计核心 — 7 文件 (含 rhythm/sop/expectations) | **partial** | 输入的 rhythm / sop / expectations 三个文件在 FR-502 映射表中**无显式目标 section**; rhythm 消失, sop 隐入 #1, expectations 归属 commitment 还是 preferences 未定 |
| §1 Vision (Laputa-owned: identity/relationship/commitment/preferences/memory_md/history_md) | **expectations.md = 父母表达期望** | **missing / ambiguous** | 输入明确"expectations.md"是用户表达期望的文件, PRD 14 section 中**无 expectations 显式 section**, 可能归属 #3 commitment 或 #4 preferences 但未定 |
| (无对应 PRD 段) | **mentle 工具集使用模式 (日常 4 工具 vs autodream 30+)** | **missing in PRD** | 输入提 mentle 工具集差异 (日常 CRUD 4 个, autodream 全量 30+), PRD 仅定 Laputa↔Mentle 写边界, **未规定 mentle 何时用全量何时用简化集** |
| §6 OQ#4 (开) | 设计哲学 — 用户如何干预 agent 自主 | **partial** | 输入明确"expectations.md 是用户干预通道", PRD §6 OQ#4 留开, 但**未把 expectations.md 列入 14 section 分配表** |

## 关键 actionable 提示 (给主 agent)

1. PRD §1 Vision 应**显式声明三轴主体性 + 进阶心跳属边界外推** (指向后续 PRD), 避免后人误读为"PRD 没写所以不支持"。
2. PRD §4.1 14 section 分配表需补: rhythm / expectations / sop 在 14 section 中的**显式归属或显式标 TBD**, 当前是 silent gap。
3. PRD §5 FR-102 sop_create 路由 + FR-501 schema 字段应**显式覆盖 rhythm 类内容**, 否则子代理 / 心跳产出的 rhythm 条目无落点。
4. PRD §6 OQ#4 应**升级到 P0**: 输入文档已把"用户=父母 / agent 自主决定"作为核心哲学, 但 PRD 写权限契约 (FR-1xx) 全程未区分"用户可改 vs agent 自改"。
5. PRD §5 应**为 autodream 的 SOP/Skill 产出 / SOUL 演化 / MEMORY 压缩 4 项产物指定写入口** (FR-1xx 通用入口 + section 归属), 否则 autodream 只能继续走 pending.jsonl 降级契约 (见 §6 OQ#7 半关)。
