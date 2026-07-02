# agent-diva 记忆框架能力对齐与实验功能规划

## 1. 本文目的

这份文档用于修正一个容易出现的误解：

> Phase A 不是“做一个理性日记功能”，而是“先把一套能承接 zeroclaw / openclaw 设计理念与能力上限的记忆框架设计完整”，理性日记只是其中第一个实验性功能。

换句话说：

- **目标主线**：能力框架对齐
- **首个实验点**：理性日记存储

本文强调的是“框架能力边界”和“演进路线”，不是具体代码实现。

## 2. 目标校正

## 2.1 真正目标

未来的 `agent-diva` 记忆框架，至少要能承接以下能力方向：

- `zeroclaw` 式的 memory trait / backend / retrieval pipeline 思路
- `openclaw` 式的 manager / tool / prompt policy / recall-before-answer 思路
- 当前 `agent-diva` 已有的 soul / identity / bootstrap / consolidation 能力

如果只做一个“日记文件”，那只是功能碎片，无法形成可持续架构。

因此本阶段的目标必须改写为：

> 设计一个可扩展、可分层、可检索、可治理的记忆框架，并在这个框架里只落地最小实验能力：理性日记存储。

## 2.2 最低能力基线

你提到“至少能力对齐 zeroclaw”，这是一个非常有效的锚点。

这意味着未来 `agent-diva` 的记忆框架至少要预留以下能力类型：

1. 统一记忆抽象
2. 多类存储后端
3. query-driven recall
4. 结构化长期记忆
5. 记忆工具化暴露
6. prompt 注入的主动召回层
7. 后续接 diary / soul / relationship 的能力

本阶段不实现这些能力，但架构必须为这些能力预留位置。

## 3. 应承接哪些设计理念

## 3.1 承接 zeroclaw 的核心理念

从本地源码与既有调研看，`zeroclaw` 最关键的不是某个具体实现，而是这几条理念：

### 理念 A：Memory 是独立子系统，不是附属文件

`zeroclaw` 有：

- `Memory` trait
- 多 backend
- retrieval pipeline
- memory tool

对 `agent-diva` 的意义：

- 记忆不能继续只是 `MEMORY.md` + `HISTORY.md`
- 这些文件未来应只是某种视图或落盘形式
- 真正核心应是可扩展的 memory framework

### 理念 B：记忆分层

`zeroclaw` 明确区分：

- 会话历史
- 长期记忆
- 检索/注入层

对 `agent-diva` 的意义：

- session 不等于 memory
- diary 也不等于 memory 全部
- soul 更不等于 memory 全部

### 理念 C：RAG 可以是 domain-aware 的

`zeroclaw` 不只有通用 memory，还有专用 hardware RAG。

对 `agent-diva` 的意义：

- 将来不必只有一个统一 recall
- 可以有 `workspace recall`
- 可以有 `memory recall`
- 可以有 `diary recall`
- 可以有 `relationship/self recall`

## 3.2 承接 openclaw 的核心理念

`openclaw` 最值得承接的是工程组织方式：

### 理念 D：Recall 是运行时能力，不是静态 prompt 拼接

对 `agent-diva` 的意义：

- 未来不应继续靠“全量 MEMORY 注入”
- 应该是“需要时 recall”

### 理念 E：Tool 化是必要边界

`openclaw` 的 `memory_search` / `memory_get` 说明：

- 检索和读取应分离
- 检索结果应结构化
- 模型应被约束先检索再回答

### 理念 F：配置与运行时状态应可观测

未来 `agent-diva` 记忆框架至少应支持：

- 当前使用什么 backend
- diary 是否启用
- recall policy 是否启用
- 哪些源参与索引

## 3.3 承接 agent-diva 自己已有的理念

当前 `agent-diva` 已经有自己独特的方向：

- soul
- identity
- bootstrap
- 对持续人格演化的重视

所以未来框架不是把 `agent-diva` 变成 `zeroclaw` 或 `openclaw` 的翻版，而是：

> 以 zeroclaw/openclaw 的记忆工程能力为骨架，以 agent-diva 的 soul/continuity 哲学为中枢。

## 4. 框架能力地图

下面这张能力地图用来区分：

- 哪些是未来框架必须具备的一级能力
- 哪些是当前阶段只做接口和边界
- 哪些只是实验功能

## 4.1 一级能力模块

建议未来记忆框架至少包含这 8 个模块：

1. `Session Context`
2. `Durable Memory`
3. `Diary System`
4. `Recall Engine`
5. `Memory Tools`
6. `Prompt Recall Policy`
7. `Soul/Identity Integration`
8. `Governance/Audit`

## 4.2 当前阶段的落实关系

### 这阶段必须设计完整的

- `Durable Memory` 的抽象边界
- `Diary System` 在框架中的位置
- `Recall Engine` 的未来接口
- `Memory Tools` 的未来形态
- `Soul/Identity Integration` 的衔接关系
- `Governance/Audit` 的基本约束

### 这阶段允许只作为实验功能落地的

- `Diary System` 中的 `rational diary storage`

### 这阶段明确不实现的

- 真正 recall engine
- embeddings/vector
- emotional diary
- soul signal automation

## 5. 框架视角下，理性日记到底是什么

## 5.1 不是主功能，而是实验锚点

理性日记在本阶段的角色不是“系统目标本身”，而是：

- 验证记忆框架是否能容纳新 memory domain
- 验证半结构化存储方案是否稳定
- 验证 future recall-ready metadata 是否合理
- 验证“process memory”是否值得保留

所以它的价值在于：

> 用最低风险的方式测试未来记忆框架的一部分。

## 5.2 为什么优先选理性日记做实验

因为它最适合做第一步实验：

- 风险低
- 解释性强
- 易于审查
- 不容易触发人格漂移
- 能直接服务调研、规划、架构设计类工作

## 5.3 当前阶段它服务什么能力

理性日记应该优先服务这些典型场景：

- 对某个仓库做架构分析
- 记录某类文档的入口位置
- 沉淀某次调研的阶段性判断
- 记录下一步技术路线判断

也就是你说的：

- “某一些 GitHub 项目是什么样子”
- “去哪里找文档”

这正说明它是**分析型实验功能**，不是情感型能力。

## 6. 对齐 zeroclaw 级能力时，本阶段必须预留什么

如果未来想至少能力对齐 `zeroclaw`，那么现在的文档和抽象必须至少预留以下接口位。

## 6.1 Memory Abstraction

未来必须有统一抽象，类似：

- `store`
- `recall`
- `get`
- `list`
- `forget`

本阶段虽然不编码，但文档必须把 diary 明确为：

- memory domain 的一种
- 而不是孤立旁路系统

## 6.2 Backend Strategy

未来至少应支持的后端路线：

- markdown/file view
- sqlite local store
- 后续 remote/vector backend

因此本阶段的日记存储设计不能把自己锁死为“只有 markdown 文本，没有结构字段”。

## 6.3 Retrieval-ready Metadata

即使现在不做 recall，也必须让 diary 条目具备未来可检索字段：

- domain
- scope
- tags
- source paths
- confidence
- timestamps

## 6.4 Prompt Integration Slot

未来应有独立的 recall 注入层，而不是在 `ContextBuilder` 里直接硬拼文件。

所以本阶段文档里必须明确：

- rational diary 是未来 recall source
- 不是永久的“只靠人工读文件”功能

## 6.5 Tool Contract Slot

未来 diary 相关能力建议至少预留：

- `diary_write`
- `diary_read`
- `diary_list`
- 后续 `diary_search`

## 7. 建议的目标架构表述

为了避免日后再把 diary 误当作主目标，建议把未来框架表述固定成下面这句话：

> agent-diva 的未来核心是一个可承接 session、memory、diary、soul、relationship 的长期连续性框架；理性日记只是该框架在第一阶段落地的实验域。

## 8. Phase A 重新定义

建议把当前阶段重新命名为：

> `Phase A: Capability-Parity Foundation + Rational Diary Experiment`

而不是：

> `Phase A: Diary Storage`

因为两者含义完全不同。

前者强调：

- 框架先行
- 实验功能后置

后者容易让后续开发偏到“为 diary 造系统”。

## 9. 本阶段的正确交付标准

如果按能力对齐视角来看，这个阶段的交付不应该只检查“日记目录和格式”。

更应该检查这四件事：

### 1. 框架边界是否完整

是否清楚区分：

- session
- durable memory
- diary
- future recall
- soul integration

### 2. 是否能承接 zeroclaw/openclaw 的未来能力

是否已经预留：

- abstraction
- backend
- retrieval slot
- tool slot
- policy slot

### 3. 理性日记是否只是实验域

是否明确：

- 只是一种 memory domain
- 不是记忆系统本体

### 4. 是否避免锁死未来设计

是否确保：

- emotional partition 未来可接入
- retrieval future-ready
- soul governance future-ready

## 10. 对现有 Phase A 文档的修正建议

现有 `dev/docs/2026-03-26-agent-diva-memory-phase-a-spec.md` 的方向基本正确，但建议在理解上做以下修正：

### 修正 1

把“本阶段只做两件事”理解为：

- 设计完整基础框架
- 在框架里只选理性日记做实验落点

而不是“框架目标就是理性日记”。

### 修正 2

把理性日记明确标记为：

- `experimental memory domain`

### 修正 3

把未来能力基线明确写成：

- 至少对齐 `zeroclaw` 的 memory abstraction 能力
- 组织方式尽量靠近 `openclaw` 的 tool/policy/manager 分层

## 11. 推荐的下一份文档

如果继续只写文档、不写代码，最合理的下一步不是继续扩写 diary 内容，而是补一份：

> `Memory Framework Interfaces Spec`

内容应包括：

- memory domain model
- memory store trait
- diary store trait
- recall engine trait
- tool contracts
- prompt integration contracts

这样后续真正实现时，就不会再围着 diary 单点打转。

## 12. 结论

最后把你的意思翻成一句最清楚的话：

> 日记不是这套系统的唯一功能，也不是主要目标；它只是第一阶段为了验证整个记忆框架设计而选择的一个重要实验功能。

而这套框架真正的目标应当是：

> 在设计理念和能力上，至少能承接 zeroclaw 级别的记忆系统，并吸收 openclaw 的运行时检索与工具化经验。

如果后续开发始终围绕这个锚点推进，那么 Phase A 就不会跑偏。
