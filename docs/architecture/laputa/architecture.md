# Laputa 现行架构

- 状态：`Approved Product Architecture / Implementation Pending`
- 汇总日期：2026-08-14
- 性质：把 2026-08-12 至 08-14 已冻产品决策收成一份可读架构。**不是**当前 Rust 实现说明书，也**不是** Research Gate 已过、可以开工的许可证。
- 决策原文：[sources.md](./sources.md)
- 旧说法黑名单：[do-not-read-as-current.md](./do-not-read-as-current.md)

实施门禁：Research Gate **部分通过**（R0/R2/R3/R4 过，R1 挂起）。不得按本文改生产认知主链。D3 / GA 进化落地停到 R1 自主进化说清。本文冻结的是**产品形状**。

---

## 1. Laputa 是什么

Laputa 是 Diva 的**人格与认知治理面**：人能打开的 Markdown 权威、内容审查、完整历史、Frozen Core 投影。它不是数据库，不是 Agent 循环，不是调度器，不是 BML，不是 ACTMEM，不是 Skill。

整机 agent-diva **只有一套**人格（P22）：七份权威同一目录。不按 git 项目复制，也不按 profile 再开一套伴侣。哲学：更好对待当前这个伙伴，而不是跟多个 agent 卿卿我我。

和它分开、但经常被叫错名字的东西：

| 名字 | 是什么 | 不是 Laputa 七文件 |
| --- | --- | --- |
| **BML** | 普通长期记忆（LTM）唯一权威；`memory.sqlite3` 跟人格同一套家 | 是 |
| **ACTMEM.MD** | 跨会话活动记忆文件（STM 概念的落地文件名） | 是 |
| **Session transcript** | 某个聊天的原文 | 是 |
| **SessionCheckpoint / working_memory** | 会话级草稿，结束可清 | 是 |
| **CanonicalCheckpoint** | 本会话 compact 摘要 | 是 |
| **Skill / Evolution** | 可复用能力 | 是 |
| **MEMRULES.MD** | 记忆写入手册（S9），不是人格 | 是 |
| **Mask** | 临时外在覆层；摘下后本体仍在 | 是 |

概念上仍可说 **STM / LTM**（短时活动 vs 长期事实）。核心文件禁止叫 `STM.MD`、`MEMORY.MD`、`STMEM.MD`。

---

## 2. 七份权威

正文格式只有 Markdown。全大写文件名。一个对象一份文件。禁止 JSON 当权威正文。

| 文件 | 写什么 | 首次引导 | Frozen Core | 进 Prompt |
| --- | --- | --- | --- | --- |
| `IDENTITY.MD` | Agent 是谁、性格、**当前身体/形态** | 是 | 200 字 | 永冻投影 |
| `RELATIONSHIP.MD` | 用户是谁、这段关系、对关系的看法 | 是 | 120 | 永冻投影 |
| `REDLINE.MD` | 用户红线与必须先问什么 | 是 | 200 | 永冻投影 |
| `USER.MD` | 用户能自述的偏好 + Agent 对用户的观察 | 是（只收偏好） | 160 | 永冻投影 |
| `DREAM.MD` | Agent 自己的心愿 | **否** | **10 字** | 永冻投影 |
| `DARK.MD` | FEAR / SHADOW 两个展位 | **否** | 60 | 永冻投影 |
| `WORLD.MD` | 当前环境 | 是 | **不进** | **不装配；工具读写** |

字数：去掉首尾空白后，一个汉字或一个拉丁字母计 1。超限拒绝写入，禁止静默截断。

正文上限：Identity 800、Relationship 600、Redline 400、User 800、Dream 40（本体可略宽于 FC）、Dark 300、World 1000。

v1 种类闭集。架构按登记表可加第八种，用户不能自加。禁止再使用 Commitment / Preferences 当权威名。

根目录旧 `USER.md`（retire 源）**不是** `USER.MD`。

---

## 3. 进上下文的三条车道

禁止再问「这份文件全注入还是全工具」。先归车道。

| 车道 | 规则 | 现在谁在上面 |
| --- | --- | --- |
| **永冻装配（Frozen Core）** | 会话开始截取投影，本会话不变 | Identity 200、Relationship 120、Redline 200、User 160、Dream 10、Dark 60 |
| **动态加载** | 不冻，可刷新的有界投影 | **空。** WORLD / ACTMEM 都不塞回来 |
| **工具增删改查** | 默认不进 Prompt | **WORLD 全文**、**整份 ACTMEM**、超出 FC 的正文、完整历史、BML、报告 |

`MEMRULES.MD` 不是这七份，不占这三条车道。日常不进全文；常驻最多几行「写记忆先读手册」
指针；**写记忆时才注入全文**（S9）。不得为手册另开第四条 Laputa 车道。

完整历史永不整包注入。禁止把 WORLD 再接回 `WorldStore::project()` 当默认上下文。

**ACTMEM 整份走工具，正文不装配。** CORE 日常只挂一个查询 `actmem`。改/整理类管理工具要做，但必须进 **DEFERRED**（`tool_search` 才挂上），沿用已有延迟工具集，不进稳定前缀。

子代理不进 Laputa 人格/ACTMEM/BML 生态。子代理上下文以后由主 Agent 装配（面具仍进）。本阶段不设计。

---

## 4. 谁写

| 动作 | 路径 |
| --- | --- |
| 用户改 IDENTITY / RELATIONSHIP / REDLINE / USER 偏好 / WORLD | 直接保存 |
| Agent 改 IDENTITY / RELATIONSHIP / REDLINE / WORLD / USER 偏好 | Persona **内容审查（P5）**，不是 Approval Center，不是 Governance |
| Agent 写 DREAM / DARK / USER 观察 | **P16 直写**当前头；用户可改可删 |
| AutoDream 改允许的人格 | **只提案，不直写、不 apply**（P19） |
| AutoDream 整理 ACTMEM | 直写，不走提案 |
| Memory CRUD（BML） | 直写，不审批 |
| 用户改 ACTMEM | 直写，不审批 |

P19 允许 AutoDream 提案：IDENTITY、RELATIONSHIP、USER 观察、DARK、WORLD 新 claim。  
禁止：REDLINE、DREAM、USER 偏好、BML/`MemoryPatch`、`memory_md`、Skill、种类表、自己 apply。

旧工具 `laputa_propose_section_write` 与 JSON `IdentityPatch` / `LearningNote` / `MemoryPatch` 合同删除。当前代码里这些东西还在，是**旧实现未通过**，不是产品。

---

## 5. 首次引导与历史

- 仅当 `IDENTITY`、`RELATIONSHIP`、`REDLINE`、`USER`、`WORLD` **全部不存在**时出现引导。
- 一次原子直写这五份及首批历史。不创建 Proposal / Approval。不创建 `DREAM.MD`。
- 五份都在之后，引导永久不再出现。部分存在、空、损坏走修复。`DREAM`/`DARK` 缺席不是 incomplete。
- 每次真实变化追加不可变完整快照 + 文本 Diff。不自动裁剪。历史不进 Prompt。
- Frozen Core 本会话冻结；当前文件改了，下一会话再生效。

Persona 工作区：左侧七份导航 + 一个中央区（当前文档 | 待审变更 | 历史）。无永久右栏。无通用 Governance UI。

---

## 6. ACTMEM（STM 概念的文件）

- 文件名：`ACTMEM.MD`（activity memory）。
- 就是**一个** Markdown 文件，不是库。
- 全局一份：`{config_dir}/actmem/ACTMEM.MD`，不进某个项目的 `.laputa/`，不进 BML sqlite。
- 正文三节：`## Pulse`（用户短原话）+ `## Recap`（每轮完成态）+ `## Work`（活动集）。胶囊另存。
- 用户一发言写 Pulse。助手本轮一结束立刻写 Recap（学 Grok recap，不等 10 分钟）。
- 空闲 10 分钟把该 session 的 Pulse+Recap **折进胶囊并从头删掉**，不是这时才第一次归纳。
- 预算 Pulse 1600 / Recap 1600 / Work 1600 / 单条 Recap 200 / 单胶囊 800。
- 第一版不做 STM→BML 自动晋升。
- **不自动装配正文。** CORE 只有一个查询工具；管理工具 DEFER。
- 旧 Garden `MEMORY.MD` 全舍弃。

产品 STM ≠ SessionCheckpoint ≠ CanonicalCheckpoint ≠ BML。

---

## 6.5 MEMRULES（记忆写入手册）

- 文件：`{config_dir}/memory/MEMRULES.MD`。缺则用内置 R1–R7。
- **不是** Laputa / 人格 / WORLD 姊妹文件。Persona 左栏不挂它（P21）。
- 人在 Memory 设置窗口改；v1 只给人改，Agent 不改手册。
- 上下文对齐 GA L0：日常不进全文；常驻几行指针；写记忆时才塞全文。系统自动写 Pulse / Recap / 折叠胶囊不为此塞全文。
- 不要和 GA L1 `[RULES]`（操作避坑）并成一份。

---

## 7. BML

typed SQLite + FTS5 是普通长期记忆唯一生产权威。文件名 `memory.sqlite3`，**跟人格走**（整机一份家）。工作区 `.laputa/memory.sqlite3` 是旧落点。  
`memory_md` / `MemoryMd` / `MEMORY.md` 长期记忆链路 **clean-break 删除**，不自动导入。  
Memory CRUD 不走审批。STM/ACTMEM 清理不得删 BML；反之亦然。

---

## 8. Evolution 与 AutoDream

**Diva 特色路线（D6，已决策）：** AutoDream **整理** ACTMEM，并 **诞生进化提案**；
人审接受后的提炼结果是 **SOP / Skill 文件**。不可自己 apply。  
旧 AutoDream → Memory/人格混 Governance Inbox **仍退役**。这不是救旧链，是自有叙事。  
不再以 GA 为进化参考（论文重点是密度）。后面可能自己写论文。

人格提案仍走 P5，不进 Evolution。BML 不走提案。  
SOP 与 Skill 的文件形态由 D3 设计。

2026-08-14 独立测试：现有 crate 测的是旧 `MemoryPatch` 合同。新路线未接线。

---

## 9. 代码现状（避免把产品当成已落地）

今天仓库里仍然大量存在：`.laputa/sections/*.json`、`Commitment`/`Preferences` 符号、`MemoryPatch`、AutoDream 读 Identity JSON、WORLD `project()` 未当生产装配（与「WORLD 走工具」碰巧一致）、产品 STM/ACTMEM 对象为零。

这些是 **R0 事实**，不是许可继续做旧模型。实施必须等 Research Gate + 保护分支 + clean break，不得双轨兼容。

---

## 10. 仍开放（不要假装已冻）

- Research Gate：**分域。** R0/R2/R3/R4 过。R1 不再跟 GA 做进化；D6 已冻 Diva 路线。D3 设计这条路线，不抄 GA。保护分支：**用户叫切再切**。
- （ACTMEM 车道已冻：全部工具；第一版一个读工具。）
- （MEMRULES 已冻 S9/P21：不进 Laputa；GA 式按需注入。）
- 七份人格目录：D1 设计稿写死 `{config_dir}/persona/`（待用户评 D1）。
- Evolution：SOP 与 Skill 的关系。
- STM→BML 晋升（明确第一版不做）。
- 子代理如何装配上下文（明确本阶段不做）。
- D0–D4 实现设计、删除切片。保护分支：文档收完后切备份，不追旧 SHA。
- D0 设计稿：[`../../research/cognitive-d0-domain-authority-2026-08/domain-authority.md`](../../research/cognitive-d0-domain-authority-2026-08/domain-authority.md)。**A/B/C 已拍**（P22 整机一份伴侣、BML 跟人格、蒸馏一律人审）。其余待整体点头。
- D1：**已批准** [`../../research/cognitive-d1-persona-workspace-2026-08/persona-architecture.md`](../../research/cognitive-d1-persona-workspace-2026-08/persona-architecture.md)。
- D2：**已批准** [`../../research/cognitive-d2-memory-stm-2026-08/memory-architecture.md`](../../research/cognitive-d2-memory-stm-2026-08/memory-architecture.md)。
- D3：**已批准** [`../../research/cognitive-d3-evolution-skill-2026-08/evolution-architecture.md`](../../research/cognitive-d3-evolution-skill-2026-08/evolution-architecture.md)。
- D4：**已批准** [`../../research/cognitive-d4-clean-break-delivery-2026-08/delivery.md`](../../research/cognitive-d4-clean-break-delivery-2026-08/delivery.md)。保护分支你说切再切。

---

## 11. 四个用户工作区

| 工作区 | 只做什么 |
| --- | --- |
| Persona | 七份权威、内容审查、历史 |
| Memory | BML 列表 + ACTMEM/STM 入口 + MEMRULES 设置 |
| Evolution | Skill |
| Chat Approval Center | 危险工具授权 |

Persona 不做 Memory/STM/Skill/安全审批。Approval 不做人格审查。
