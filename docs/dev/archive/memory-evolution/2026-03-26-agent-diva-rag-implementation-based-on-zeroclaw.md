# 参考 `.workspace/zeroclaw` / `.workspace/openclaw` 为 `agent-diva` 实现记忆检索能力

## 1. 目标

本文聚焦一个具体问题：

> 参考 `.workspace/zeroclaw` 与 `.workspace/openclaw` 的现有实现，为 `agent-diva` 设计一条可落地、符合当前 Rust workspace 架构的“结构化记忆 + 检索”能力建设路径。

这里的 RAG 不指“把 `MEMORY.md` 全量塞进 prompt”，而指一条完整链路：

- 可持续写入的知识源
- 可增量更新的索引
- 查询时的召回与排序
- 工具化的显式检索
- 将结果以小体积、可追溯的方式注入模型上下文

同时，本方案补充三个约束：

- 新能力尽量不要影响现有基于 `MEMORY.md` / `HISTORY.md` 的旧行为
- `MEMORY.md` 保留，但不再作为唯一主记忆来源
- 命名上优先强调“memory/记忆能力”，而不是一开始就强调“RAG”

## 1.1 命名建议：优先考虑 `agent-diva-memory`

如果从长期演进和“不要过早暴露实现细节”的角度看，`agent-diva-memory` 比 `agent-diva-rag` 更稳妥。

原因：

- `memory` 描述的是领域职责，`rag` 更像实现方式
- 第一阶段核心是“结构化记忆 + recall + 兼容旧 Markdown”，而不是专用知识库平台
- 后续如果包里继续加入：
  - `MEMORY.md` 兼容层
  - consolidation 持久化
  - FTS / embedding / hybrid recall
  - 专用索引器
  - 经验图谱
  那么 `memory` 这个名字比 `rag` 更不容易过时

因此本文后续建议统一调整为：

- 新 crate 名称：`agent-diva-memory`
- 其中的检索子模块可命名为：
  - `retrieval`
  - `index`
  - `embedding`
  - `workspace_rag`

换句话说：

> `RAG` 应该是 `agent-diva-memory` 里的一个能力子集，而不是整个包名。

## 2. 先看 `zeroclaw` 已经做了什么

从 `.workspace/zeroclaw` 的源码看，它不是只有一个“记忆文件”，而是已经形成了三层能力。

### 2.1 通用长期记忆抽象

`src/memory/traits.rs` 定义了统一的 `Memory` trait，核心能力包括：

- `store`
- `recall`
- `get`
- `list`
- `forget`
- `count`
- `store_with_metadata`
- `recall_namespaced`

它对应的数据结构 `MemoryEntry` 已经具备这些关键字段：

- `key`
- `content`
- `category`
- `session_id`
- `namespace`
- `importance`
- `score`
- `superseded_by`

这意味着 `zeroclaw` 的记忆不是“一个大 Markdown 文件”，而是“结构化条目 + 可替换后端”。

### 2.1.1 `MEMORY.md` 在 `zeroclaw` 里的真实定位

进一步看 `.workspace/zeroclaw/src/channels/mod.rs`，可以发现它做的是：

- `AGENTS.md` / `SOUL.md` / `TOOLS.md` / `IDENTITY.md` / `USER.md` 注入
- `MEMORY.md` 也会注入
- 但 `memory/*.md` 每日日志不会全量注入
- 这些 daily memory 倾向于通过 `memory_recall` / `memory_search` 按需访问

这说明 `zeroclaw` 并不是“只有 `MEMORY.md`”，而是：

- `MEMORY.md` 作为 curated bootstrap memory
- 结构化 memory backend 作为主检索引擎
- recall tool 作为查询时入口

这个分层很值得 `agent-diva` 借鉴。

### 2.2 检索流水线

`src/memory/retrieval.rs` 提供了 `RetrievalPipeline`，核心思想是多阶段召回：

1. 热缓存
2. FTS/BM25
3. 向量检索
4. 混合结果返回

它还有两个非常实用的设计：

- 热缓存 TTL 与容量控制，避免重复 query 的开销
- FTS 高分 early return，降低无意义的向量检索成本

这对 `agent-diva` 很重要，因为当前项目既有 CLI，又有服务层和 GUI，检索延迟不能过大。

### 2.3 SQLite 本地脑

`src/memory/sqlite.rs` 基本就是一个本地可持久化“脑”：

- `memories` 主表
- `memories_fts` FTS5 虚表
- `embedding_cache`
- `WAL` 模式
- schema migration
- hybrid merge 所需的 embedding 存储

它说明一件事：对桌面/本地 agent 来说，SQLite 是实现第一版 RAG 的合理底座，不需要一开始就上远程向量库。

### 2.4 工具化 recall

`src/tools/memory_recall.rs` 把 recall 暴露成一个显式工具：

- 输入 query / since / until / limit
- 输出带 score 的结构化结果
- 允许 agent 在“需要回忆”时主动检索

这个能力比“只在系统提示词里注入记忆”更可靠，因为它把“是否检索”从隐式 prompt 约束变成了可观测行为。

## 2.7 再看 `openclaw`：`MEMORY.md` 不是主存储，但仍是重要兼容层

`.workspace/openclaw` 的设计给了另一个非常重要的参考：

### 2.7.1 `MEMORY.md` 依然被保留为 bootstrap/reference 文件

从 `.workspace/openclaw/src/agents/workspace.ts` 可以看到：

- 默认文件名仍然是 `MEMORY.md`
- 同时兼容小写 `memory.md`
- 系统会优先使用 `MEMORY.md`，仅在不存在时回退到 `memory.md`

这说明成熟系统并不会急着删除 Markdown 记忆文件，而是保留它作为：

- 用户可读
- 用户可编辑
- 首轮上下文可用
- 向后兼容

的一层接口。

### 2.7.2 `openclaw` 的主路径已经转向 `memory_search`

从 `.workspace/openclaw/src/agents/tools/memory-tool.ts` 与配置帮助文档看，`openclaw` 更强调：

- `memory_search` 是语义检索入口
- 检索来源默认是 `MEMORY.md + memory/*.md`
- 可选把 session transcript 也纳入索引
- 检索结果返回 snippet、path、lines，而不是整份 memory 文件

这意味着 `openclaw` 的思路不是“取消 `MEMORY.md`”，而是：

> 保留 `MEMORY.md` 作为可读可编辑的默认记忆源，但主 recall 行为已经交给索引和检索工具。

这正好符合你提出的要求：

- 新特性不应破坏旧机制
- `MEMORY.md` 继续存在
- 但它不再是唯一且主要的记忆系统

### 2.5 专用 RAG 而不是只有统一知识库

`src/rag/mod.rs` 是一个面向 datasheet 的专用 RAG：

- 面向目录 ingestion
- markdown/txt/pdf 分块
- pin alias 解析
- board-aware 检索

这给 `agent-diva` 一个很关键的启发：

> 不要把所有知识都混进一个统一 memory 库，应该允许按领域做专用索引器。

### 2.6 知识图谱是增强项，不是第一阶段前置条件

`src/tools/knowledge_tool.rs` 说明 `zeroclaw` 还尝试把模式、决策、lesson learned、expert 变成图谱节点和边。

对 `agent-diva` 来说，这一层很有价值，但不应该作为第一阶段的前置条件。第一阶段先把“可检索记忆”做好，第二阶段再考虑“结构化经验图谱”。

## 3. `agent-diva` 当前差距

对照当前仓库实现：

- `agent-diva-core/src/memory/manager.rs`
- `agent-diva-agent/src/context.rs`

当前能力主要是：

- `MEMORY.md`
- `HISTORY.md`
- 每日日志文件
- consolidation 后写回 Markdown
- 构造 prompt 时把 `MEMORY.md` 全量注入

它的问题比较明确：

### 3.1 没有真正的索引层

当前长期记忆主要是文件，不是结构化条目，没有：

- FTS
- embedding
- hybrid recall
- namespace / domain 隔离

### 3.2 没有 query-time recall

现在的“回忆”本质上是：

- 先把旧对话压缩进 `MEMORY.md`
- 然后每轮把整个 `MEMORY.md` 再塞给模型

这不是 RAG，而是“大记忆文件 prompt 注入”。

### 3.3 没有工具层闭环

当前工具集中没有：

- `memory_search`
- `memory_get`
- `memory_store`

所以 agent 无法在回答前显式执行“先检索，再引用，再回答”。

### 3.4 没有领域拆分

当前 `agent-diva` 的 memory 基本混在一起，不区分：

- 用户长期偏好
- 项目规则
- 工作区文档
- 历史任务决策
- 专用知识库

这会导致未来随着规模增长，召回质量快速下降。

## 4. 适合 `agent-diva` 的目标架构

建议采用“`zeroclaw` 式结构化 memory + `openclaw` 式 Markdown 兼容层”双轨设计，但按 `agent-diva` 的 crate 边界重新落位。

### 4.1 crate 责任划分

#### `agent-diva-memory`

负责“结构化记忆域模型 + 存储抽象 + 本地索引后端 + recall pipeline”：

- `MemoryRecord`
- `MemoryScope`
- `MemoryDomain`
- `MemoryStore` trait
- `SqliteMemoryStore`
- `RetrievalPipeline`
- `EmbeddingProvider` trait
- schema migration
- 兼容导入器

这一层只负责“存什么、怎么查”，不负责 prompt 拼装。

#### `agent-diva-core`

继续保留现有 `MemoryManager`，但职责收敛为 Markdown 兼容层：

- `MEMORY.md` / `HISTORY.md` 文件管理
- daily note 管理
- legacy context 读取
- 旧功能开关与回退逻辑

#### `agent-diva-agent`

负责“什么时候检索、如何注入上下文”：

- `MemoryLoader`
- recall policy
- consolidation 输出到 `MemoryStore`
- `ContextBuilder` 只注入 recall 结果，不再注入整个 `MEMORY.md`

这一层对应 `zeroclaw` 的 recall 驱动 prompt 组装逻辑。

#### `agent-diva-tools`

负责给 agent 暴露显式工具：

- `memory_search`
- `memory_get`
- 可选 `memory_store`
- 第二阶段可加 `knowledge_search`

这一层是让 agent 真正“会用 RAG”的关键。

#### `agent-diva-providers`

负责 embedding provider 抽象，不和 chat provider 强耦合：

- 本地/远程 embedding 调用
- 维度声明
- model id 透传

这里要遵守仓库已有规则：如果对接 provider 原生 OpenAI-compatible endpoint，发送原始 model id，不做 LiteLLM 风格前缀改写。

#### `agent-diva-manager` / `agent-diva-gui`

第一阶段不是必需，但建议预留：

- 查看索引状态
- 手动触发 reindex
- 观察 recall 命中结果

### 4.2 数据分层

建议把第一阶段的数据源拆成四类，而不是直接把所有内容扔进一个表里。

#### A. Core Memory

来自：

- 用户偏好
- 已确认的项目规则
- agent 与用户之间稳定约定

特点：

- 高价值
- 数量少
- 高权重召回

#### B. Session/Event Memory

来自：

- consolidation 提炼出的关键任务进度
- 决策摘要
- 卡点与待办

特点：

- 和时间强相关
- 需要衰减
- 适合混合检索

#### C. Workspace Knowledge

来自：

- `AGENTS.md`
- `README`
- `docs/`
- 代码注释、设计文档、命令说明

特点：

- 更像工作区知识库
- 不应和“用户记忆”完全混存
- 更适合 namespace/domain 检索

#### D. Domain RAG Index

对应 `zeroclaw` 的 `src/rag/mod.rs` 思路，面向特定目录构建专用索引，例如：

- `docs/specs/`
- `docs/prd/`
- `knowledge/`
- 将来可能的 `memory/rational/`、`memory/emotional/`

特点：

- 分块策略可定制
- 检索规则可定制
- 不必和通用 memory 共用一套 rank 逻辑

## 5. 第一版推荐实现

### 5.0 第一原则：新能力默认“旁路接入”，不能破坏旧功能

如果你的优先级是“现在还不能影响原来的功能”，那么第一阶段必须采用旁路式接入，而不是替换式接入。

具体原则：

1. 保留现有 `MemoryManager`
2. 保留 `MEMORY.md` / `HISTORY.md` 读写语义
3. 新的 `agent-diva-memory` 默认只做增量写入和可选 recall
4. 旧 prompt 逻辑先不删除，只做可配置切换
5. 只有在 recall 链路稳定后，才把 system prompt 的主注入源从整份 `MEMORY.md` 切到 top-k recall

也就是说，第一阶段目标不是“替换旧功能”，而是：

> 在不破坏旧流程的前提下，为 `agent-diva` 增加一条新的结构化检索路径。

### 5.0.1 建议采用双开关策略

为避免新能力侵入旧行为，建议配置上拆成两个开关：

- `memory.indexing_enabled`
- `memory.recall_injection_enabled`

默认策略建议：

- `indexing_enabled = true`
- `recall_injection_enabled = false`

这样第一阶段可以做到：

- consolidation 继续照旧写 Markdown
- 同时旁路写入 `agent-diva-memory`
- 工具层可以手动使用 `memory_search`
- 但主 prompt 仍沿用旧逻辑

等验证完成后，再单独开启 `recall_injection_enabled`。

### 5.1 最小可行目标

先不要追求“全功能知识平台”，第一版只做下面这条闭环：

1. 结构化存储长期记忆
2. 提供 SQLite FTS 检索
3. 可选 embedding 检索
4. 暴露 `memory_search` 工具
5. `ContextBuilder` 改为注入 top-k recall 结果

只要这 5 点闭环成立，`agent-diva` 就从“memory 文件注入”升级为真正的基础 RAG。

### 5.2 第一版数据模型

建议在 `agent-diva-memory` 中引入：

```rust
pub struct MemoryRecord {
    pub id: String,
    pub key: String,
    pub title: Option<String>,
    pub content: String,
    pub domain: MemoryDomain,
    pub scope: MemoryScope,
    pub session_key: Option<String>,
    pub source_path: Option<String>,
    pub tags: Vec<String>,
    pub importance: f32,
    pub score: Option<f32>,
    pub created_at: String,
    pub updated_at: String,
}
```

建议第一版 `MemoryDomain` 至少包括：

- `Core`
- `Session`
- `Workspace`
- `Daily`
- `Custom(String)`

建议第一版 `MemoryScope` 至少包括：

- `Global`
- `Workspace`
- `Session`
- `User`

这比当前单个 `MEMORY.md` 更适合后续演进，而且能和 `zeroclaw` 的 category / session / namespace 思想对齐。

### 5.3 第一版 SQLite 设计

可以直接借鉴 `zeroclaw` 的 SQLite 思路，但做一次适度收敛。

同时要明确：

- SQLite 结构化 store 是新主存储
- `MEMORY.md` 是兼容层和人工编辑层
- 两者短期并存，不做“谁覆盖谁”的激进切换

建议表：

- `memory_records`
- `memory_records_fts`
- `embedding_cache`

建议字段：

- `id`
- `key`
- `title`
- `content`
- `domain`
- `scope`
- `session_key`
- `source_path`
- `tags_json`
- `importance`
- `embedding`
- `created_at`
- `updated_at`

建议特性：

- `WAL`
- FTS5 trigger 同步
- schema migration
- embedding cache

### 5.4 RetrievalPipeline 设计

建议直接复用 `zeroclaw` 的分阶段思想，但接口更贴近 `agent-diva`：

```rust
pub struct RecallRequest<'a> {
    pub query: &'a str,
    pub limit: usize,
    pub session_key: Option<&'a str>,
    pub domains: Option<&'a [MemoryDomain]>,
    pub scopes: Option<&'a [MemoryScope]>,
}
```

召回顺序建议：

1. 热缓存
2. FTS
3. embedding
4. hybrid merge
5. importance / recency rerank

其中：

- `Core` domain 固定加权
- `Session` domain 施加轻度时间衰减
- `Workspace` domain 更依赖 query term overlap

## 6. 工具与上下文注入策略

### 6.1 新增工具

第一阶段建议至少新增两个工具。

#### `memory_search`

输入：

- `query`
- `limit`
- `domain`
- `session_key`

输出：

- `id`
- `key`
- `domain`
- `score`
- `summary/snippet`
- `source_path`

用途：

- 回忆用户偏好
- 查询项目历史决策
- 查找之前做过的任务

#### `memory_get`

输入：

- `id`

输出：

- 完整内容
- 元数据

用途：

- 在 `memory_search` 命中后，拉取高价值条目原文
- 避免搜索结果直接塞入过多内容

`zeroclaw` 只有 recall 工具，但 `agent-diva` 更适合拆成 search/get 两段，这样更利于控制 token 和 UI 展示。

### 6.2 `ContextBuilder` 的改造原则

当前 `agent-diva-agent/src/context.rs` 会把 `MEMORY.md` 全量注入。考虑“不影响旧功能”的要求，建议分两步做。

#### 第一步：保持旧注入不变，只增加 recall 工具

先做：

1. 保留 `MEMORY.md` 现有注入
2. 新增 `memory_search` / `memory_get`
3. consolidation 同时写 Markdown 与结构化 store
4. 观察 recall 命中质量

#### 第二步：再切换 prompt 主路径

等第一步稳定后，再改成：

1. 保留 `AGENTS.md` / `SOUL.md` / `IDENTITY.md` / `USER.md`
2. 将整份 `MEMORY.md` 注入降级为可选兼容模式
3. 改为在构建 system prompt 时调用 `MemoryLoader`
4. 只注入 top 3 到 top 7 条 recall 结果

注入格式建议：

```markdown
## Relevant Memory
- [core] user_language: 用户偏好中文
- [workspace] provider-model-id-safety: 原生 provider endpoint 不要自动加 LiteLLM 前缀
- [session] provider-routing-followup: 需要补 outbound model 值断言测试
```

### 6.3 什么时候强制 recall

如果当前 query 命中以下场景，应强制做 recall：

- “我们之前说过什么”
- “我偏好什么”
- “这个项目之前怎么定的”
- “上次做到哪了”
- “这个仓库有什么约束”

也就是说，RAG 不应只靠模型自己决定是否搜索，系统层要给出 recall policy。

### 6.4 关于 `MEMORY.md` 的最终定位

综合 `zeroclaw` 与 `openclaw`，更合理的结论是：

- `MEMORY.md` 不应该被删除
- `MEMORY.md` 也不应该继续承担全部记忆职责
- `MEMORY.md` 适合定位为：
  - bootstrap memory
  - curated memory
  - human-editable compatibility layer

而真正的主 recall 来源应当是：

- 结构化 store
- FTS / embedding 索引
- 显式 recall 工具

## 7. 专用 RAG 的落地方式

`zeroclaw` 最值得借鉴的，不只是 `Memory` trait，而是“专用索引器”思维。

对 `agent-diva`，建议第二阶段补一个 `WorkspaceRagIndexer`，面向指定目录做 chunk + index：

- `docs/`
- `commands/`
- 未来专门的 `knowledge/`

这层不要与用户记忆完全混用，建议单独 namespace，例如：

- `workspace_docs`
- `project_rules`
- `release_notes`

这样可以避免“用户偏好”和“项目文档”在同一个结果集中互相污染。

## 8. 推荐实施阶段

### Phase 0：兼容层准备

目标：

- 保留 `MEMORY.md` / `HISTORY.md`
- 新增结构化 memory store，不立即删除旧机制
- 新能力默认不改变现有 prompt 行为

工作：

- 新建 `agent-diva-memory`
- `agent-diva-core` 保持 Markdown 兼容职责
- consolidation 同时写 Markdown 和 SQLite
- 增加 migration 与最小测试

### Phase 1：基础 recall 闭环

目标：

- 有结构化 recall
- 有 `memory_search`
- 但默认 prompt 仍可继续依赖整个 `MEMORY.md`

工作：

- `agent-diva-agent` 新增 `MemoryLoader`
- `agent-diva-tools` 新增 `memory_search` / `memory_get`
- `ContextBuilder` 先增加 recall 可选注入模式
- 增加 CLI smoke test

### Phase 1.5：切换主注入路径

目标：

- 在验证通过后，再把主 prompt 从整份 `MEMORY.md` 切到 top-k recall

工作：

- 默认开启 recall 注入
- 将整份 `MEMORY.md` 注入改为兼容开关
- 比较旧行为与新行为的回答稳定性

### Phase 2：workspace knowledge RAG

目标：

- 对 `docs/`、规则文档、项目说明进行独立索引

工作：

- 实现目录扫描、chunk、增量 reindex
- 增加 namespace/domain 过滤
- 加入 source path 与 snippet 引用

### Phase 3：embedding 与 hybrid rerank

目标：

- 解决纯关键词召回不足

工作：

- `agent-diva-providers` 新增 embedding provider abstraction
- SQLite 存储 embedding blob
- 增加 hybrid merge、importance、recency 权重

### Phase 4：结构化经验图谱

目标：

- 沉淀决策、模式、专家经验

工作：

- 参考 `zeroclaw` `knowledge_tool`
- 先做 architecture decision / lesson learned 两种节点
- 暴露独立工具，不和基础 memory 紧耦合

## 9. 与当前架构的具体映射

### 9.1 `agent-diva-memory`

建议新增模块：

- `agent-diva-memory/src/store.rs`
- `agent-diva-memory/src/sqlite_store.rs`
- `agent-diva-memory/src/retrieval.rs`
- `agent-diva-memory/src/types.rs`
- `agent-diva-memory/src/compat.rs`

### 9.2 `agent-diva-core`

保留现有：

- `agent-diva-core/src/memory/manager.rs`

但让 `manager.rs` 从“唯一记忆入口”降级为“兼容 Markdown 文件管理器”。

### 9.3 `agent-diva-agent`

建议新增模块：

- `agent-diva-agent/src/memory_loader.rs`

并修改：

- `agent-diva-agent/src/context.rs`
- `agent-diva-agent/src/consolidation.rs`

### 9.4 `agent-diva-tools`

建议新增：

- `agent-diva-tools/src/memory_search.rs`
- `agent-diva-tools/src/memory_get.rs`

如果仓库已有统一工具注册表，则同步注册 schema 与 help 文案。

### 9.5 `agent-diva-manager` / `agent-diva-gui`

建议后续增加：

- 查看 recall 结果
- 手动重建索引
- 显示 memory backend health

但这不是第一阶段阻塞项。

## 10. 验证与测试建议

建议最少覆盖下面几类测试。

### 10.1 `agent-diva-core`

- 存储 / 读取 / 删除 memory record
- FTS 查询命中
- session/domain/scope 过滤
- migration 回归
- embedding cache 行为

### 10.2 `agent-diva-agent`

- recall 命中时 prompt 只注入 top-k 结果
- recall 为空时不注入空段落
- consolidation 能同时写 Markdown 与结构化 memory

### 10.3 `agent-diva-tools`

- `memory_search` 参数校验
- `memory_search` 返回结构化结果
- `memory_get` 能取回完整条目

### 10.4 smoke test

至少补一个真实路径测试，例如：

- 先写入一条 memory
- 再通过 CLI 发起一个依赖该 memory 的问题
- 观察 agent 是否通过 recall 给出正确回答

## 11. 最终建议

如果只给一句结论，我的建议是：

> `agent-diva` 不要继续扩展“基于 `MEMORY.md` 的全量注入方案”，但也不要急着删除它；更合理的路线是参考 `zeroclaw` 与 `openclaw`，建立 `agent-diva-memory + RetrievalPipeline + memory_search tool + 渐进式 prompt 切换` 这条兼容闭环。

最值得直接借鉴的部分是：

- `Memory` trait 思维
- SQLite + FTS5 本地脑
- staged retrieval pipeline
- recall 工具化
- 专用 RAG 索引器
- `MEMORY.md` 作为兼容/人工编辑层继续保留

最不建议第一阶段就照搬的部分是：

- 知识图谱全量建设
- 复杂多后端矩阵
- 过早引入远程向量库
- 直接删除旧的 `MEMORY.md` 注入逻辑

先把“兼容旧功能的结构化记忆能力”做起来，再逐步把主 recall 流量从 `MEMORY.md` 转移到结构化 store，这条路线更符合 `agent-diva` 当前代码体量和交付节奏。
