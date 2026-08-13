# 新 Laputa 认知分区调研与关键人格文件回迁提案

> **日期**：2026-08-07（用户已拍板 Q1–Q5 + 子决策 D1/D2，全部冻结；待实施排期）
> **状态**：PROPOSAL · 决策已冻结（仅提案，不含代码实施）
> **来源**：桌面 Garden 仓库（`C:\Users\Administrator\Desktop\garden`）
> **前置**：`docs/research/laputa-diva-garden-2026-07/`（7 月调研，早于 Garden 决策定稿）
> **范围约束**：不回迁 Mentle（GMH-24 clean-break 不回退）；不引入 garden Go 代码；
> 本提案只迁移**认知分区决策与关键新人格文件**的语义，物理实现沿用 agent-diva
> 现有 Rust/typed 基建。

---

## 1. Garden 侧新 Laputa 设计（已定稿决策链）

Garden 在 2026-08-02 ~ 08-03 连续接受三份 ADR，将旧 14-section 扁平注册表
重构为**认知分区（cognitive partition）**：

| ADR | 文件 | 核心内容 |
|-----|------|----------|
| ADR-0002 | `docs/architecture/0002-laputa-cognitive-partition-decision.md` | 认知分区决策：按认知角色分区，取代 14-section 扁平编号 |
| ADR-0004 | `docs/architecture/0004-cognitive-files-migration.md` | MEMRULES.MD / WORLD.MD 物理存储、schema、强制边界、Context Plane 测试矩阵 |
| ADR-0008 | `docs/architecture/0008-legacy-compatibility-removal.md` | Legacy 移除：注册表收敛到 8 section，删除 06/10/11/12/13/14 |

### 1.1 分区总览（ADR-0002 §2）

```text
A. Frozen Core Prompt（会话内冻结）
   01 identity / 02 relationship / 03 commitment / 04 preferences

B. Active Work
   05 MEMORY.MD —— STM 权威检查点（当前目标/开放回路/下一步/证据指针）

C. Cognitive Governance（新命名文件，初期人类操作）
   MEMRULES.MD —— 认知治理规则手册
   WORLD.MD    —— 可行动世界理解（claim 化世界模型）

D. Human-facing Report System（主要给人类读）
   07 daily / 08 weekly / 09 monthly
   10 AMBITION（仅月报注入）/ 11 USER SUGGESTIONS（仅月报注入）

E. Infrastructure（不是内容分区）
   append-only change/audit log；transient spool

F. Removed（概念删除）
   06 history_md / LTM / LONGMEM.MD
   13 report_indexes
   14 aaak_summaries
```

### 1.2 用户所说的三类关键决策

**被明确定位的文件：**

| 文件 | 定位 | 关键规则 |
|------|------|----------|
| 01–04 | Frozen Core Prompt | 会话启动读一次、会话内冻结；治理写入可发生但**下一会话才生效**；必须紧凑，不是用户档案 |
| 05 MEMORY.MD | STM 权威检查点 | 轻量投影修订；原始素材不在这里（在 Mentle/spool） |
| MEMRULES.MD | 认知治理规则手册 | **人类手工编辑 only**；无 HTTP 写端点、无 agent 写接口；**永不注入 ContextView**；缺失时用内置 7 条默认规则兜底 |
| WORLD.MD | 可行动世界模型 | claim 化（status/confidence/scope/source/updated + ≤280 字符正文）；用户可直编；AutoDream 可治理写入但**不得覆盖 user-confirmed claim**（只能标 stale）；**永不整体注入**，只按 scope+budget 投影切片 |

**被删除的文件：**

| 文件 | 删除理由 |
|------|----------|
| 06 history_md / LTM | Mentle 已是素材/证据唯一来源，第二份 LTM 造成双权威与状态腐化 |
| 13 report_indexes | 报告目录索引是报告子系统元数据，不是认知/权威 |
| 14 aaak_summaries | 其低成本摘要角色被 STM + 人类报告 + 检索 + ContextView 取代 |

ADR-0008 进一步**物理删除**了 06/10/11/12/13/14 的常量、注册表项、默认数据与
仓库内 JSON；写已删除 section 名返回 unknown section（HTTP 400，非 410）；
用户机器上的残留文件不主动删除，任其惰性存在。

**被明确为仅给人类看的文件：**

| 文件 | 规则 |
|------|------|
| 07 daily / 08 weekly / 09 monthly | 人类可读性报告；**非默认上下文**、非权威来源、非第四层记忆 |
| 10 AMBITION | agent 的开放性愿望；娱乐/报告模块，**仅月报回顾时注入**；不是任务清单/承诺/自我修改权威 |
| 11 USER SUGGESTIONS | 对用户的非约束性建议；**仅月报注入**；不是提案队列，是否变成工作由用户决定 |
| MEMRULES.MD | 人类编辑文档，Garden 只在 ingest/recall/认知写路径上强制执行，不进上下文 |

### 1.3 Context Plane（ADR-0002 §4 / ADR-0004 §6）

```text
Layer 0  Frozen Core（01–04），会话内冻结
Layer 1  STM bootstrap（05 MEMORY.MD + 当前工作集，有界）
Layer 2  Context Facade：任务/scope/budget 控制的证据 + WORLD 选择性投影

永不默认注入：MEMRULES.MD、完整 WORLD.MD、完整人类报告、原始素材、审计日志
```

ADR-0004 给出 8 行不变量测试矩阵（Frozen Core 在/STM 有界在/WORLD 仅
scope 切片在/MEMRULES 永不/完整 WORLD 永不/报告永不/Compat 数据永不/
证据有界）。

### 1.4 MEMRULES.MD 内置默认规则（ADR-0004 §2.2，实现见 `governance/cognitive/defaults.go`）

```text
R1 证据优先：原始素材与证据是首要真相来源
R2 断言区分：confirmed fact / observation / inference / hypothesis 必须区分
R3 矛盾处理：新矛盾证据不得静默覆盖既有理解
R4 用户权威：用户确认信息高于 agent 推断
R5 范围约束：scope/时间/置信度/出处/可见性约束使用
R6 WORLD 入门门槛：进入 WORLD 需与行动相关且是有界、可审查的 claim
R7 禁止整体注入：WORLD 不得整体拷贝进 agent 上下文
```

### 1.5 WORLD.MD claim schema（ADR-0004 §3.2）

```markdown
## [environment] Development machine
- status: confirmed        # confirmed/observed/inferred/hypothesis/stale
- confidence: high         # high/medium/low
- scope: dev, infra
- source: user             # user / autodream / 证据卡 ID
- updated: 2026-08-03T00:00:00Z

Windows 11, 64GB RAM, Go 1.26...（正文 ≤280 字符）
```

写策略：用户直编不受限；AutoDream 治理写入不得覆盖 `confirmed + source=user`
claim（只能标 `stale` + 追加备注，等用户审查）；普通 agent 无写接口。
投影 API 只返回 scope 匹配且预算内（默认 4000、上限 16000 字符）的 claim，
**永不返回完整文件**。

---

## 2. agent-diva 侧现状盘点

### 2.1 已核实的代码事实

| 现状 | 位置 | 说明 |
|------|------|------|
| 旧 14-section 扁平枚举仍在 | `agent-diva-core/src/evolution/types.rs:250` `LaputaSectionName` | 含 HistoryMd/JournalReflective/ProposalInbox/ReportIndexes/AaakSummaries |
| 14 个 section 文件 stem 全映射 | `agent-diva-laputa/src/layout.rs:167` `section_file_stem` | `.laputa/sections/*.json` |
| HistoryMd 仍可治理写入 | `agent-diva-laputa/src/service.rs:362` | `HistoryPatch` proposal，RiskLevel::**Low**——garden 已判死刑的概念在 diva 侧还是低风险可写目标 |
| JournalReflective/ReportIndexes/AaakSummaries/ProposalInbox 为 Tbd | `service.rs:960` `section_status` | 常量存在但未用 |
| 人格层 = SOUL.md + Mask + AGENTS.md + BOOTSTRAP.md | `agent-diva-agent/src/context.rs`、`mask/mod.rs` | SoulContextSettings（enabled/max_chars/bootstrap_once）；无会话冻结语义 |
| typed 记忆权威 | `.laputa/memory.sqlite3`（typed_store/typed_provider） | L0–L4、FTS5、MemoryRecordKind::Identity、evidence_refs（Wave 6）、supersedes/tombstone |
| 治理审批基建 | `.laputa/governance.sqlite3` + proposals/ | E0–E7 闭环：candidate → gate → PendingReview → apply → recall feedback |
| AutoDream | `agent-diva-autodream` | 真实 worker、有界脱敏输入、rejection suppression |
| MEMRULES / WORLD 概念 | **完全不存在**（全仓库 grep 0 命中） | — |
| Mentle | **已移除**（GMH-24 clean-break） | typed store 兼作长期记忆 |

### 2.2 与 garden 的关键结构差异

garden 是三层分工：**Mentle = 素材宇宙/证据湖；Laputa = 认知治理与稳定操作面；
Garden = 摄入、策略召回与一次性 ContextView 装配**。

agent-diva 因 GMH-24 移除了 Mentle，Laputa typed store 实际兼任
"长期记忆 + 治理权威"双重角色。这**不构成回迁 Mentle 的理由**（用户已明确
不回迁），但意味着 garden 的 R1「证据优先」在 diva 侧的映射对象是
**typed MemoryRecord 的 evidence_refs 证据链**，而非外部素材湖。

### 2.3 workspace 人格/记忆文件层现状清单（决策 Q3 影响面）

用户已拍板：**删除这一整层文件，人格与记忆完全使用 Laputa 治理**。
现状盘点（均已核实代码位置）：

| 文件 | 当前角色 | 现状机制 |
|------|----------|----------|
| `SOUL.md` | 人格核心 | soul_settings 控制注入；写入被监控，变更触发边界确认提示（`loop_turn.rs:793` soul-file watchlist） |
| `IDENTITY.md` / `USER.md` | 身份/用户档案 | 同属 soul-file watchlist，仅透明通知，无治理 |
| `BOOTSTRAP.md` | 一次性 onboarding | prompt 中声明“非运行时权威”（`context.rs:185`） |
| Mask（persona markdown） | 多人格切换 | 非默认 mask 整体注入 prompt 顶部（方案 A，`context.rs:99`） |
| `memory/MEMORY.md` | legacy 记忆读输入 | MemoryManager 读取；已声明“仅兼容/迁移输入” |
| `memory/HISTORY.md` | legacy 历史追加 | `manager.rs:36` 读取；**`consolidation.rs:408` 仍在主动 append**——即 garden 已删除的 06-history_md 概念在 diva 侧仍有活跃写路径 |
| `AGENTS.md` | 项目规则（非人格） | 注入为 Agent Rules（`context.rs:192`）——**不属于本次删除范围**，保留 |
| Daily/Weekly/Monthly 报告系统 | 记事本（Notebook）日/周/月报 | **完整存在且有人使用**（用户实测手动生成可用）：`core/reports/` 完整管线（session digest → fact bundle → LLM 策展叙事/确定性回退 → 校验 → 渲染）；AutoDream `notebook-daily/weekly/monthly` 触发器；产物存 `.agent-diva/autodream/reports/{daily,weekly,monthly}/*.md`；GUI NotebookView 有生成按钮与展示面；**节律自动触发未经真机测试** |

---

## 3. 差距对比（Gap Matrix）

| # | 维度 | Garden 新设计 | agent-diva 现状 | 差距定性 |
|---|------|---------------|-----------------|----------|
| G1 | Section 注册表 | 8-section 目标模型（01–05、07–09） | 14-section 扁平枚举 | **反向决策缺口**：diva 仍保留 garden 已删除的概念 |
| G2 | Frozen Core 语义 | 01–04 会话启动快照、会话内冻结、写入下一会话生效 | Identity/Relationship/Commitment 有 High-risk 治理，但**无会话冻结快照** | 语义缺失 |
| G3 | MEMRULES.MD | R1–R7 规则手册，人类 only，永不注入，缺失兜底 | 仅 `L0_MEMORY_POLICY` 三条硬编码文本（Wave 2） | **完全缺失**（新关键人格文件） |
| G4 | WORLD.MD | claim 化世界模型，scope/budget 投影，confirmed 保护 | 世界事实散落在 MemoryRecordKind::Identity 记录里，无结构化 claim、无投影纪律 | **完全缺失**（新关键人格文件） |
| G5 | 06/13/14 | 物理删除，写=unknown section | HistoryMd 仍为 Low-risk 可写 proposal 目标；13/14 Tbd 常量 | **决策回退风险** |
| G6 | 07–09 报告 | 人类可读定位、report_system 权威、永不默认注入 | 完整报告管线存在（记事本版块，用户实测手动生成可用）；但 agent 仍可提 Daily/Weekly/MonthlyPatch 提案写报告区；无「永不注入」不变量 | **边界未明确 + 注入纪律缺失**（非功能缺失） |
| G7 | 10/11 | 改名 AMBITION / USER SUGGESTIONS，仅月报注入，非约束 | ProposalInbox/JournalReflective Tbd 常量 | 未跟进 |
| G8 | proposal_inbox | 删除，EvoMap mailbox 另行设计 | proposals/ + governance ledger（**比 garden 更强的审批基建**） | **diva 优势项，保留不回退** |
| G9 | Context Plane 不变量 | 8 行测试矩阵（永不注入项显式断言） | startup 注入/prefetch 有预算，但无"永不注入"负向不变量矩阵 | 测试纪律缺口 |
| G10 | 审计 | Frozen Core/WORLD/MEMRULES 变更必审计；STM 日常写不审计 | governance ledger + audit sink 已覆盖 apply/rollback | 基本对齐，补 WORLD/MEMRULES 事件类型即可 |

---

## 4. 回迁提案（Proposal，不含代码）

### 4.1 目标

把 garden 新 Laputa 的**认知分区决策**与两个**关键新人格文件**
（MEMRULES.MD、WORLD.MD）的语义回迁到 agent-diva，同时收口 14→8
注册表，建立 Frozen Core 会话冻结语义与 Context Plane 负向不变量。

### 4.2 非目标（明确不做）

- ❌ **不回迁 Mentle**（用户明确指示；GMH-24 clean-break 不回退）。
  R1「证据优先」映射为 typed `MemoryRecord.evidence_refs` 证据链。
- ❌ 不引入 garden Go 代码 / FileStore JSON section 模式——diva 的 typed
  SQLite 权威是更强实现，认知文件只做 Markdown 文件层，不进 SQLite。
- ❌ 不回退 diva 现有 proposals/governance 审批基建（G8 优势项）。
- ❌ 不迁移 garden rhythm 报告引擎（**diva 已有自研完整报告管线**，见 S6；
  不回退为 garden Go 实现）与 EvoMap mailbox（另行独立决策）。
- ❌ 本提案不含任何代码变更；实施需单独走切片 + BMAD/story 流程。

### 4.3 切片规划（S1–S7，逐切片独立提交）

**S1：cognitive/ 目录与 MEMRULES.MD（新关键人格文件 #1）**

- 在 workspace `.laputa/cognitive/` 下建立 `MEMRULES.MD`（缺省种子 + 永不覆盖已存在文件，对齐 garden `InitializeDir` 语义）。
- R1–R7 七条默认规则回迁，并按 diva 语境改写映射：
  - R1：「原始素材与证据首要」→「typed MemoryRecord 的 evidence_refs 证据链首要；无 evidence 的记忆写入保持 advisory（Wave 6 S3 已有 evidence_advisory）」；
  - R4：对齐现有混合分级写入决策（低风险即时 / 高风险审批，inventory §10.1）；
  - R6/R7：为 S2 的 WORLD 入门门槛与禁止整体注入预留条款。
- 强制边界：**人类手工编辑 only**（无工具、无 API 写入口）；启动时读取并
  在记忆写路径（memory_add/update/remove/distill 路由与 AutoDream gate）
  上作为策略参考；**永不注入 system prompt / ContextView**。
- 缺失时内置默认规则兜底（与 garden 一致）。

**S2：WORLD.MD（新关键人格文件 #2）**

- 在 `.laputa/cognitive/WORLD.MD` 建立 claim 化世界模型，schema 与 garden
  完全对齐：`## [domain] title` + status/confidence/scope/source/updated
  元数据 + ≤280 字符正文；五态 status（confirmed/observed/inferred/
  hypothesis/stale）。
- 写路径（三条，权限递减；**用户已拍板 AutoDream 首切片即可写**，Q4）：
  1. 用户直编文件——不受限；
  2. AutoDream 治理写入——**首切片即开启**，复用现有 candidate → gate →
     PendingReview → apply 基建，新增 `WorldClaimUpsert` proposal 类型；
     **不得覆盖 `confirmed + source=user` claim**（只能标 stale + 追加备注），
     复用现有 superseded/tombstone 机制实现；
  3. 普通 agent 交互写——**不暴露**（与 garden 一致；agent 观察到的世界事实
     走现有 memory_add 路径，由 AutoDream/蒸馏晋升入 WORLD）。
- 读路径：接入现有 prefetch/recall 预算机制，按 scope + budget 投影切片
  （默认预算与上限对齐 garden 4000/16000 字符）；**永不整体注入**，
  永不进 startup index 全量渲染。
- 审计：WORLD 治理写入与 confirmed 保护触发均进 governance ledger。

**S3：Frozen Core 会话冻结语义（01–04 定位回迁）**

- Identity/Relationship/Commitment/Preferences 四区在**会话启动时快照**，
  会话内上下文装配一律使用启动快照；治理写入（现有 High-risk proposal 路径）
  成功后**下一会话生效**。
- 与 workspace 人格文件层的关系已由用户拍板（Q3）：**不并存、不合并讨论，
  直接退役整层文件**，其内容职责全部归 Frozen Core，具体退役与迁移见 S4。

**S4：workspace 人格文件层退役与内容迁移（用户拍板 Q3）**

删除对象（§2.3 清单）：`SOUL.md`、`IDENTITY.md`、`USER.md`、`BOOTSTRAP.md`、
`memory/MEMORY.md`、`memory/HISTORY.md`，以及 soul 配套机制（SoulStateStore、
SoulGovernanceSettings、soul-file watchlist 透明通知、`consolidation.rs` 的
HISTORY.md append 写路径）。`AGENTS.md`（项目规则）与 Skills 不属于人格权威，
**保留**。

执行约束（诚实/治理原则，不静默删数据）：

1. **内容先行治理迁移**：SOUL/IDENTITY/USER 既有内容 → 一次性迁移生成
   Frozen Core proposal（identity/relationship/commitment/preferences 分区），
   走现有审批路径，用户批准后 apply；MEMORY.md 存量内容 → distill 入 typed
   记忆（或标记只读存档）。
2. **文件处置**：迁移批准后，源文件移入 `.laputa/legacy/` 惰性存档并从
   上下文装配中除名（对齐 ADR-0008 §3 惰性保留），不物理粉碎。
3. **Mask 边界明确（D1 已拍板：Mask 保留，与 Laputa 互不影响）**：
   Mask 顾名思义是面具系统——“戴上了一个面具”，**不影响人格本体**，
   不随人格文件层退役。上下文装配逻辑明确为：
   - Frozen Core（01–04）= 真实人格本体，Layer 0，会话内冻结；
   - Mask = 会话/任务级**临时外在覆层**（现行方案 A 顶部注入保留），
     是“穿戴”而非“改写”；摘下面具本体人格不变；
   - 装配序：Mask 覆层 → Frozen Core 本体 → 其余层；Mask 不得解除
     03-commitment 红线（覆层优先级永远低于红线）；
   - Mask 文件不进 Laputa 治理（不是权威、不走 proposal），其变更也
     不属于 Frozen Core 变更；soul-file watchlist 退役时 Mask 相关
     监控单独保留或按产品需求另定。
4. **负向回归**：退役后 system prompt 装配不得再读取 SOUL/IDENTITY/
   USER/BOOTSTRAP/legacy MEMORY、HISTORY 文件；旧 watchlist/透明通知
   代码路径一并删除（归入 S5 硬删批次的同一清理）。

**S5：注册表收敛 14 → 8（硬删，用户拍板 Q2）**

- **一步硬删** `LaputaSectionName` 的 HistoryMd/JournalReflective/
  ProposalInbox/ReportIndexes/AaakSummaries 五个变体；layout/service/
  proposals/memory_records 映射同步收敛。
- 序列化兼容约束（硬删的实施红线）：既有 governance/proposal 历史记录中
  若存在已删变体的落盘值，反序列化必须走稳定的 unknown-variant 失败/忽略
  路径（只读历史不得因硬删而崩溃）；写入已删除 section 一律返回稳定失败码
  （unknown/unauthorized target，对齐 ADR-0008）。
- 不物理删除用户 `.laputa/sections/` 残留文件（惰性保留）。
- 07–09 报告区按 Q5=b 完全重构，见 S6（本切片只删枚举/映射，报告边界
  重构的其余部分归 S6）。

**S6：报告系统边界重构（用户拍板 Q5=b，完全重构）**

已验证的现状（修正早期误判）：报告管线完整且有人使用——`core/reports/`
（digest → fact bundle → LLM 策展/确定性回退 → validate → render）、AutoDream
`notebook-daily/weekly/monthly` 触发器、产物 `.agent-diva/autodream/reports/
{daily,weekly,monthly}/*.md`、GUI 记事本版块（手动生成用户实测可用，
节律自动触发未真机测试）。

目标边界（四句定义，用户拍板 + ADR-0002 D 分区）：

```text
1. 报告 ≠ 记忆：从记忆体系拆出，非权威、非第四层记忆、不进召回/FTS；
2. 永不注入：报告内容不得默认注入 agent 上下文（负向不变量）；
3. 可选读取：agent 可按需主动读报告（工具读取，取决于 agent 自己），
   系统不推送；
4. 生成权威 = report_system：只有手动/节律触发生成，agent 不得以
   proposal 改写报告。
```

实施子项：

1. **写权威收口**：删除 Daily/Weekly/MonthlyPatch proposal 目标与
   `MemoryRecordKind::Daily/Weekly/Monthly` 记忆映射（报告不再是可提
   案写入目标、不再是记忆 kind，写入权归 report_system，对齐 garden
   8-section 权威表）。
2. **注入禁令**：审计并移除报告/节律内容进入 prompt 的既有路径（含
   legacy `StartupContextSnapshot` 的 `## Rhythm Signals` 渲染段，随 S4
   soul 层退役一并清理）；S7 负向回归断言报告内容永不入 prompt。
3. **agent 可读入口**：报告为普通 markdown 文件，agent 用现有 read_file
   即可按需读；prompt 不主动提示、不索引（可选读 = agent 自决）。
4. **节律验证与收口**：真机测试 `notebook-daily/weekly/monthly` 自动
   触发链路（cron → AutoDream → 生成 → Notebook 可见）；未触发/失败
   走稳定原因码，不静默。
5. **产物归属（待确认 D2）**：报告现居 `.agent-diva/autodream/reports/`；
   认知分区后建议迁 `.laputa/reports/`（对齐 workspace 权威边界，Q1
   一致），需一次性只搬文件不改内容的迁移与回滚。

**S7：Context Plane 不变量矩阵（测试纪律回迁）**

- 移植 garden ADR-0004 §6 的 8 行矩阵为 diva 负向回归测试：
  MEMRULES 永不入 prompt；完整 WORLD 永不入 prompt；报告永不默认注入；
  Frozen Core 会话内不变；WORLD 投影只含 scope 匹配且预算内 claim；
  已删除 section 数据永不出现；已退役人格文件（SOUL.md 等）永不入 prompt。

### 4.4 建议执行顺序与依赖

```text
S1 (MEMRULES) ──┐
                ├──► S7 (不变量矩阵)
S2 (WORLD)   ───┤
S3 (Frozen)  ───┴─► S4 (人格文件层退役，依赖 S3 的 Frozen Core 就位)
S5 (硬删收敛) 与 S6 (报告边界重构) 同批：Daily/Weekly/MonthlyPatch 删除
   既是 S5 枚举收口的一部分，也是 S6 写权威收口的载体，两切片相邻提交
```

每个切片按仓库纪律：四件套文档（docs/logs）、单一 concern 提交、
`just ci` 通过、不 push。

### 4.5 与现有待办的关系

- 本提案即 TODOLIST Wave 6 延期项「**Laputa 设计同步（待调研/待决策）**」
  的调研产出；批准后该项从「待调研」转「已调研，待实施排期」。
- 与 GA-MEM-PARITY 决策冻结不冲突：混合分级写入、Typed 默认权威、
  consolidation 兜底均不变；MEMRULES 是其上层的规则手册层。
- 与「记忆检索增强（FTS5→语义）」延期项正交：WORLD 投影走独立读路径，
  不经过 FTS5。

---

## 5. 决策记录（2026-08-07 用户拍板）

| # | 决策 | 结果 |
|---|------|------|
| Q1 | 认知文件存放层级 | ✅ **workspace 级** `.laputa/cognitive/`（跟随 diva workspace-centric 模型） |
| Q2 | 枚举删除策略 | ✅ **一步硬删** 5 个变体；实施红线：落盘历史反序列化走 unknown-variant 稳定失败路径（见 S5） |
| Q3 | Frozen Core 与人格文件层关系 | ✅ **删除现有所有 SOUL.md/HISTORY.md 类文件，人格与记忆完全使用 Laputa 治理**；退役与内容迁移见 S4；`AGENTS.md`/Skills 保留 |
| Q4 | WORLD 写路径 | ✅ **AutoDream 首切片即可治理写入 + 人类可直编**；confirmed+user claim 保护不变 |
| Q5 | 报告区（07–09）收口范围 | ✅ **b 完全重构**（用户拍板 2026-08-07 追加）。边界四句定义见 S6：报告≠记忆、永不注入、agent 可选读取（自决）、生成权威=report_system。**更正**：本提案早期版本误判「无生成引擎/无展示面」——实际报告管线完整（core/reports + AutoDream rhythm + GUI 记事本），用户实测手动生成可用，节律自动触发待真机验证 |

**实施期子决策（已拍板）：**

| # | 子决策 | 结果 |
|---|--------|------|
| D1 | Mask 去留 | ✅ **保留，与 Laputa 互不影响**。Mask = 面具系统，“戴上面具”不影响人格本体；上下文装配：Frozen Core 为本体 Layer 0，Mask 为临时外在覆层（不得解除 commitment 红线，摘下后本体不变）；Mask 不进 Laputa 治理（见 S4-3） |
| D2 | 报告产物存储位置（S6-5） | ✅ **批准**：从 `.agent-diva/autodream/reports/` 迁 `.laputa/reports/`，只搬文件不改内容，可回滚 |

---

## 6. 附录：证据索引

| 事实 | 证据 |
|------|------|
| Garden ADR 决策链 | `garden/docs/architecture/0002-…`、`0004-…`、`0008-…`（均 Status: accepted） |
| MEMRULES/WORLD 实现契约 | `garden/laputa/governance/cognitive/{defaults,memrules,world}.go` + `cognitive_test.go` |
| Garden 注册表已收敛 8 section | `garden/laputa/AGENTS.md` §Purpose；`ARCHITECTURE.md` cognitive partition notice |
| diva 侧 14-section 枚举 | `agent-diva-core/src/evolution/types.rs:250` |
| diva 侧 section 文件布局 | `agent-diva-laputa/src/layout.rs:167-184` |
| diva 侧 HistoryMd 仍可写 | `agent-diva-laputa/src/service.rs:355-394` |
| diva 侧无 MEMRULES/WORLD | 全仓库 `grep -i 'WORLD\.MD\|MEMRULES\|WorldClaim'` = 0 命中 |
| diva 侧人格层 | `agent-diva-agent/src/context.rs`（SoulContextSettings）、`mask/mod.rs` |
| diva 侧报告管线（修正误判） | `agent-diva-core/src/reports/mod.rs`；`agent-diva-autodream/src/{rhythm,monthly,layout}.rs`（notebook-daily/weekly/monthly 触发器、`reports/{daily,weekly,monthly}/*.md`）；`agent-diva-manager/src/runtime.rs:438`（LLM curation）；GUI `NotebookView.vue` + locales（生成日报/周报/月报） |
