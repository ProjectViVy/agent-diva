# agent-diva 记忆系统 Phase A 规格

## 1. 本阶段目标

本阶段只做两件事：

1. 完整的基础能力架构设计
2. 基础的“日记存储”功能设计，但仅限理性分析型日记

本阶段明确不做：

- 不实现感性日记
- 不实现双分区之间的桥接逻辑
- 不实现 soul signal 自动演化
- 不实现 embeddings / vector search / hybrid retrieval
- 不写任何代码

这是一份用于后续开发的边界清晰的基础规格文档。

## 2. 范围收敛后的核心判断

虽然长期目标是“综合记忆系统 + 双分区日记”，但从工程节奏看，第一阶段必须先把最底层打稳。

因此 Phase A 的正确定位是：

> 先把 future-proof 的 memory foundation 设计完整，再只实现最安全、最稳定、最有工程价值的理性日记存储。

这里的“理性日记”不是情绪表达，而是 agent 对外部对象和工作进展的分析记录，例如：

- 某个 GitHub 项目是什么结构
- 去哪里找文档
- 某个仓库的主要模块是什么
- 某件事已经查到了什么、还缺什么
- 下一步建议如何推进

换句话说，Phase A 的日记更接近：

- research log
- analysis diary
- reasoning note

而不是：

- mood journal
- emotional state log

## 3. 本阶段的设计原则

## 3.1 先保证“可存、可读、可检索扩展”，不追求一步到位

Phase A 不应该设计成一个一次性临时方案，否则后续感性分区接入时会推翻重来。

所以本阶段虽然只实现理性日记存储，但数据模型必须提前预留：

- partition
- domain
- scope
- future retrieval metadata

## 3.2 理性日记先作为“分析型工作记忆”落地

Phase A 的理性日记只服务三类内容：

1. 项目/仓库分析
2. 文档定位与知识路径
3. 任务推进中的判断与结论

不记录：

- 情绪
- 人际温度
- 主观不适
- 关系投射

## 3.3 文件可读性优先，数据库扩展预留

本阶段最终真正落盘的对象建议仍然是 markdown 文件。

原因：

- 易读
- 易审查
- 易手动修正
- 与现有 `memory/` 体系兼容

但规格必须预留未来数据库字段映射。

## 4. Phase A 的系统边界

## 4.1 在系统中的位置

本阶段只真正触碰以下两层：

```text
Session Layer
  -> Consolidation / Diary Extraction Design
  -> Rational Diary Storage Design
```

不会真正展开的层：

- Emotional Diary Layer
- Retrieval Engine
- Soul Evolution Governance

## 4.2 与现有模块的关系

### 保留不动

- `SessionManager`
- `consolidation.rs`
- `MemoryManager`
- `ContextBuilder`
- `SOUL.md` / `IDENTITY.md` / `USER.md`

### 设计上新增但暂不编码

- 结构化 memory engine
- diary store abstraction
- diary query API
- future recall policy

## 5. Phase A 的基础架构设计

## 5.1 总体分层

即便本阶段不写代码，后续实现必须按下面的分层推进：

### Layer 1: Session History

职责：

- 保存原始会话消息
- 提供 consolidation 输入

### Layer 2: Durable Memory

职责：

- 保存长期事实
- 保存历史摘要
- 承接未来结构化 memory records

### Layer 3: Rational Diary

职责：

- 保存“分析型日记”
- 记录探索过程与阶段判断
- 衔接 future retrieval

### Layer 4: Retrieval-ready Metadata

职责：

- 为未来 recall 留出路径
- 不在本阶段实现真正检索

## 5.2 未来 crate 归属建议

本阶段虽然不写代码，但必须先明确归属。

### `agent-diva-core`

适合放：

- diary 基础类型
- partition/domain/scope 枚举
- 文件路径约定

### `agent-diva-agent`

适合放：

- 日记提炼策略
- 写入触发条件
- 与 conversation/consolidation 的协作逻辑

### `agent-diva-tools`

未来适合放：

- `diary_write`
- `diary_read`
- `diary_list`

### 新 crate 预留：`agent-diva-memory`

后续需要独立记忆引擎时引入。

## 6. 理性日记的定义

## 6.1 核心语义

理性日记是 agent 对客观对象和工作推进过程的分析性沉淀。

它记录的是：

- 我观察到了什么
- 我确认了什么
- 我还不确定什么
- 我建议下一步做什么

它不是最终知识库，而是：

> 介于原始会话和长期事实之间的“中层分析沉淀”。

## 6.2 典型内容

应当记录的例子：

- “`.workspace/openclaw` 的 memory 系统主要由 manager、search-manager、memory-tool 三层组成。”
- “zeroclaw 的硬件 RAG 在 `src/rag/mod.rs`，支持 markdown/txt/pdf。”
- “该项目的文档主要在 `docs/reference` 和 README 中。”
- “下一步若要实现 recall，建议优先补 embeddings trait，而不是先做 qdrant。”

不应记录的例子：

- “今天我有点烦躁。”
- “我觉得用户是不是不信任我。”
- “我很喜欢这个项目的风格。”

## 6.3 理性日记与 MEMORY.md 的区别

`MEMORY.md`
- 更接近稳定事实
- 适合较长期、较确定内容

理性日记
- 更接近过程分析
- 可保留阶段性判断
- 允许“当前结论，但可能更新”

所以两者不是互斥关系，而是：

```text
Conversation -> Rational Diary -> Stable Fact (optional)
```

## 7. 理性日记的存储设计

## 7.1 目录结构

建议未来目录如下：

```text
memory/
  MEMORY.md
  HISTORY.md
  diaries/
    rational/
      2026-03-26.md
```

本阶段只定义 `rational/`，不定义 `emotional/` 的实现行为。

可以预留目录，但不启用：

```text
memory/
  diaries/
    emotional/
```

## 7.2 文件命名规则

采用按天归档：

- `YYYY-MM-DD.md`

原因：

- 与现有 daily note 逻辑相容
- 便于人工浏览
- 便于未来做按日期 recall

## 7.3 单日日记结构

建议每日日记采用稳定模板：

```markdown
# Rational Diary - 2026-03-26

## Entries

### 2026-03-26 10:15
Title: OpenClaw memory architecture
Domain: workspace-analysis
Scope: repository
Tags: openclaw, memory, architecture

Observation:
...

Conclusion:
...

Next Step:
...
```

## 7.4 为什么采用半结构化 markdown

原因有四个：

1. 人可读
2. 未来可解析
3. 方便审计
4. 与现有 `memory/` 文件哲学一致

也就是说，本阶段的 markdown 不是“随便写一段话”，而是：

> 用 markdown 作为结构化记录的可读承载层。

## 8. 理性日记条目模型

虽然本阶段不编码，但建议条目模型已经固定。

## 8.1 逻辑字段

每条理性日记建议具备：

- `timestamp`
- `title`
- `domain`
- `scope`
- `tags`
- `observation`
- `conclusion`
- `next_step`
- `sources`
- `confidence`

## 8.2 字段语义

### `domain`

建议 Phase A 先限制在以下枚举：

- `workspace-analysis`
- `docs-discovery`
- `project-research`
- `task-planning`
- `architecture-note`

### `scope`

建议：

- `repository`
- `workspace`
- `external-project`
- `session`

### `sources`

即便本阶段不做真正 citations engine，也建议记录来源路径。

例如：

- `agent-diva-agent/src/context.rs`
- `.workspace/openclaw/src/memory/manager.ts`
- `docs/dev/archive/architecture-reports/...`

### `confidence`

建议用低复杂度枚举：

- `high`
- `medium`
- `low`

这是为未来 recall 和二次稳定化准备的。

## 9. 写入触发规则

## 9.1 本阶段只定义，不实现自动写入

这很重要。

Phase A 只做规格，不做自动化行为。

未来可实现的触发点建议如下：

### Trigger A: research milestone

当 agent 完成一轮明显的调研阶段时写一条。

例如：

- “已确认 openclaw memory 结构”
- “已定位 zeroclaw RAG 源码入口”

### Trigger B: architecture conclusion

当 agent 形成了一个明确设计判断时写一条。

### Trigger C: doc path discovery

当 agent 找到关键文档入口时写一条。

例如：

- “项目开发文档主要在 `docs/dev/archive/...`”

### Trigger D: plan refinement

当 agent 对下一步实施路线有了稳定判断时写一条。

## 9.2 不建议的触发

以下情况不应写理性日记：

- 每轮微小命令输出
- 纯闲聊
- 未形成结论的噪声搜索
- 仅情绪化表达

## 10. 读取与使用规则

## 10.1 本阶段只设计“存储”，但必须先定义未来怎么用

否则存储格式会很快失控。

未来理性日记主要有三种用途：

1. 复盘
2. recall
3. 稳定事实提炼

## 10.2 复盘用途

适合：

- 回顾某天做过哪些调研
- 找到之前关于某项目的判断

## 10.3 recall 用途

未来如果用户问：

- “你之前怎么评价 openclaw 的 memory 结构？”
- “文档入口你上次查到了哪些？”

则理性日记应作为优先 recall 来源之一。

## 10.4 稳定事实提炼

部分理性日记内容未来可以上升为稳定 memory。

例如：

- “文档入口在何处”
- “某仓库架构主轴是什么”

但不是所有理性日记都应进入 `MEMORY.md`。

## 11. 与未来感性分区的兼容约束

虽然本阶段不做感性分区，但现在必须规定兼容边界。

## 11.1 目录兼容

当前理性日记目录必须允许未来扩展为：

```text
memory/diaries/rational/
memory/diaries/emotional/
```

## 11.2 字段兼容

即便当前不使用，也建议模型层预留：

- `partition`
- `emotional_weight`

但 Phase A 文档里要明确：

- 理性日记写入时 `partition = rational`
- `emotional_weight` 恒为 0 或未使用

## 11.3 使用兼容

未来感性分区加入后：

- 理性日记仍然是默认可引用、可复盘、可稳定化的主干
- 感性日记不会反向污染理性日记的字段定义

## 12. 推荐的数据演进路线

## Phase A

- markdown-only rational diary
- semi-structured entries
- manual/human-readable first

## Phase B

- 增加 diary record parser
- 增加 list/read API
- 增加理性日记索引元数据

## Phase C

- 增加 emotional partition
- 增加 partition-aware recall

## Phase D

- diary 与 structured memory records 打通
- diary-to-memory promotion

## 13. 本阶段的交付物建议

如果后续进入真正开发，Phase A 只需要落三类能力：

1. 类型与路径约定
2. 理性日记文件模板
3. 基础的 rational diary storage 行为

注意，这里的“基础 storage 行为”也应限定为：

- 创建目录
- 按天落盘
- 追加条目
- 保持模板结构

不包含：

- 检索
- rerank
- 自动 soul 演化
- emotional partition

## 14. 最终建议

本阶段最重要的不是“先把感性也做掉”，而是：

> 先让 `agent-diva` 拥有一个结构清晰、未来可扩展、且真正有工程价值的理性分析日记层。

因为这一层一旦稳定，后面无论接：

- recall
- emotional diary
- soul governance
- relationship memory

都会有一个干净的基础可依附。

所以本阶段的正确策略是：

> 只做完整基础架构设计，并把“理性日记存储”作为第一个落地能力。
