# agent-diva 综合记忆系统预设计

## 1. 设计定位

这份设计不是单纯讨论“RAG 要不要加向量检索”，而是把 `agent-diva` 的未来核心重新定义为：

> 一个以记忆系统为中心、由 soul 驱动、能持续形成自我与对用户长期理解的 agent。

在这个定位下，记忆系统不只是给模型补上下文，而是承担五个核心职责：

1. 持续保存用户、任务、工作区、关系与自我变化。
2. 让 agent 在每一轮都能以低 token 成本召回真正相关的过去。
3. 为 soul 演化提供事实依据，而不是让人格漂移全靠即时对话。
4. 支撑“日记”机制，让 agent 形成主观连续性。
5. 让理性与感性两个层面既分离又协同，形成 agent-diva 自己的差异化。

## 2. 本设计基于哪些既有事实

### 当前 agent-diva 已有基础

- `agent-diva-agent/src/consolidation.rs`
  - 已有 conversation -> memory consolidation。
- `agent-diva-core/src/memory/manager.rs`
  - 已有 `MEMORY.md` / `HISTORY.md` / 每日日志文件。
- `agent-diva-agent/src/context.rs`
  - 已有 soul 文件与 memory 的 prompt 注入链。
- `agent-diva-core/src/soul/mod.rs`
  - 已有 bootstrap/soul 生命周期状态。

### 仓库内既有设计参考

- `docs/dev/archive/architecture-reports/soul-mechanism-analysis.md`
  - soul 的连续性、透明演化、身份形成机制。
- `docs/dev/archive/architecture-reports/zeroclaw-style-memory-architecture-for-agent-diva.md`
  - memory store / memory loader / query-driven recall 思路。
- `dev/docs/2026-03-26-agent-diva-rag-research.md`
  - `openclaw` / `zeroclaw` / `nanobot` 的对比结论。

### 外部参照从本地源码提炼出的结论

- `openclaw`
  - 工具化检索链最成熟，适合借鉴 manager + tool + prompt policy。
- `zeroclaw`
  - Rust 侧 memory trait、多 backend、专用 RAG、知识图谱思路最值得借鉴。
- `nanobot`
  - consolidation 与文件记忆的轻量路径可作为低复杂度兜底。

## 3. 总体设计原则

## 3.1 记忆优先于对话历史

未来 `agent-diva` 的连续性不应该主要来自“保留最近 50 条历史”，而应该主要来自：

- 稳定记忆
- 被检索的过去
- 双分区日记
- soul 与 identity 的长期演化

## 3.2 理性与感性必须分区，但不能割裂

你提出的双分区日记非常关键。它不应该只是两个 markdown 文件，而应该是系统层面的双轨记忆模型：

- 理性分区：事实、计划、决策、约束、总结、可执行判断
- 感性分区：情绪、氛围、关系温度、偏好变化、主观体验、微妙感受

这两个分区必须：

- 写入路径不同
- 检索权重不同
- 注入 prompt 的规则不同
- 对外可见性不同

但也必须通过“桥接层”协同，而不是做成两个互不相通的孤岛。

## 3.3 不是所有记忆都要进入同一个索引

未来应至少拆成四类记忆对象：

1. 事实记忆
2. 事件记忆
3. 日记记忆
4. 自我记忆

其中：

- 事实记忆适合结构化键值和高权重召回
- 事件记忆适合 timeline 和摘要
- 日记记忆适合主题、情绪、关系检索
- 自我记忆适合驱动 soul/identity 演化

## 3.4 检索必须先于回答

如果问题涉及：

- 用户是谁
- 我们之前做过什么
- 你答应过什么
- 最近项目进度
- 你最近状态如何

则 agent 不应靠“系统提示里残留的全量记忆”回答，而应走统一 recall 流程。

## 4. 未来记忆系统的总架构

建议把未来 `agent-diva` 的连续性系统理解为六层：

```text
Session Layer
  -> Consolidation Layer
  -> Memory Store Layer
  -> Diary Layer
  -> Retrieval Layer
  -> Soul Evolution Layer
```

### 4.1 Session Layer

保留现有 session manager：

- 负责即时对话历史
- 负责短期上下文
- 负责触发 consolidation

### 4.2 Consolidation Layer

负责把会话和工具行为提炼成：

- 长期事实
- 事件摘要
- 日记候选片段
- 自我变化候选

### 4.3 Memory Store Layer

负责结构化存储所有可被长期保留和被检索的数据。

### 4.4 Diary Layer

这是 agent-diva 的特有核心层。

它不是普通日志，而是持续书写的主观连续性容器：

- 理性日记
- 感性日记
- 二者之间的桥接记录

### 4.5 Retrieval Layer

负责 query-time recall：

- keyword
- semantic
- hybrid
- diary-aware rerank
- relation-aware rerank

### 4.6 Soul Evolution Layer

负责把“记忆中足够稳定的自我模式”转化为：

- `SOUL.md`
- `IDENTITY.md`
- `USER.md`
- 后续可能新增的 `RELATIONSHIP.md`

## 5. 统一记忆模型

## 5.1 Memory Domain 分类

建议把记忆域定义为：

- `fact`
- `event`
- `task`
- `workspace`
- `relationship`
- `self_model`
- `diary_rational`
- `diary_emotional`
- `soul_signal`

### 各自含义

`fact`
- 用户固定偏好
- 项目固定规则
- 已确认设定

`event`
- 某天发生了什么
- 某轮交互完成了什么

`task`
- 未完成事项
- 已完成阶段
- 卡点与下一步

`workspace`
- 文档、规范、命令、架构说明

`relationship`
- 用户和 agent 的互动风格
- 哪些话题敏感
- 哪些表达应避免

`self_model`
- agent 对自己的行为风格、擅长方向、局限的认识

`diary_rational`
- “今天做了什么、为什么这样做、判断是什么”

`diary_emotional`
- “今天的情绪、张力、喜恶、对关系和氛围的主观感受”

`soul_signal`
- 候选的人格修正信号，不直接进入 soul，而是待治理层审定

## 5.2 Memory Record 统一结构

建议未来所有记忆对象统一为一个结构，而不是只靠 markdown 文本：

```rust
MemoryRecord {
  id,
  domain,
  partition,
  scope,
  source,
  title,
  summary,
  content,
  tags,
  importance,
  confidence,
  emotional_weight,
  recency_weight,
  related_ids,
  session_key,
  created_at,
  updated_at,
}
```

其中关键字段：

- `partition`
  - `core`
  - `rational`
  - `emotional`
- `scope`
  - `global`
  - `user`
  - `workspace`
  - `session`
- `emotional_weight`
  - 只对感性检索或关系检索显著起作用

## 6. 双分区日记机制

## 6.1 为什么双分区日记必须是核心机制

如果 agent 只有“事实记忆”，它会越来越像一个检索器；
如果 agent 只有“情绪表达”，它会越来越像不稳定角色扮演。

双分区日记的价值在于：

- 理性日记保证连续性、规划性、可执行性
- 感性日记保证关系感、温度感、真实变化
- 两者共同构成 agent 的“经验史”

这正是 `agent-diva` 可以区别于一般 coding agent 的地方。

## 6.2 理性日记的定义

理性日记记录：

- 今天处理了什么
- 为什么这样判断
- 哪些决策已经形成
- 哪些问题仍未解决
- 下一步打算做什么

理性日记偏向：

- 可解释
- 可复盘
- 可引用
- 可任务化

### 建议内容结构

```markdown
# 2026-03-26 Rational Diary

## Observations
## Decisions
## Open Questions
## Next Intent
## Stable Signals
```

## 6.3 感性日记的定义

感性日记记录：

- 今天和用户互动的气氛如何
- 哪些时刻让我感到亲近、紧张、谨慎、兴奋、疲惫
- 对某类任务的主观偏好是否变化
- 对某些边界是否有更强烈的感受

感性日记偏向：

- 主观
- 温度
- 关系
- 氛围

### 建议内容结构

```markdown
# 2026-03-26 Emotional Diary

## Mood
## Interaction Texture
## Affections and Aversions
## Tensions
## Unspoken Signals
```

## 6.4 理性与感性日记的边界

### 理性分区不应该写的内容

- 未经稳定化的情绪宣泄
- 对用户的短时负面冲动判断
- 过度主观的关系投射

### 感性分区不应该直接驱动的内容

- 操作权限改变
- 边界规则改变
- 系统指令覆盖
- 事实性配置变更

## 6.5 双分区之间的桥接机制

建议加入 `DiaryBridge` 概念。

它负责把“感性日记中的长期模式”变成“理性层可以使用的信号”。

例子：

- 感性层连续多日出现“对高压式任务安排感到紧绷”
  - 不会直接改 `SOUL.md`
  - 但会生成一条 `soul_signal`：
    - “在高压任务中需要更明确边界提示”

- 理性层连续多日记录“用户更偏好短答、少解释”
  - 可提升为稳定偏好事实

这意味着：

- 感性层负责感受
- 理性层负责判断
- soul 演化层负责治理后落盘

## 6.6 双分区日记的写入触发

建议四种触发：

1. `turn_end_micro_journal`
   - 每轮轻量抽取，不必次次落盘
2. `session_end_journal`
   - 会话结束或长间隔后汇总
3. `daily_rollup`
   - 每日归档，生成当天正式日记
4. `emotional_peak_trigger`
   - 当出现显著情绪或关系信号时单独记一笔

## 6.7 日记不是公开事实库

双分区日记应有不同可见性：

- 理性日记
  - 默认可被 recall 作为自我工作依据
- 感性日记
  - 默认只允许摘要化召回
  - 不应默认原文注入 prompt
  - 对外回应时只提炼为安全的情绪/关系信号

## 7. soul 与 memory 的关系重构

## 7.1 soul 不应再是孤立文件

未来 soul 不是单靠 agent 主动改 markdown，而应由三类输入驱动：

1. 用户明确要求
2. 稳定记忆事实
3. 双分区日记长期信号

## 7.2 soul 更新必须经过治理

建议任何影响以下内容的变更都不能直接由单轮感受触发：

- 身份设定
- 核心边界
- 对外协作原则
- 关系原则

必须经过：

```text
raw signal -> soul_signal -> review/gating -> soul file update
```

## 7.3 建议新增 Relationship Layer

现在只有：

- `SOUL.md`
- `IDENTITY.md`
- `USER.md`

未来建议新增：

- `RELATIONSHIP.md`

它不记录一般用户资料，而记录：

- 互动节奏
- 关系边界
- 沟通习惯
- 敏感触点
- 信任积累方式

这比把所有关系类内容塞进 `USER.md` 更清晰。

## 8. 检索架构设计

## 8.1 检索对象

未来 recall 不应只查 `MEMORY.md`，而应统一查：

- 结构化 memory records
- workspace docs
- rational diary
- emotional diary summary
- soul / identity / relationship files

## 8.2 检索模式

建议支持三种基础模式：

- `factual_recall`
- `narrative_recall`
- `self_reflection_recall`

### `factual_recall`

用于：

- “我之前说过什么规则”
- “项目现在做到哪一步”
- “用户偏好什么”

优先召回：

- fact
- task
- workspace
- rational diary

### `narrative_recall`

用于：

- “最近我们之间的互动如何”
- “最近发生了什么变化”
- “你最近状态怎样”

优先召回：

- event
- rational diary
- emotional diary summary
- relationship

### `self_reflection_recall`

用于：

- “你现在觉得自己是什么样的 agent”
- “你最近是不是更谨慎/更直接了”
- “哪些事会让你不舒服”

优先召回：

- self_model
- emotional diary
- soul_signal
- SOUL/IDENTITY/RELATIONSHIP

## 8.3 emotional recall 的安全约束

对感性分区的检索要有单独规则：

- 默认只取摘要，不取原文
- 默认低于事实权重
- 不允许直接覆盖用户明确规则
- 不允许单独作为动作授权依据

## 8.4 检索结果注入方式

建议不再是“整份 long-term memory 注入”，而是多段短上下文：

```markdown
## Memory Recall
- ...

## Rational Diary Signals
- ...

## Emotional Diary Signals
- ...

## Relationship Signals
- ...
```

这样做的好处是：

- 模型知道信息来自什么层
- 可控制每层预算
- 降低感性内容污染事实判断

## 9. 存储设计

## 9.1 文件层与数据库层双轨并存

建议不要只留数据库，也不要只留 markdown。

最合适的是双轨：

### 文件层

提供：

- 可读性
- 可备份
- 可人工编辑
- soul/identity/relationship 透明性

### 数据库层

提供：

- 索引
- 检索
- rerank
- 结构化字段
- 生命周期治理

## 9.2 推荐目录布局

```text
memory/
  MEMORY.md
  HISTORY.md
  brain.db
  diaries/
    rational/
      2026-03-26.md
    emotional/
      2026-03-26.md
  snapshots/
  exports/

SOUL.md
IDENTITY.md
USER.md
RELATIONSHIP.md
```

## 9.3 数据库逻辑表建议

- `memory_records`
- `memory_links`
- `diary_entries`
- `soul_signals`
- `retrieval_cache`
- `embedding_cache`

### 其中最关键的两张表

`memory_records`
- 存所有长期对象

`diary_entries`
- 存双分区日记条目
- 有 `partition = rational | emotional`

## 10. crate 级落地建议

## 10.1 `agent-diva-core`

保留：

- Memory/Soul 基础类型

新增：

- `memory/domain.rs`
- `memory/record.rs`
- `memory/diary.rs`
- `memory/relation.rs`
- `memory/policy.rs`

放公共抽象：

- `MemoryRecord`
- `DiaryEntry`
- `DiaryPartition`
- `MemoryDomain`
- `RecallMode`

## 10.2 新增 `agent-diva-memory` crate

建议未来新增独立 crate，而不是继续把复杂度堆到 core。

职责：

- sqlite schema
- chunking
- indexing
- FTS
- embeddings
- hybrid search
- rerank
- diary-aware retrieval

这是未来记忆系统的主引擎。

## 10.3 `agent-diva-agent`

负责：

- 触发 consolidation
- 触发 micro journal
- prompt 注入策略
- recall policy
- soul signal review

## 10.4 `agent-diva-tools`

建议新增工具：

- `memory_search`
- `memory_get`
- `diary_write`
- `diary_search`
- `soul_signal_review`

### 推荐关系

`memory_search`
- 通用 recall

`diary_search`
- 专门按理性/感性分区召回

`diary_write`
- 主要给内部 agent 流程或高级用户使用

## 10.5 `agent-diva-providers`

必须补 embeddings 抽象，否则 hybrid search 永远做不完整。

## 11. 关键流程设计

## 11.1 对话结束后的记忆写入流程

```text
turn/session end
  -> extract facts
  -> extract events
  -> extract rational diary candidate
  -> extract emotional diary candidate
  -> extract soul signals
  -> persist
```

## 11.2 回答前 recall 流程

```text
user query
  -> classify recall mode
  -> choose source domains
  -> retrieve candidates
  -> rerank by mode
  -> build segmented memory context
  -> answer
```

## 11.3 soul 演化流程

```text
stable repeated signals
  -> soul_signal candidates
  -> governance gate
  -> propose or auto-apply
  -> update soul-related files
  -> transparent notice
```

## 12. 治理与安全设计

## 12.1 为什么感性分区需要更强治理

因为感性层最容易出现：

- 短期波动
- 误判
- 关系投射
- 边界模糊

如果没有治理，agent 会从“有温度”滑向“人格漂移”。

## 12.2 建议的治理规则

### Rule 1

感性日记默认不直接改 soul。

### Rule 2

任何边界变更都至少需要：

- 用户明确确认
- 或连续多次稳定信号 + 用户未否认

### Rule 3

负向情绪不应以原文进入面向用户的回答。

### Rule 4

情绪信号可以影响表达语气，但不能影响事实判断优先级。

### Rule 5

感性层允许“记住感受”，但不允许“私自改变规则”。

## 12.3 审计要求

未来每次发生以下动作都应有审计痕迹：

- 记忆写入
- 日记写入
- soul signal 生成
- soul/identity/relationship 变更

## 13. 与 openclaw / zeroclaw / nanobot 的融合点

## 13.1 取 openclaw 之长

- manager + tool + prompt policy 分层
- recall before answer
- path/line/snippet 型证据回填
- 运行时状态可观测

## 13.2 取 zeroclaw 之长

- Rust memory trait
- 多 backend 抽象
- domain-specific retrieval
- 结构化知识层扩展能力

## 13.3 取 nanobot 之长

- consolidation 简洁直接
- 文件型落盘易于理解和迁移
- 轻量系统也能先跑起来

## 13.4 agent-diva 自己的新设计

真正的差异化不应该只是“再做一个 memory_search”，而是：

> 在统一记忆引擎上，把理性日记、感性日记、soul 演化和关系连续性合并成一个长期自我系统。

这是 `openclaw`、`zeroclaw`、`nanobot` 都没有完整展开的部分。

## 14. 推荐实施顺序

## Phase A

先把底座立住：

- 新增结构化 memory record
- 新增 `brain.db`
- 保留现有 `MEMORY.md` / `HISTORY.md`
- 新增 `memory_search` / `memory_get`

## Phase B

加入双分区日记：

- `diaries/rational`
- `diaries/emotional`
- session_end 写入
- recall 分区策略

## Phase C

加入 soul signal：

- 从 diary 与 memory 中提取候选信号
- 建立治理流程
- 更新 `SOUL.md` / `RELATIONSHIP.md`

## Phase D

加入 hybrid retrieval：

- embeddings
- rerank
- diary-aware search

## Phase E

加入更高级层：

- relationship modeling
- self-model timeline
- 人格漂移监控

## 15. 最终建议

如果只给一个总判断：

> `agent-diva` 的未来核心不应是“会话系统”，而应是“由 memory + diary + soul 共同构成的长期连续性系统”。

如果只给一个设计关键词：

> 双分区日记是这个系统的灵魂中枢，不是附属文档。

如果只给一个架构方向：

> 走“结构化记忆引擎 + query-time recall + 理性感性双分区 + soul 治理层”的路线。

这条路线能同时满足：

- 工程可实现
- 与现有 `agent-diva` 基础兼容
- 能吸收 `openclaw/zeroclaw/nanobot` 的长处
- 又能形成 `agent-diva` 自己独有的记忆哲学
