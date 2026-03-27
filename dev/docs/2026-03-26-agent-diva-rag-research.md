# agent-diva RAG 调研与方案设计

## 1. 调研目标

本文基于仓库内 `.workspace/zeroclaw`、`.workspace/openclaw`、`.workspace/nanobot` 与当前 `agent-diva` 源码做横向调研，回答四个问题：

1. `zeroclaw` 是否已经具备 RAG 或近似 RAG 体系。
2. `openclaw` 的 RAG/记忆检索系统是怎么做的。
3. `nanobot` 是否具备同等级的 RAG 系统。
4. 如果 `agent-diva` 要建设 RAG，当前基础是什么，建议怎么做，应该落在哪些 crate。

这里把“RAG”限定为一条完整链路：

- 可被检索的数据源
- 可持续更新的索引
- 查询时的召回与排序
- 将检索结果安全注入到模型上下文
- 通过工具或框架约束让模型在需要时显式检索，而不是只靠大上下文硬塞

## 2. 本次阅读的关键源码

### agent-diva

- `agent-diva-agent/src/context.rs`
- `agent-diva-agent/src/consolidation.rs`
- `agent-diva-core/src/memory/manager.rs`
- `agent-diva-core/src/memory/mod.rs`
- `agent-diva-providers/src/lib.rs`

### zeroclaw

- `.workspace/zeroclaw/src/rag/mod.rs`
- `.workspace/zeroclaw/src/memory/mod.rs`
- `.workspace/zeroclaw/src/memory/traits.rs`
- `.workspace/zeroclaw/src/memory/sqlite.rs`
- `.workspace/zeroclaw/src/memory/retrieval.rs`
- `.workspace/zeroclaw/src/memory/qdrant.rs`
- `.workspace/zeroclaw/src/tools/memory_recall.rs`
- `.workspace/zeroclaw/src/tools/knowledge_tool.rs`

### openclaw

- `.workspace/openclaw/src/agents/memory-search.ts`
- `.workspace/openclaw/src/agents/tools/memory-tool.ts`
- `.workspace/openclaw/src/memory/manager.ts`
- `.workspace/openclaw/src/memory/search-manager.ts`
- `.workspace/openclaw/src/memory/manager-search.ts`
- `.workspace/openclaw/src/memory/hybrid.ts`
- `.workspace/openclaw/src/memory/types.ts`
- `.workspace/openclaw/extensions/memory-core/index.ts`

### nanobot

- `.workspace/nanobot/nanobot/agent/memory.py`
- `.workspace/nanobot/nanobot/templates/memory/MEMORY.md`

## 3. 结论先行

### 3.1 简短结论

- `openclaw` 有成熟、独立、可配置、工具化的 RAG/记忆检索体系，已经不是“提示词里塞 MEMORY.md”这种轻量方案。
- `zeroclaw` 也已经具备较完整的记忆检索能力，并额外有面向硬件 datasheet 的专用 RAG 与知识图谱能力。
- `nanobot` 目前更像“持久化记忆 + 历史归档 + grep 友好文本”，不属于完整 RAG。
- `agent-diva` 当前只有“长期记忆压缩 + 系统提示词注入”，严格说不是 RAG，而是 memory consolidation。

### 3.2 对 agent-diva 的关键判断

`agent-diva` 现有“记忆”基础是：

- 会把历史消息压缩到 `memory/MEMORY.md` 与 `memory/HISTORY.md`
- 会在构造 system prompt 时直接注入 `MEMORY.md`
- 有 workspace、本地文件、shell、web 等工具

但它缺少 RAG 的四个关键部件：

- 没有独立的索引结构
- 没有 embedding 接口与向量检索实现
- 没有 query-time recall tool
- 没有“先检索再回答”的执行约束

所以如果说“`agent-diva` 现在有什么 RAG”，最准确的表述是：

> 它现在有一个文件型长期记忆系统，但没有真正的 retrieval-augmented generation。

## 4. 三个参照项目分别是什么状态

## 4.1 openclaw：完整的 memory-RAG 系统

从源码看，`openclaw` 的记忆检索不是附属能力，而是一个独立子系统，特点很明确：

- 有独立配置解析层：`src/agents/memory-search.ts`
- 有独立 manager：`src/memory/manager.ts`
- 有后端选择层：`src/memory/search-manager.ts`
- 有检索执行层：`src/memory/manager-search.ts`
- 有工具暴露层：`memory_search`、`memory_get`
- 有 prompt 注入约束：`extensions/memory-core/index.ts`

### 关键能力

1. 数据源不是单一文件

- `memory`
- `sessions`
- `extraPaths`
- 可选 multimodal 资源

2. 检索不是单一路径

- FTS 关键词检索
- 向量检索
- hybrid merge
- MMR 重排
- temporal decay

3. 索引不是临时的

- SQLite 持久化
- 可启用向量扩展
- 文件 watch
- 会话增量同步
- session start/on search/interval 多触发点同步

4. 工具链闭环完整

- `memory_search` 返回 snippet、path、line range、score、source
- `memory_get` 再按路径和行范围读取原文
- prompt 明确要求“回答历史/偏好/决策类问题前先搜 memory_search”

### 设计上的优点

- 检索结果是结构化的，不是把整份 `MEMORY.md` 硬塞给模型
- 检索与原文读取分离，能控制注入 token
- 支持 backend fallback，QMD 失败可回 builtin
- 检索状态、provider 状态、vector/fts 状态都可探测

### 对 agent-diva 的启发

`openclaw` 最值得借鉴的不是某个向量库，而是这三个分层：

1. `config/resolve`
2. `index/search manager`
3. `tool + prompt policy`

这三层拆开后，RAG 才能长期维护。

## 4.2 zeroclaw：双轨体系，既有 memory-RAG，也有专用 datasheet RAG

`zeroclaw` 的特征和 `openclaw` 不完全一样。它不是单一“记忆搜索”，而是两条线并行：

1. 通用 memory 系统
2. 面向硬件资料的专用 RAG

### 通用 memory 系统

从 `src/memory` 看，它已经有：

- `Memory` trait，定义 store/recall/get/list/forget 等能力
- 多 backend：`sqlite`、`qdrant`、`markdown`、`postgres`、`none`
- `retrieval.rs` 多阶段检索流水线
- `sqlite.rs` 内建 FTS5 + embedding blob + hybrid merge
- `qdrant.rs` 远程向量库后端
- `tools/memory_recall.rs` 检索工具

这说明 `zeroclaw` 的 memory 已经不是单纯文本文件，而是统一抽象后的可替换检索层。

### 专用 hardware RAG

`src/rag/mod.rs` 非常有代表性。它做的不是通用聊天记忆，而是：

- 读取 datasheet 目录
- 支持 markdown、txt、可选 pdf
- 解析 pin alias
- 将 datasheet chunk 化
- 面向硬件 pin/board 问题做专用检索

这类设计说明一个重要思想：

> RAG 不一定只有一套“全局知识库”，也可以是按任务域拆开的专用索引器。

### 知识图谱能力

`tools/knowledge_tool.rs` 与相关 `knowledge_graph` 模块说明它还尝试把“经验、决策、模式、专家”结构化为图，而不只是文本 chunk。

这比常规 RAG 更进一步，适合：

- 架构决策沉淀
- lessons learned
- 模式复用
- 专家发现

### 对 agent-diva 的启发

`zeroclaw` 给出的价值不在于“一定要上知识图谱”，而在于两点：

1. RAG 可以按 domain 拆成多个专用索引器
2. 不是所有知识都该进统一向量库，决策/规则/模式类知识可以结构化

## 4.3 nanobot：不是完整 RAG，更像文件型记忆系统

`nanobot/nanobot/agent/memory.py` 的重点在：

- `MEMORY.md`
- `HISTORY.md`
- 消息段 consolidation
- provider tool call `save_memory`
- consolidation 失败时 raw archive

它的核心目标是：

- 保留长期记忆
- 把旧消息压缩成可读文件
- 让 agent 下次带着记忆继续工作

但没有看到以下关键件：

- 独立 embedding provider 接口
- 向量索引
- hybrid retrieval
- query-time memory_search tool
- path/line/snippet 级引用回填

所以 `nanobot` 的定位更接近：

- “可持续演进的文本记忆”
- 而不是“检索增强生成系统”

这点和当前 `agent-diva` 非常接近。

## 5. agent-diva 当前到底有什么

## 5.1 已有能力

### 1. 记忆归档

`agent-diva-agent/src/consolidation.rs` 会在消息数达到阈值后：

- 取旧消息
- 调用模型做 consolidation
- 更新 `memory/MEMORY.md`
- 追加 `memory/HISTORY.md`

### 2. 记忆读取

`agent-diva-core/src/memory/manager.rs` 已有：

- `load_memory`
- `save_memory`
- `append_history`
- `load_daily_note`
- `list_memory_files`
- `get_memory_context`

### 3. 上下文注入

`agent-diva-agent/src/context.rs` 会把 `MEMORY.md` 直接拼到 system prompt。

### 4. 工具基础设施已经存在

`agent-diva-tools` 已经有：

- filesystem
- shell
- web
- cron
- message
- spawn

这意味着 `agent-diva` 并不缺“工具框架”，缺的是专门的 memory retrieval tool。

## 5.2 现阶段的边界

当前 `agent-diva` 的问题不是“完全没有 memory”，而是 memory 只停留在这一步：

- 写文件
- 读整份文件
- 拼进 prompt

这会带来几个直接问题：

1. 规模一大，prompt 线性膨胀。
2. 无法按 query 做局部召回。
3. 无法给出可验证的 snippet/path/line 证据。
4. 无法做 hybrid retrieval。
5. 无法对不同知识域做分桶索引。

## 5.3 因此，agent-diva 现在“有什么 RAG”

严格定义下：

- 没有完整 RAG
- 有长期记忆系统
- 有作为 RAG 前置基础的 memory 文件与工具框架

所以可以把它定位为：

> “RAG-ready 的 memory substrate”，但还不是 retrieval system。

## 6. 如果 agent-diva 建 RAG，RAG 应该是什么

我建议不要把 `agent-diva` 的 RAG 理解成“给 MEMORY.md 做 embedding”这么窄。

更合适的定义是：

> agent-diva 的 RAG 是一个面向 agent 执行场景的 workspace knowledge retrieval system。

它应该服务四类场景：

1. 用户历史与偏好
2. 会话与任务历史
3. 工作区文档与代码说明
4. 结构化规则与操作知识

对应可检索数据源建议如下：

- `memory/MEMORY.md`
- `memory/HISTORY.md`
- `memory/*.md` 日记与沉淀
- `AGENTS.md`、`SOUL.md`、`USER.md`、`IDENTITY.md`
- `docs/`、`commands/`、`dev/docs/`
- 可选：session transcript
- 可选：crate-level README / design docs
- 后续可选：代码符号级摘要，而不是原始源码全文

## 7. agent-diva 适合采用哪种路线

## 7.1 不建议直接照搬 openclaw

原因：

- `openclaw` 是 TypeScript 架构，迁移成本高
- 它的能力范围更大，直接照搬容易过重
- `agent-diva` 当前 provider 层甚至还没有 embeddings 接口

## 7.2 更适合“openclaw 分层 + zeroclaw 渐进式后端”组合路线

建议组合：

- 借 `openclaw` 的 tool/prompt/manager 分层
- 借 `zeroclaw` 的 Rust memory trait + backend 抽象
- 先做 SQLite FTS + 本地 chunk store
- 再补 embeddings 和 hybrid
- 最后再考虑 qdrant/pgvector 等远程后端

## 8. 面向 agent-diva 的建议架构

## 8.1 crate 级职责划分

### `agent-diva-core`

放公共抽象与数据结构：

- `RagDocument`
- `RagChunk`
- `RagSource`
- `RagSearchResult`
- `EmbeddingProvider` trait
- `RagIndex` trait
- `RagIngestPlan`
- `RagQueryOptions`

原因：这是跨 agent、tools、providers、manager 都会用到的核心域模型。

### `agent-diva-providers`

新增 embedding 能力：

- `embed(texts, model)` trait 方法
- OpenAI-compatible embeddings client
- provider/model 选择与路由
- output dimension 元数据

这是当前最大的基础缺口之一。现在 `providers.yaml` 里有 embedding 模型名，但运行时代码没有 embedding 抽象。

### `agent-diva-tools`

新增 RAG 工具：

- `memory_search`
- `memory_get`
- 后续可选 `knowledge_search`
- 后续可选 `workspace_search`

其中：

- `memory_search` 负责召回 snippet
- `memory_get` 负责按 path + line 取原文

这比单工具直接返回大段文本更稳。

### `agent-diva-agent`

负责：

- prompt policy
- 何时强制先检索
- 会话开始时 warm 索引
- search result 注入策略
- retrieval budget 控制

### 新增 crate 建议：`agent-diva-rag`

建议新增独立 crate，而不是把所有东西塞回 `agent-diva-core`。

职责：

- chunking
- ingestion
- sqlite schema
- FTS query
- vector search
- hybrid merge
- watcher / reindex
- search manager

原因：RAG 很快会长大，不适合堆进 core。

## 8.2 数据与索引模型

建议最小模型如下：

```text
Source File -> Document -> Chunk -> Index Row -> Search Result
```

每个 chunk 至少包含：

- `source_type`: memory | docs | soul | session | command | code_summary
- `path`
- `section`
- `start_line`
- `end_line`
- `text`
- `hash`
- `updated_at`
- `embedding_model`
- `embedding_vector`

## 8.3 检索模式

MVP 到完整体建议分三阶段：

### Phase 1: FTS only

- SQLite
- FTS5
- chunk 化
- keyword recall

优点：

- 不依赖 embedding provider
- 工程复杂度最低
- 已足够替代“整份 MEMORY.md 注入”

### Phase 2: hybrid

- embedding provider trait
- 向量列
- cosine similarity
- weighted merge

### Phase 3: advanced retrieval

- MMR
- temporal decay
- namespace / scope filtering
- extra path source groups
- remote vector backend

## 8.4 召回链路建议

一次标准查询建议走这条链：

1. query normalize
2. source filter decide
3. FTS recall
4. vector recall
5. hybrid merge
6. top-k trim
7. path/line/snippet return
8. `memory_get` 二次精读
9. 注入回答上下文

## 8.5 prompt 侧约束建议

借鉴 `openclaw/extensions/memory-core/index.ts`，建议新增一段固定策略：

- 当问题涉及“过去做过什么、约定、偏好、日期、决策、待办、用户习惯”时
- 必须先调用 `memory_search`
- 若结果命中具体文件，再调用 `memory_get`
- 若结果不足，要明确说“已检查记忆但置信度不足”

这能显著降低模型编造历史。

## 9. 对 agent-diva 的具体实现建议

## 9.1 推荐 MVP

最推荐先做这个版本：

- backend: SQLite
- retrieval: FTS5 only
- sources:
  - `memory/MEMORY.md`
  - `memory/HISTORY.md`
  - `memory/*.md`
  - `AGENTS.md`
  - `docs/**/*.md`
- tools:
  - `memory_search`
  - `memory_get`
- prompt:
  - 历史问题先搜再答

这个版本就已经能显著优于现在的“整份 MEMORY.md 注入”。

## 9.2 第二阶段

补 embedding 与 hybrid：

- 在 `agent-diva-providers` 增加 embeddings API
- 在 `agent-diva-rag` 增加 chunk embedding
- SQLite 增加向量存储
- 实现 hybrid merge

## 9.3 第三阶段

补更强的知识层：

- session transcript 索引
- code summary 索引
- domain-specific indexer
- knowledge graph for decisions/rules

这里更接近 `zeroclaw` 的方向。

## 10. 与现有 memory consolidation 如何协作

不是替换关系，而是上下游关系：

- consolidation 负责把对话压缩成长期记忆材料
- RAG 负责对这些材料做检索与回填

可以理解为：

```text
Conversation -> Consolidation -> Memory Files -> Chunk/Index -> Retrieval -> Prompt Injection
```

所以当前 `agent-diva-agent/src/consolidation.rs` 仍然保留，而且是有价值的。

它的问题不在“要删掉”，而在“下游还没有检索层”。

## 11. 推荐的增量开发顺序

### Step 1

新增 `agent-diva-rag` crate，先只做：

- source scan
- markdown/text chunker
- SQLite chunk store
- FTS search

### Step 2

在 `agent-diva-tools` 增加：

- `memory_search`
- `memory_get`

### Step 3

在 `agent-diva-agent/src/context.rs` 的 system prompt 中加入“先检索再回答”的规则。

### Step 4

在会话启动和 memory 写入后触发增量重建索引。

### Step 5

给 `agent-diva-providers` 增加 embeddings trait 与 OpenAI-compatible 实现。

### Step 6

把检索从 FTS-only 升级为 hybrid。

## 12. 风险与设计注意点

### 1. 不要一开始就把源码全文做向量化

源码体量大、噪声高、更新频繁。更合理的是：

- 先索引文档与 memory
- 再考虑代码摘要
- 必要时按 symbol/README/API 注释做结构化抽取

### 2. 不要只返回大段文本

应该返回：

- 路径
- 行号范围
- 短 snippet
- 分数

否则 token 控制会很差。

### 3. 不要把 retrieval 和 memory file I/O 混成一个工具

`search` 和 `get` 分开，维护性更高，也更安全。

### 4. embedding provider 必须明确区分 chat model 与 embedding model

这一点在 provider 路由上非常关键，尤其是未来如果接 OpenAI-compatible、LiteLLM 或 native provider。

## 13. 最终判断

### 对三个参考项目的判断

- `openclaw`：完整 memory-RAG，工程化程度最高。
- `zeroclaw`：完整 memory 系统 + 专用 datasheet RAG + knowledge graph，Rust 侧最值得参考。
- `nanobot`：文件型记忆，不是完整 RAG。

### 对 agent-diva 的判断

`agent-diva` 现在没有完整 RAG，但它已经具备三块很重要的前置基础：

- consolidation
- memory files
- tools framework

因此最合理的路线不是重写 memory，而是：

> 保留现有 consolidation，把 `agent-diva` 从“文件记忆系统”升级成“可检索的 memory/workspace RAG 系统”。

### 推荐路线

优先级最高的落地方向是：

1. `SQLite + FTS5` 的最小检索系统
2. `memory_search` / `memory_get` 工具
3. prompt 侧强制 recall policy
4. embedding + hybrid 作为第二阶段

如果要一句话概括：

> `agent-diva` 最适合走“openclaw 的工具化检索分层 + zeroclaw 的 Rust memory/backend 抽象 + 自己现有 consolidation 基础”的组合路线。
