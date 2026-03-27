# agent-diva 最终记忆路线文档

## 1. 文档定位

这份文档是 `agent-diva` 记忆系统的最新总纲。

目标不是一次性定义“最终完整记忆体系”的所有细节，而是给出一条在现实工程上可持续推进的路线：

1. 先保证“日记”能力可靠可用。
2. 在不破坏现有外部 contract 的前提下，把基础能力尽可能做强。
3. 基础能力至少不低于 `.workspace/zeroclaw` 当前已证明有效的本地检索能力。
4. 在此之上，为你后续提出的新理念和更复杂的完整记忆体系预留明确接口与演进路径。

这意味着：

- 这份文档优先讨论“现在必须先做什么”。
- 这份文档也明确“后面怎么接 LanceDB，以及更后期如何接 Qdrant 或其他向量/检索后端”。
- 这份文档刻意避免把新理念的复杂语义、治理和产品形态过早写死。

今后若要归档旧文档，应以本文为主，旧文档作为背景资料即可。

## 2. 先讲结论

当前最合理的路线不是直接把完整记忆体系一次做完，而是分成两个大阶段：

### 阶段 A：最小可用 + 基础能力打底

目标是先完成一个可稳定运行的“基础记忆底座”，满足下面三个要求：

- `diary` 相关能力可靠可用，不回归。
- `memory_recall` 已具备本地结构化索引、关键词检索、可选语义检索、混合召回能力。
- 检索后端、embedding 能力、外部向量库接入方式已经解耦，为下一阶段升级做好准备。

### 阶段 B：引入更先进的技术与完整记忆体系

目标是接入你后续提出的新理念、新型 memory 技术或更复杂的长期记忆治理体系，但不要求在阶段 A 时就把这些复杂部分全部实现。

也就是说：

> `agent-diva` 现在应先实现“最小但完整的基础层”，再把复杂记忆体系挂载到这个基础层上，而不是反过来。

这是当前路线的核心判断。

## 3. 当前已形成的基础

截至目前，`agent-diva` 在 memory 方向已经形成了第一层有效基础：

- 独立 `agent-diva-memory` crate 已建立。
- `MemoryToolContract` / `DiaryToolContract` 已存在，tool schema 已稳定。
- `WorkspaceMemoryService` 已作为 facade 工作。
- 已有 diary 文件存储与读取链路。
- 已补本地 SQLite `brain.db`、FTS、最小 embedding provider abstraction、query/document embedding cache、hybrid-ready rerank。
- 已支持 `MEMORY.md` chunk backfill、diary backfill、SQLite recall、file fallback、snapshot/hydrate 灾后恢复链路。
- 已新增 `memory_search` / `memory_get`，并保持 `memory_recall` / `diary_read` / `diary_list` 向后兼容。
- prompt 主路径已经切换为 compact auto-recall，`MEMORY.md` 不再作为默认 system prompt 主上下文来源。

这意味着当前项目已经不再是“只有 Markdown 和记忆文件注入”的状态，而是进入了“本地混合召回基础已成型”的状态。

这一步非常重要，因为它决定了后面不需要推翻重来。

### 3.1 当前状态校正

需要明确的是，当前代码虽然已经明显接近本文路线，但还**没有**到“完整基础层完全交付”的程度。

下面这些能力当前是“已具备第一版”：

- compact recall 驱动的 prompt 主路径
- 本地 SQLite + FTS recall
- `Noop` / OpenAI-compatible embedding provider
- 懒生成 document embedding 与 query embedding cache
- hybrid-ready rerank
- snapshot 导出与冷启动 hydrate
- `memory_search` / `memory_get`

下面这些能力当前仍然**未完成**或只完成了最小骨架：

- 独立 retrieval engine 分层
- 更成熟的 semantic / hybrid 策略治理
- relationship / self-model / soul-signal 的真实闭环
- diary / memory 的治理层
- 外部向量后端适配面
- 完整长期记忆 ontology

## 4. 为什么不能直接上完整记忆体系

因为“完整记忆体系”通常包含以下复杂度：

- 更复杂的 memory ontology
- 自我模型、关系模型、长期偏好等治理
- 多层索引与多后端协同
- 写入策略治理
- 召回策略治理
- prompt 注入治理
- UI / tool / service / storage 全链路协作

如果现在直接做全量，会出现四个问题：

### 4.1 变量太多

一旦把这些内容和向量后端、tool schema、prompt policy、完整记忆模型一起改，会很难判断问题来自哪里。

### 4.2 无法保证 diary 稳定

日记是 `agent-diva` 的差异化核心之一，不能在大规模重构中失去可用性。

### 4.3 容易把边界做坏

尤其容易犯下面几种错误：

- 把复杂记忆类型重新塞回 `agent-diva-core`
- 让 service 层重新承担过多 recall/rerank 逻辑
- 一边改工具 schema 一边改后端
- 一边接外部数据库一边重做全部 memory ontology

### 4.4 复杂技术并不适合直接成为第一层底座

新技术可以更先进，但不一定适合一开始承担系统全部职责。它更适合挂在稳定基础层之后，而不是替代基础层本身。

## 5. 未来路线总原则

后续所有 memory 开发，都应遵守下面这些原则。

### 5.1 `agent-diva-memory` 是增强记忆主引擎

增强记忆能力都应放在 `agent-diva-memory` 内，而不是继续扩张 `agent-diva-core`。

`agent-diva-core` 只保留最小兼容层：

- 基础错误类型
- 基础共享类型
- 旧 memory manager 的兼容能力

不要把 hybrid、embedding、vector backend、复杂 recall 类型回流到 core。

### 5.2 tool contract 稳定优先

下面这些 contract 应长期保持稳定，至少在基础层阶段不应轻易改动：

- `memory_recall`
- `memory_search`
- `memory_get`
- `diary_read`
- `diary_list`

这五个 contract 已经足够支撑当前基础层阶段。

如果未来需要：

- `diary_search`
- `memory_write`

也应该在基础层稳定后，以增量方式新增，而不是把当前 tool schema 和后端一起推翻。

### 5.3 `WorkspaceMemoryService` 继续只做 facade

它应该负责：

- 组织调用
- 初始化和 backfill
- fallback 编排
- tool contract 适配

它不应该负责：

- 复杂 ranking
- 多后端策略治理
- 向量检索细节
- embedding 计算策略

当前实现里，部分 hybrid/semantic 编排仍在 `WorkspaceMemoryService` 内，是下一步优先要继续下沉的内容。
这些逻辑最终都应该继续下沉到 `retrieval` / `sqlite` / `vector backend` 层。

### 5.4 “先本地闭环，再外部增强”

先把本地闭环做稳，再接轻量本地向量后端，最后再考虑更重的服务型数据库，是正确顺序。

也就是说：

- 本地 SQLite / FTS / hybrid 是第一层。
- 本地向量层（如 LanceDB）是第二层增强。
- 服务型向量库（如 Qdrant）是更后期增强。
- 完整记忆体系是第三层。

### 5.5 “日记”优先级高于“先进技术展示”

任何新技术接入都不能破坏：

- diary 的写入
- diary 的读取
- diary 的日期聚合
- diary 与 recall 的桥接能力

如果新技术暂时做不到稳定支持 diary，就先只承担 semantic retrieval，不要抢 diary 的主写路径。

## 6. 第一目标：保证“日记”能力可用

在你提出的新理念开发之前，必须先把 diary 作为一个稳定子系统保障好。

### 6.1 diary 的最低保障目标

必须确保下面能力始终可用：

1. 能写入日记。
2. 能按日期读取 diary。
3. 能按日期列表聚合 diary。
4. 能把 diary 增量镜像到结构化索引中。
5. `memory_recall` 可以召回 diary 内容。
6. 在高级 semantic / vector backend 不可用时，diary 仍能通过文件 + SQLite keyword recall 工作。

### 6.2 diary 的稳定分层

未来 diary 应继续维持三层：

#### 第一层：文件层

用途：

- 用户可读
- 用户可编辑
- 可审计
- 向后兼容

这层当前已经存在，不应取消。

#### 第二层：结构化索引层

用途：

- 提供 recall 用的高性能索引
- 做 domain/scope/time 过滤
- 做 FTS
- 做语义召回的对象载体

这层应由 `brain.db` 中的 `diary_entries` 及其 FTS / embedding 表承载。

#### 第三层：未来治理层

用途：

- 控制对外暴露范围
- 支持未来更复杂的关系、自我演化与长期记忆治理能力

这一层现在不必一次做完，但必须在模型和接口上预留。

### 6.3 diary 当前不应做的事

在基础层阶段，避免以下做法：

- 不要让 diary 完全依赖外部向量库才能工作。
- 不要把 diary 写路径改成只写数据库不写文件。
- 不要让 diary 读取路径直接改为数据库唯一来源。
- 不要因为新 memory 体系而让 diary 失去可读、可写、可回溯的文件层。

## 7. 在新理念开发前，必须完成的“最小可用能力”

这部分是最关键的交付清单。

### 7.1 能力一：稳定本地 memory engine

应把以下能力视为基础底座，而不是“实验功能”：

- `brain.db` 自动初始化
- schema migration
- `memory_records` 结构化索引
- FTS5 keyword recall
- `EmbeddingProvider` 抽象
- `Noop` fallback
- hybrid-ready recall / rerank
- diary backfill
- `MEMORY.md` chunk backfill
- file fallback
- snapshot/hydrate 恢复链路

这些能力现在已经形成第一版，应继续固化和补强。

### 7.1.1 当前已完成的部分

截至当前代码状态，下列内容已经落地：

- SQLite durable store 与 schema migration
- FTS keyword recall
- compact recall 注入
- `memory_search` / `memory_get`
- embedding cache
- query/document embedding 分离
- snapshot/hydrate 恢复

后续不应再把这些能力视为临时补丁，而应围绕它们继续做结构化整理。

### 7.2 能力二：最小可用 semantic 能力

需要做到：

- embedding provider 可注入
- embedding 存储与 provider/model 解耦
- query embedding 与 document embedding 分离
- embedding 按需生成，不强制全量预计算
- semantic 不可用时自动退化为 keyword-only

这一点已经开始具备，但后面还需要继续扩展到可替换后端。

### 7.3 下一阶段优先级

在“基础层修复”完成后，建议按下面顺序推进，而不要并行大改：

1. 抽离独立 retrieval 层，把 semantic/hybrid 编排从 `WorkspaceMemoryService` 下沉。
2. 落地 `relationship` / `self_model` / `soul_signal` 的最小写入与召回闭环。
3. 增加 diary / memory 的治理策略：
   写入规则、冲突规则、注入规则、生命周期规则。
4. 预留外部向量后端接口，再考虑 LanceDB / Qdrant 适配。

这个顺序非常关键。
如果先做外部后端或复杂治理，反而会把当前刚稳定的基础层再次耦合坏。

### 7.3 能力三：最小可用多后端能力

在新理念开发前，基础层至少需要支持“后端可替换”。

最低要求：

- 本地 SQLite 可单独工作
- 未来可插入 LanceDB
- 后期可插入 Qdrant
- 未来可插入其他向量数据库
- recall pipeline 可从“单后端”演进为“混合后端”
- session 级隔离能力可表达
- namespace 级隔离能力可表达

这不意味着现在就要把所有后端都实现完，但抽象层必须为此准备好。

### 7.4 能力四：最小可用检索策略能力

至少要不弱于 `zeroclaw` 当前能力的下限：

- 关键词检索
- 语义检索
- 混合加权 merge
- backfill / reindex
- schema migration
- keyword-only fallback
- cache 或轻量重复计算抑制
- 基于 domain/scope/time/limit 的过滤
- 基于 session / namespace 的隔离过滤

这不是要求完全复制 `zeroclaw`，而是能力下限不能低于它的本地可用性。

### 7.5 能力五：最小可用可观测性

后面接更复杂技术时，必须知道系统到底在做什么，所以基础层要逐步补：

- 当前 recall 走了哪个 backend
- semantic 是否启用
- 哪个 provider/model 参与 embedding
- query 是 keyword-only 还是 hybrid
- backfill 是否执行
- reindex 是否成功
- 外部向量后端是否健康

没有这层可观测性，后面接复杂 memory 体系会非常痛苦。

## 8. 基础能力的最低标准：不能低于 zeroclaw

这里的意思不是“把 `zeroclaw` 全盘抄过来”，而是：

> 如果 `zeroclaw` 已经证明某种基础能力在本地 agent 场景下可用，那么 `agent-diva` 的基础层不应长期停留在更弱状态。

### 8.1 至少应达到的能力下限

#### 检索能力

- 本地结构化 store
- FTS keyword recall
- optional embedding recall
- hybrid merge
- basic score handling

#### 存储能力

- 本地 `brain.db`
- schema migration
- diary / memory 分表
- embedding 独立存储
- session 元数据
- namespace 元数据
- importance 元数据
- superseded / conflict resolution 元数据

#### 编排能力

- recall engine 独立
- service facade 独立
- file fallback 保留
- memory lifecycle surface 有明确落点

#### 兼容能力

- `MEMORY.md` 继续存在
- diary 文件继续存在
- 旧 tool contract 不破坏

### 8.2 后面应争取超过 zeroclaw 的部分

真正属于 `agent-diva` 差异化的，不是只达到 `zeroclaw`，而是后续在这些方向超过它：

- 更稳定的 diary 连续性
- soul 演化挂钩
- relationship / self-model 治理
- 更清晰的 memory 与人格协同边界
- 更适合本项目 GUI / manager / agent 的跨层可观测性

## 9. 下一阶段应该怎么扩：先接 LanceDB，Qdrant 后置

你提到“先实现最小可用性，以及基础能力的实现，再逐步接更复杂的后端”，这是合理路线，但要控制接入方式。

### 9.1 正确目标不是“把 LanceDB 变成唯一 memory backend”

而是：

> 让 LanceDB 成为当前阶段 semantic retrieval 的一个可插拔本地后端，同时保持本地 SQLite keyword recall 仍然存在。

换句话说，下一阶段应该做的是：

- SQLite 继续负责本地结构化记录、FTS、兼容层、最小闭环
- LanceDB 负责本地 semantic/vector 检索增强
- recall engine 负责 merge

而不是：

- 让 LanceDB 一上来就接管所有 memory 读写
- 让 diary 写入依赖 LanceDB
- 让本地闭环失效

### 9.2 为什么推荐 “SQLite + LanceDB” 的组合，而不是二选一

因为这两者承担的职责不同：

#### SQLite 更适合

- 本地单机工作
- 结构化记录
- FTS keyword recall
- migration
- diary 镜像
- 向后兼容

#### LanceDB 更适合

- 本地向量检索
- 轻部署、低运维负担
- 单用户 / 单工作区 memory 语义增强
- 与本地文件和 SQLite 闭环更贴合

#### Qdrant 更适合

- 后期服务化部署
- 多端共享或远程访问
- 更重的向量服务治理
- 更大的向量集合与长期扩展

所以合理架构不是替换关系，而是“阶段不同、职责不同”。

### 9.3 下一阶段建议新增的抽象

如果要开始接外部向量后端，建议在 `agent-diva-memory` 内继续新增或明确下面几个抽象：

#### `VectorStore` 或 `SemanticIndex`

职责：

- upsert vector
- delete vector
- query similar
- health check
- backend metadata

它不负责 keyword recall。

#### `IndexSync` 或 `MemoryIndexer`

职责：

- 把 `memory_records` / `diary_entries` 映射到向量后端
- 处理增量同步
- 处理按需 embedding
- 处理重建索引

#### `RecallPlan` 或 `RecallStrategy`

职责：

- 决定本次 recall 是否只走 SQLite keyword
- 是否走 semantic
- 是否走 hybrid merge
- 外部后端失败时如何降级

这几层补出来之后，LanceDB 接入会自然得多，Qdrant 也可以在后期顺着同一抽象接入。

### 9.4 LanceDB 最小可用版本应该包含什么

如果做一个 `v0.0.9` 或类似阶段，最小版本建议只做这些：

- `VectorStore` trait
- `LanceDbVectorStore` 最小实现
- 基于 `record_id` / `entry_id` 的 vector upsert
- metadata 至少包含：
  - `kind`
  - `domain`
  - `scope`
  - `timestamp`
- `HybridRecallEngine` 支持：
  - SQLite keyword candidates
  - LanceDB semantic candidates
  - simple weighted merge
- 外部后端失败时自动退化为 SQLite-only

### 9.5 LanceDB 这一步不该一起做的事

- 不要同时改 tool schema
- 不要同时重写 prompt recall policy
- 不要同时引入完整 relationship/self/soul graph
- 不要同时把所有 memory write path 改成事件驱动或分布式
- 不要同时把 GUI/manager 做成完整 memory console

### 9.6 这一步同时应为后续 lifecycle 能力留接口

即使当前不新增新的 tool schema，也应该在内部模型和服务层预留：

- store / upsert memory
- forget memory
- purge by namespace
- purge by session
- mark superseded / conflict resolved

原因很简单：

- 没有 lifecycle surface，就没有真正可治理的 memory system
- 后续新理念接入时，必须能修正、覆盖、清理旧记忆
- 仅有 recall 没有写入/删除/覆盖的路线是不完整的

## 10. 如果以后不用 LanceDB，或者再往后扩，还可以考虑什么

如果后面不想停在 LanceDB，或者后期要扩展，也可以接这些类型的后端，但原则不变：

### 10.1 PostgreSQL + pgvector

优点：

- 如果后期服务化部署明确，管理方便
- SQL + vector 可以一体化

缺点：

- 对本地桌面/单机 agent 来说更重

### 10.2 LanceDB / DuckDB / sqlite-vector 类方案

优点：

- 本地优先
- 更接近单机使用

其中，当前最推荐的是 LanceDB。

缺点：

- 长期多端共享和服务化能力不如 Qdrant 这类服务型后端直接

### 10.3 自建文件向量索引

优点：

- 控制权强

缺点：

- 不值得在这一阶段投入
- 很容易把精力浪费在底层索引细节，而不是 memory product 本身

总体判断现在应改成：

> 如果下一阶段需要一个“更先进但仍现实可用”、同时又不违背 `agent-diva` 轻量本地哲学的 semantic backend，LanceDB 是当前更合理的首选；Qdrant 应放到后期服务化阶段再考虑。

## 11. 在新理念接入前，完整路线应该怎么排

下面给出建议路线。

## 11.1 路线总览

```text
Phase 0  diary 可用与文件兼容
Phase 1  本地 SQLite / FTS / hybrid foundation
Phase 2  本地 vector backend 最小接入（LanceDB 优先）
Phase 3  recall policy / indexing / observability 强化
Phase 4  新理念挂载到稳定基础层
Phase 5  完整记忆体系治理与产品化
```

### Phase 0：diary 可用与文件兼容

目标：

- diary 写入、读取、列表稳定
- 用户仍可直接读写 Markdown

状态：

- 已基本具备

### Phase 1：本地 SQLite / FTS / hybrid foundation

目标：

- 建立本地 `brain.db`
- diary / `MEMORY.md` backfill
- embedding abstraction
- hybrid recall

状态：

- 已完成第一版基础

### Phase 2：本地 vector backend 最小接入

目标：

- 增加 `VectorStore` 抽象
- 最小接入 LanceDB
- 支持 `sqlite keyword + external semantic + merge`

状态：

- 建议作为下一阶段首要工作

### Phase 3：recall policy / indexing / observability 强化

目标：

- 增量索引同步
- cache
- background reindex
- health / metrics / tracing
- recall decision policy
- FTS early return
- session / namespace aware recall

状态：

- 必须在新理念真正落地前补强

### Phase 4：新理念挂载到稳定基础层

目标：

- 把你后续提出的复杂 memory 技术或新理念接入已有基础层
- 不改 diary 基础能力
- 不推翻 recall engine 基础接口

状态：

- 这是正确接入点，不应提前

### Phase 5：完整记忆体系治理与产品化

目标：

- 完整 ontology
- soul / relationship / self-model 治理
- 更复杂的 recall mode
- 明确的 UI / manager 可视化能力

状态：

- 属于后续长期路线

## 12. 在新理念开发前，真正必须完成的技术清单

这一段可以作为后续开发 checklist。

### 12.1 存储与索引

- 固化 `brain.db` schema
- 增加 schema version / migration 说明
- 补 `VectorStore` 抽象
- 区分本地结构化 store 与本地/远程 vector store
- 明确 reindex 入口
- 明确 session / namespace / importance / superseded 字段的统一表达

### 12.2 diary 保障

- 明确 diary 文件格式稳定约束
- 明确 diary 到 SQLite 的增量同步策略
- 明确 diary 记录在结构化层的统一表达
- 明确 external vector backend 不可用时 diary 不受影响

### 12.3 recall 编排

- 明确 keyword recall 输入输出
- 明确 semantic recall 输入输出
- 明确 merge 优先级
- 明确 fallback 顺序
- 明确外部 backend 错误时降级逻辑
- 明确 session / namespace 过滤与隔离规则
- 明确 cache 与 early-return 策略

### 12.4 memory lifecycle

- 明确 store / upsert 的内部接口
- 明确 forget 的内部接口
- 明确 purge by namespace / session 的内部接口
- 明确 superseded / conflict resolution 的内部接口
- 决定这些能力何时暴露为正式 tool，而不是等完整记忆体系时再补

### 12.5 embedding 与 provider

- 保持 provider/model 与存储解耦
- 明确 query/document embedding 生命周期
- 明确 embedding cache 或按需回填策略
- 不把运行时复杂 provider 配置和 memory schema 同时重构

### 12.6 可观测性

- recall path trace
- backend health
- reindex status
- last sync status
- fallback reason

### 12.7 测试

至少要稳定覆盖：

- diary 回读
- diary list
- SQLite-only recall
- semantic enabled recall
- backend failure fallback
- reindex / backfill idempotency
- tool contract compatibility
- session / namespace 隔离
- superseded 记录不回归

## 13. 未来完整记忆体系应该挂在哪些位置

当你后续的新理念开始落地时，建议按下面方式挂载，而不是直接侵入所有现有层。

### 13.1 可挂在 `retrieval` 层的能力

- 更先进的 recall 策略
- 多阶段 rerank
- temporal decay
- MMR
- relation-aware rerank
- query intent aware recall

### 13.2 可挂在 `indexing` 层的能力

- 更复杂的 chunking
- entity extraction
- memory graph node generation
- summary cache
- hierarchical memory
- procedural memory extraction

### 13.3 可挂在 `governance` 层的能力

- self / soul / relation 候选治理
- emotion / preference 演化治理
- 记忆写入质量门禁
- 记忆冲突解决

### 13.4 可挂在 `product` 层的能力

- memory inspect UI
- reindex UI
- recall trace view
- diary bridge view
- memory source transparency

## 14. 明确哪些事情现在不该做

为了避免偏离路线，下面这些事情在现阶段都不应优先：

- 不要先做完整 memory ontology 再补基础 recall
- 不要先做所有 fancy retrieval 策略再补可用性
- 不要先改全部 tool schema
- 不要先把核心 memory 类型塞回 core
- 不要先让外部向量库成为唯一依赖
- 不要先做“高级人格系统”而牺牲 diary 和 recall 稳定性

## 15. 最终建议

如果把这份文档压缩成一句话，结论是：

> `agent-diva` 接下来应先把 diary 可用性和本地/可扩展 recall 基础层做稳，基础能力至少做到不低于 `zeroclaw` 的本地检索能力，再优先用可插拔方式接入 LanceDB 这类轻量本地向量后端，把 Qdrant 这类服务型后端放到后期，最后再把你后续提出的更复杂、更先进的完整记忆体系挂载到这个稳定底座上。

## 16. 建议的下一步实施顺序

建议后续开发严格按下面顺序推进：

1. 固化当前 `agent-diva-memory` 的本地基础层，并补足迁移、可观测性和索引同步说明。
2. 新增 `VectorStore` / `SemanticIndex` 抽象，准备接本地优先的 vector backend。
3. 做一个“最小 LanceDB 接入”版本，保持 SQLite keyword recall 不变。
4. 补 recall policy、backend health、reindex，以及 session / namespace / lifecycle 基础接口。
5. 在这之后，再开始把你的新理念以增量方式接到现有 memory foundation 上。

这就是建议保留的唯一主路线。
