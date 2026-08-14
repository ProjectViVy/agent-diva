# D0 — 总体认知领域与权威图

- 状态：`Design Draft / A-B-C Frozen / Remainder Awaiting Review`
- 日期：2026-08-14
- 性质：跨域 ADR。把已冻产品收成一张可检查的权威图，让 D1/D2/D3 不能各自发明第二权威。
- **不是** Architecture Gate，**不授权**改生产认知主链，**不切**保护分支。

产品原文仍以这些记录为准，本包不得重开：

- Persona P1–P21
- STM/Memory S1–S9
- Evolution D1–D6
- [Laputa 汇总](../../architecture/laputa/architecture.md)

**A / B / C 已拍（2026-08-14）。** 整机一份伴侣（P22）；BML 跟人格走（S1 修订）；
`memory_distill` 一律 Evolution 人审（D7）。  
**2026-08-15：** §3 Evolution 用户行补「直存已有 Skill」（D3 评审）。其余 D0 仍不是 Architecture Gate。

---

## 0. 本包做什么 / 不做什么

**做：** 一域一权威；scope；生命周期隔离；跨域写入者总表；Prompt 投影总表；禁止依赖；消解 R0 §8 八处重叠。

**不做（留给后面的 D 包）：**

| 留给 | 不在 D0 定 |
| --- | --- |
| D1 | 七文件绝对路径字符串、revision/Diff/CAS、CM6、WorldGovernance 去留细节、Persona HTTP DTO |
| D2 | Pulse/Work/胶囊字段、合并算法、SessionCheckpoint 改名实现、R1–R7 措辞微调、C1 hash 预算算法 |
| D3 | SOP 与 Skill 是一份还是两份、Evolution GUI、SelfEvolution cron、报告是否独立资产 |
| D4 | 删除切片顺序、保护分支 SHA、发布说明 |
任何 D 包都**禁止**做：双读、双写、启动导入、runtime fallback。这不是延期项。

---

## 1. 调研结论（先看代码，再画图）

今日产品已经拆成四个工作区。今日代码还是**一条混域脊柱**：

```text
Persona JSON + memory_md + BML 人格 kind + AutoDream 候选
        → EvolutionProposal / ProposalType::target_section()
        → MemoryGovernanceCoordinator
        → governance.db + governance.sqlite3
        → apply 时 TypedMemoryStore::put_governed 或写 section JSON
```

源码事实（2026-08-14 复核）：

- `ProposalType` 把 MemoryPatch、IdentityPatch、SopCreate、LearningNote 收进同一枚举；`SopCreate` 的 target 是 **Identity**。
- AutoDream 生产路径默认反射 `MemoryPatch`，经 `LaputaService::create_proposal` 进通用提案箱。crate 内 **零** ACTMEM 符号。
- Memory GUI 删除只返回 `proposal_id` 并打开审批。Persona 用户保存走 `create_user_edit_proposal` + governance submit。
- Frozen Core 仍捕获旧四节 JSON（Identity/Relationship/Commitment/Preferences）。
- MEMRULES 仍在 `.laputa/cognitive/`，挂 Persona 左栏；治理读 R1 拦无证据提案。
- `bml-boundary-check` 拦的是 **Laputa 治理模块直调** BML 写 API。Manager apply 仍从治理缝 `put_governed`。
- 旁路各自活着：`skills/*/SKILL.md`、`WORLD.MD`、M3 Approval、C1–C5。

旧桌面失败（`[object Object]`、Evolution 加载失败、`approval request not found`）的根因就是这条脊柱，不是某一处 stringify。D0 的工作是把脊柱拆开，不是给它打补丁。

---

## 2. 领域卡

绝对路径字符串仍由 D1/D2 写进 deletion-proof。本条只冻 **scope 轴**。

### 2.0 Scope 轴（三层）

| 轴 | 含义 | 住在这 |
| --- | --- | --- |
| **Session** | 一次聊天 | SessionCheckpoint、`canonical_checkpoint_v1`、transcript |
| **整机一份家** | 一台机器上的 agent-diva 只有一个伴侣 | 七份人格、BML `memory.sqlite3`、ACTMEM、MEMRULES。绝对路径 D1/D2 写死，须同一父家 |
| **工作区（不跟家走）** | 某个 git checkout 的运行时安全 | `governance.db`（危险工具）。**不**因 P22/S1 搬家 |

**已冻哲学（P22 / D0-A）：** 与其跟多个 agent 卿卿我我，不如更好对待当前这个伙伴。
禁止第二套人格 / 多 profile 约会。这和 AGENT-VIVY 式大型协作是另一条路，不要预留。

**已冻（D0-B / S1 修订）：** BML 跟人格走，整机共用一套 LTM。工作区 `.laputa/memory.sqlite3` 是旧落点。

Skill 落点：**轴跟这份家**（能力属于这个伴侣，不是属于某个 git 仓库）。文件形态仍 D3。今日 `workspace/skills/` 是旧实现。

P18 种类登记表：**全仓只有一张人格种类表**，所有权在 Persona，D1 设计存放与校验。D2/D3 禁止另建平行「权威花名册」。MEMRULES / ACTMEM / BML / Skill 都不是第八种人格。

### 2.1 Persona / WORLD

| 项 | 合同 |
| --- | --- |
| 权威 | 同目录七份 Markdown：`IDENTITY.MD`（含身体）/ `RELATIONSHIP.MD` / `REDLINE.MD` / `USER.MD` / `DREAM.MD` / `DARK.MD` / `WORLD.MD` |
| Scope | 整机一份家（P22）。禁止 per-git、禁止第二套伴侣。WORLD 不得另放子目录 |
| 不是 | Memory、ACTMEM、MEMRULES、Skill、审批、JSON section |
| 写入 | 见 §3。用户直存；Agent 改 IDENTITY/RELATIONSHIP/REDLINE/WORLD/USER 偏好 **必须 P5**（读 WORLD 走工具，写 WORLD 不是工具豁免）；DREAM/DARK/USER 观察走 P16 直写；AutoDream 只按 P19 提案、不可 apply |
| 建家 | D1 可建人格目录。**不得**预创建 BML / ACTMEM / MEMRULES / 空壳 WORLD |
| 历史 | 每次真实变化追加不可变完整快照 + 文本 Diff；不自动裁剪；不整包进 Prompt |
| 投影 | Frozen Core 按 P14/P20。`WORLD.MD` **不进** FC、**不**动态装配，工具读写 |
| 删除面 | `.laputa/sections/*.json`、`content: Value`、JSON 编辑器、`null` 种子、空壳 `# WORLD`、`persona-retire`、Persona 右栏治理、左栏 `memory_md` / MEMRULES |

### 2.2 BML（LTM）

| 项 | 合同 |
| --- | --- |
| 权威 | typed SQLite + FTS5，文件名 `memory.sqlite3`。普通长期记忆**唯一**生产权威 |
| Scope | **跟人格走**（D0-B）。与七文件同一套整机家。工作区 `.laputa/memory.sqlite3` 删除为产品落点 |
| 不是 | 人格正文、ACTMEM、SessionCheckpoint 的产品名、Skill |
| 写入 | CRUD 直写。不建 Proposal，不进 Evolution，不进 Approval，不进 Governance Ledger |
| 历史 | record revision + 软删 / 墓碑。误操作靠精确目标、历史、撤销 |
| 投影 | 稳定前缀只许 **Applied 长期记忆的有界 L1 索引**。禁止把人格 kind、MemoryMd、WorkingMemory、全文记录装进 L1 |
| 删除面 | `memory_md` 全链；`LaputaMemoryProvider` 五段 JSON 权威块；Memory 走 `MemoryGovernanceCoordinator` |

`MemoryRecordKind::{Identity,Relationship,Commitment,Preference}` **不得再当人格/FC/L1 权威**。枚举留墓碑还是迁 `LongTerm` 由 D2 定；在那之前这些行也**不得投影**。

### 2.3 ACTMEM（STM 概念的文件）

| 项 | 合同 |
| --- | --- |
| 权威 | `{config_dir}/actmem/ACTMEM.MD` 一份 Markdown |
| Scope | 整机一份家，与人格/BML 同套。不进某个 git 的 `.laputa/`，不进 sqlite，不是七文件 |
| 不是 | BML、transcript、`canonical_checkpoint_v1`、SessionCheckpoint |
| 写入 | 系统写 Pulse（发言）+ Recap（本轮助手结束立刻）+ 空闲折叠胶囊；聊天 Agent 与 AutoDream **直写整理 Work**；用户 Memory 页直改。都不审批 |
| 历史 | 胶囊另存。具体 schema D2。v1 不自动晋升 BML / Skill |
| 投影 | **整份走工具**。CORE 只常驻查询 `actmem`。管理工具 DEFER。不进 FC，不动态装配 |

禁止核心文件名 `STM.MD` / `STMEM.MD` / `MEMORY.MD`。

### 2.4 SessionCheckpoint 与会话压缩

| 项 | 合同 |
| --- | --- |
| 现行实现对象 | `MemoryRecordKind::WorkingMemory` + `update_working_checkpoint`。产品名 **SessionCheckpoint**，符号 D2 改 |
| 权威（压缩） | `canonical_checkpoint_v1`（C1–C5 现合同，D0 不重开） |
| Scope | 单 session。结束可物理删 SessionCheckpoint |
| 不是 | ACTMEM。禁止一个 `working_memory` 枚举表达两种寿命 |
| 投影 | SessionCheckpoint 可进 TurnVolatile。CanonicalCheckpoint 仍是独立第二条 system。二者都不是 STM |

会话结束清 SessionCheckpoint **不得**删 ACTMEM / BML / 人格 / Skill。ACTMEM 清理不得删另外三样。

### 2.5 MEMRULES

| 项 | 合同 |
| --- | --- |
| 权威 | `{config_dir}/memory/MEMRULES.MD`；缺则内置 R1–R7 |
| Scope | 整机一份家，与人格/BML/ACTMEM 同套。不进 Laputa，不进 sqlite |
| 不是 | 人格种类（禁止用 P18 加第八种）、WORLD 姊妹文件、操作避坑 L1 `[RULES]` |
| 写入 | v1 **只给人**在 Memory 设置改。Agent / AutoDream / Skill 不得 patch |
| 投影 | 日常不进全文；常驻最多几行指针；**写记忆时**才注全文。自动写 Pulse / Recap / 折叠胶囊不为此塞全文 |

### 2.6 Evolution / Skill

| 项 | 合同 |
| --- | --- |
| 权威（落地后） | `skills/<name>/SKILL.md` + `SkillsLoader`。SOP 与 Skill 的文件关系 **D3** 定，但提炼结果就是这类文件（D6） |
| Scope | **跟这份家**（伴侣的能力，不是仓库的能力）。文件形态 D3。今日 `workspace/skills/` 旧 |
| 不是 | Persona 沉淀、BML、旧 Governance Inbox、`SopCreate→Identity` |
| 写入 | AutoDream **只诞生提案**，不可 apply、不可静默直写 Skill（D6）。Settings Skills 是安装投影，不是第二权威 |
| `memory_distill` | **已冻 D7 / D0-C：一律 Evolution 人审。** 取消新建静默。不是 Chat Approval。S9 只负责灌手册全文 |
| 投影 | 现 C1 Skills 段保留。D3 设计时用密度尺子：巩固是为了减负，不是把 SOP 全文塞进稳定前缀 |

AutoDream 运行器、报告、Notebook **不是** LTM / 人格 / ACTMEM，**不进** Prompt。去留 D3/D4。

### 2.7 Chat Approval Center

| 项 | 合同 |
| --- | --- |
| 权威 | 危险工具运行时授权。账本 `governance.db` **只服务这一域**。位置留在工作区，**不**跟人格家走 |
| 不是 | Memory CRUD、Persona 审查、STM 维护、Evolution 人审、首次引导 |
| 删除面 | Approval `domain=memory`、`MemoryApply`、Persona 右栏通用治理、把 Evolution inbox 当 Memory/Persona 入口 |
| `governance.sqlite3` | **DELETE**（Memory 提案映射双账本） |

Persona 内容审查、Evolution 能力提案、工具审批是**三套状态机**。禁止共用 `EvolutionProposal`、`MemoryGovernanceCoordinator`、任一治理账本、同一套事件。

---

## 3. 跨域写入者总表

D1/D2/D3 不得各自发明 AutoDream 权限。

| 谁 \ 写什么 | 七份人格 | WORLD | BML | ACTMEM | MEMRULES | Skill 文件 | 工具审批 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 用户 Persona 页 | 直存（P16 直写例外见下） | 直存 | — | — | — | — | — |
| 用户 Memory 页 | — | — | 直写 CRUD | 直改 | 直改 | — | — |
| 用户 Evolution 页 | — | — | — | — | — | 直存**已有** Skill；接受/拒绝提案 | — |
| 聊天 Agent | 审查集 → P5；DREAM/DARK/USER 观察 → P16 | **读走工具；写走 P5**（用户直存除外） | 直写 CRUD | 日常维护直写 | 禁 | 一律 Evolution 人审（D7） | 危险工具走 Approval |
| AutoDream | **只按 P19 提案**，不可直写、不可 apply | 只提案新 claim；不覆盖 `confirmed+source=user` | **禁**（禁 MemoryPatch） | **必须整理直写** | 禁 | **只诞生提案**，不可 apply | 不走 |
| 系统（发言 / 本轮结束 / 空闲） | — | — | — | Pulse / Recap / 折叠胶囊 | — | — | — |
| 首次引导 | 五份原子直写 | 同左 | — | — | — | — | — |

P19 禁写：REDLINE、DREAM、USER 偏好、BML、`memory_md`、种类表、自己 apply。

---

## 4. Prompt / 上下文投影

P20 三条车道闭集。不得为手册或 ACTMEM 另开第四条 Laputa 车道。C1–C5 现合同不重开；只锁权威，不改装配算法。

| 进 Prompt 的东西 | 车道 | 来源 |
| --- | --- | --- |
| Mask | 稳定前缀 | 面具运行时，**不是** `IDENTITY.MD` |
| Frozen Core 投影 | 永冻 | 七份里的 FC 段（P14 字数）。捕获 Markdown，不是 JSON |
| Skills | 稳定前缀 | `SkillsLoader`（密度：D3 不得再加肥） |
| BML L1 索引 | 稳定前缀（C1 已有槽） | 仅 Applied 长期记忆存在性索引。P20「BML 走工具」= **全文 / CRUD**，不是拆掉 L1 槽。D2 只做预算与过滤，不得把 L1 改成第二套人格 |
| 常驻 MEMRULES 指针 | 稳定前缀片段 | 最多几行「写记忆先读手册」。不是全文，不是人格文件 |
| CORE `actmem` schema | 工具 schema | 不是 ACTMEM 正文 |
| `canonical_checkpoint_v1` | 独立第二条 system | 本会话压缩 |
| SessionCheckpoint | TurnVolatile | 有块才进 |
| PrefetchRecall | TurnVolatile | 有界 |

**默认不进：** WORLD 全文、ACTMEM 正文、MEMRULES 全文、完整历史、BML 记录全文、报告、退役人格、子代理的这套生态。

**写记忆时刻才进：** MEMRULES 全文（S9）。

禁止再接线 `WorldStore::project()` 当默认上下文。动态加载车道保持 **空**。

---

## 5. 禁止依赖（D4 必须能扫）

每条写成「A 不得 B」。今日代码违规的，标成删除面，不是过渡兼容。

1. `EvolutionProposal` / `ProposalType::target_section` **不得**再当 Persona、Memory、STM、Skill 的共用信封。
2. `MemoryGovernanceCoordinator` **不得**出现在 Memory / Persona / STM 生命周期。
3. Governance / Approval 模块 **不得**写 BML（现成 `bml-boundary-check` 保留；Manager apply 缝一并拆掉）。
4. Memory CRUD / ACTMEM 维护 / Persona 用户直存 / 首次引导 **不得**创建 Approval 或 Governance Ledger 记录。
5. AutoDream **不得**创建 `MemoryPatch`，**不得**写 BML，**不得** apply 自己的提案，**不得**把 SOP 写进 Identity。
6. AutoDream **不得**直写七份人格；只许按 **P19** 提案（禁 REDLINE / DREAM / USER 偏好）。也**不得**不整理 ACTMEM。
7. `laputa_propose_section_write` **删除**。聊天改审查集走 P5，不走旧工具。
8. Frozen Core / Persona 正文 **不得**是 `serde_json::Value`。
9. `memory_md` / `MemoryMd` / `MemoryPatch→MemoryMd` **删除**，不得双读 BML。
10. BML 人格 kind **不得**进入 FC、L1、Persona GUI、AutoDream 人格输入。
11. SessionCheckpoint **不得**在 GUI 或 Prompt 里叫 STM；**不得**与 ACTMEM 共寿命。
12. MEMRULES **不得**进 Laputa 目录、Persona 左栏、种类表；Agent **不得** patch。
13. WORLD / ACTMEM **不得**重回 Frozen Core 或动态加载。
14. 子代理 **不得**读或写人格 / ACTMEM / BML。本阶段不设计装配。
15. Evolution Inbox **不得**列出或 apply 人格 / Memory 提案。
16. AutoDream 与 `memory_distill` **都不得**静默写 Skill。蒸馏一律 Evolution 人审（D7），不是 Approval Center。
17. 任何 D 包 **不得**以双读、启动导入、fallback 绕过本图。
18. 清某一权威 **不得**级联删除另一权威。

---

## 6. R0 §8 八处重叠：D0 判法

| # | 重叠 | 判法 |
| --- | --- | --- |
| 1 | Persona JSON vs BML 人格 kind | 人格权威 = 七份 MD。BML 人格 kind 失去权威资格（§2.2） |
| 2 | `memory_md` vs BML LongTerm | `memory_md` DELETE。BML LongTerm 唯一 |
| 3 | WorkingMemory vs STM vs canonical | 三分：SessionCheckpoint / ACTMEM / `canonical_checkpoint_v1` |
| 4 | `governance.db` vs `governance.sqlite3` | 前者 KEEP 仅危险工具；后者 DELETE |
| 5 | FC vs Typed L1 vs `LaputaMemoryProvider` | FC = MD 投影；L1 = 仅 BML 索引；文件五段 JSON DELETE |
| 6 | WORLD MD vs WorldGovernance vs `project()` | 权威 = `WORLD.MD`。`project()` 不作默认装配。队列去留 D1 |
| 7 | Skill 树 vs distill vs Evolution vs Settings | 权威 = `SKILL.md`。写入一律 Evolution 人审（D6+D7）。scope 跟整机家；形态 D3 |
| 8 | Approval memory 域 vs Evolution inbox vs Persona 右栏 | 对 Persona/Memory 全部 DELETE。三套状态机分账 |

已被 8/14 决策改判、不再 DECIDE 的：MEMRULES（S9/P21）、ACTMEM 物理与车道（S8）、WORLD 工具车道（P20）、AutoDream 进化路线（D6）。

STM 决策文里「物理权威待选」、R3「MEMRULES 仍 DECIDE」以 8/14 修订为准，**作废 Hold**。

---

## 7. 生命周期隔离

```text
会话开始
  捕获 Frozen Core（本会话不变）
  不装配 ACTMEM / WORLD / MEMRULES 全文

用户发言
  系统写 Pulse
  模型要查近事 → actmem 查询工具

本轮助手最终回复结束
  系统立刻写 Recap（一句完成态；不为此注入 MEMRULES）

空闲 10 分钟
  把该 session 的 Pulse+Recap 折进胶囊并从头删掉（不是这时才第一次归纳）

写长期记忆 / AutoDream 整理
  注入 MEMRULES 全文
  BML 直写 或 ACTMEM 直写
  人格变更只进 P5；能力变更只进 Evolution 提案

会话结束
  删 SessionCheckpoint
  保留 ACTMEM、BML、人格、Skill、胶囊

首次引导
  五份全不存在 → 原子直写 + 首批历史
  不建 Proposal / Approval
```

v1 禁止：STM→BML 自动晋升、STM→Skill 自动晋升、子代理进入这套生态。

---

## 8. 四个工作区（实现时不得串台）

| 工作区 | 只碰 |
| --- | --- |
| Persona | 七份、P5 审查、永久历史 |
| Memory | BML 列表/详情、ACTMEM 入口、MEMRULES 设置 |
| Evolution | Skill 提案与文件 |
| Chat Approval Center | 危险工具 |

Persona 不跑 AutoDream。Evolution 不审人格。Memory 不打开 Approval。Approval 不审文档。

---

## 9. 交给 D1–D4

- **D1** 按 §2.1 + P22 把七文件落到**整机一份家**。不得 per-workspace，不得做多伴侣入口，不得把 MEMRULES 放回左栏，不得接 `project()` 当默认上下文。建家目录不得预建空壳 WORLD；BML/ACTMEM/MEMRULES 文件由各域首次真实写入创建。
- **D2** 把 `memory.sqlite3` 放进同一套家（S1 修订）。SessionCheckpoint 行仍按 session 隔离（同一库、不同 session_id）。不得把 ACTMEM 塞进 C1 正文，不得把 BML 做成第二套人格。
- **D3** 按 D6 + D7 + §2.6 设计提案面和 SOP/Skill 文件。Skill 跟这份家。蒸馏一律人审。不抄 GA 进化。不把报告做成 LTM。
- **D4** 把 §5 变成扫描族和切片。用户叫切再切备份。

密度尺子（GA 对照笔记）：常驻只留存在性 + 人格永冻；巩固为减负。D0 不另开原则文档，D1–D3 对照即可。

---

## 10. D0 自己的验收

用户评审本包时看这几句是否成立：

1. 每个对象能指到**唯一**权威，指不出第二份。
2. AutoDream / 聊天 Agent / 用户 的写入权只有一张表（§3）。
3. 进 Prompt 的东西都能归进 P20 车道或 S9 手册政策（§4）。
4. R0 八处重叠都有判法（§6）。
5. 没有规定 D1–D4 才该定的路径字符串、Diff、Pulse schema、SOP 形状、删除顺序。

通过本包 ≠ 可以改生产代码。还要 D1–D4 评审 + 你叫切备份。

---

## 11. A / B / C 已拍（2026-08-14）

| 问 | 决定 | 落点 |
| --- | --- | --- |
| A | 整机 agent-diva **共用一份人格**。哲学：与其跟多个 agent 卿卿我我，不如更好对待当前伙伴。不是 Vivy 式多 agent 协作 | **P22** |
| B | BML **跟人格走**，共用一套。`governance.db` 不绑 | **S1 修订** |
| C | `memory_distill` **一律入审** = Evolution 人审，不是 Approval Center | **D7** |

WORLD 写权仍是 P16：读工具，写 P5。
