## Agent-Diva 记忆架构设计（Zeroclaw 风格）

> **版本**: v1.0  
> **日期**: 2026-03-03  
> **范围**: 基于 Zeroclaw 架构，为 `agent-diva` 设计一套面向长期演进的记忆（Memory）与上下文管理架构。

---

## 1. 背景与目标

在当前 Rust 版本的 `agent-diva` 中：

- 已经具备基础的 **会话持久化** 能力（`sessions/<safe_key>.jsonl`），以及 **长期记忆文件**（`MEMORY.md` / `HISTORY.md`）；
- 通过 **consolidation 流程**，会周期性地把较旧的会话历史总结到 `MEMORY.md` 中；
- `ContextBuilder` 在每轮调用 LLM 时，会将 **SOUL/AGENTS/IDENTITY/USER + MEMORY + 最近 50 条历史** 拼接成一次完整的 prompt。

这一设计已经可以支撑日常使用，但存在几个问题：

- **记忆粒度过粗**：`MEMORY.md` 越写越大，每轮都会被整体注入 system prompt；  
- **检索不够精准**：缺乏“按当前对话内容检索少量相关记忆”的主动召回层；  
- **会话与记忆耦合度偏高**：consolidation 是从会话走向 MEMORY 的单向通道，回读路径依赖整文件注入。

相比之下，Zeroclaw 在上下文管理上有几个鲜明特点：

- 将 **会话历史（Session history）** 与 **长期记忆（Memory store）** 明确分层；
- Memory 使用 **SQLite + FTS5 + 向量嵌入** 实现混合检索；
- 每轮对话前通过 `MemoryLoader` 主动召回少量高相关记忆，拼成一个短小的 `[Memory context]` 段落注入 prompt；
- 会话历史通过 `max_messages + TTL` 控制长度，不依赖复杂的 session store。

本设计文档的目标是：

- **复用 Zeroclaw 的记忆层思想**（Memory + MemoryLoader + Hybrid Search）；  
- 在不引入过度复杂 session store 的前提下，使 `agent-diva` 拥有一套 **高性能、可演进的记忆架构**；  
- 为后续与 OpenClaw 式 Session/Reset/Gateway 设计对齐预留空间。

---

## 2. Zeroclaw 记忆与上下文管理回顾（摘要）

本节简要回顾 Zeroclaw 的关键设计，仅保留与记忆架构直接相关的部分，作为后文设计的参考基线。

### 2.1 三层上下文分层

在 Zeroclaw 中，整体上下文被拆成三个层次：

- **会话历史（Session history）**  
  - 由 `SessionManager` 维护，一个 `session_id` 对应一组 `ChatMessage`；  
  - 支持内存或 SQLite 后端，带 `max_messages` 限制与 TTL 清理；  
  - 不存储 system prompt。

- **长期记忆（Memory store）**  
  - 由 `Memory` trait 抽象，典型实现为 `SqliteMemory`；  
  - 使用 SQLite + FTS5 + 向量嵌入存储“事实/偏好/日志”等；  
  - 支持按 `category` 和 `session_id` 进行作用域划分。

- **系统 Prompt（System prompt）**  
  - 由 `SystemPromptBuilder` 组装多个 section：Identity / Tools / Skills / Workspace / Runtime / Memory 等；  
  - Memory 部分由 `MemoryLoader` 生成的 `[Memory context]` 段落填充。

### 2.2 Memory 抽象与存储

- `Memory` trait 定义典型接口：
  - `store(key, content, category, session_id)`  
  - `recall(query, limit, session_id)`  
  - `get(key)` / `list(category, session_id)` / `forget(key)` / `count()` / `reindex()`。
- 底层使用 SQLite：
  - 主表 `memories` 存储 key / content / category / embedding / session_id / 时间戳；
  - FTS5 虚表 + 触发器实现全文检索；
  - embedding 缓存表避免重复计算。

### 2.3 MemoryLoader：从“脑”到 Prompt

`DefaultMemoryLoader` 的关键行为：

- 输入：当前用户消息（和可选 session 上下文）；  
- 查询：调用 `memory.recall(query, limit * OVER_FETCH, session_id)`，使用混合检索得到候选集合；  
- 重排：
  - 对非核心（如 Daily/Conversation）记忆做时间衰减；  
  - 对 `Core` 类目加权加分；  
  - 丢弃低于 `min_relevance_score` 的记忆；  
  - 最终取前 `limit` 条。
- 输出：一个简短、结构化的文本段落，例如：

```markdown
[Memory context]
- user_name: Alice
- user_pref_lang: 简体中文
- project_main_repo: agent-diva
```

这一机制确保：

- **每轮上下文中记忆部分体积恒定且可控**；  
- 记忆注入由“当前 query 驱动”，而非被动地把所有长期记忆塞给模型。

---

## 3. Agent-Diva 当前记忆与上下文现状

本节只摘录与记忆直接相关的现状，用于对比 Zeroclaw 方案。

### 3.1 会话与上下文构建

- `agent-diva-core` 中：
  - `SessionManager` 基于 `sessions/<safe_key>.jsonl` 管理会话消息列表；  
  - `Session::get_history(max_messages)` 返回最近 N 条未合并消息（默认 50 条左右），并保证从第一个 user 消息开始。

- `agent-diva-agent` 中：
  - `ContextBuilder::build_messages`：
    - 构造 system prompt：读取 `SOUL.md` / `AGENTS.md` / `IDENTITY.md` / `USER.md` / `MEMORY.md` / skills 等；  
    - 追加最近会话历史（`Session::get_history`）；  
    - 追加当前用户消息。

### 3.2 长期记忆与 consolidation

- `agent-diva-core::memory::MemoryManager`：
  - 将长期记忆存放于 `memory/MEMORY.md`；  
  - 历史记录存放于 `memory/HISTORY.md`。

- `agent-diva-agent::consolidation`：
  - 当某个会话未合并消息数超过 `memory_window`（默认 100）时：  
    - 取旧的一半消息；  
    - 用一个专门的 LLM 调用生成 `memory_update` 与 `history_entry`；  
    - 写入 `MEMORY.md` 与 `HISTORY.md`；  
    - 更新 `session.last_consolidated` 以避免重复处理。

- 在构建 system prompt 时，`MemoryManager::get_memory_context()` 会：
  - 直接把 `MEMORY.md` 的全部文本作为一个 `## Long-term Memory` 段落注入。

### 3.3 问题归纳

与 Zeroclaw 相比，主要差异与问题集中在：

- **记忆存储形态**：仅为 Markdown 文件，缺少结构化索引与检索能力；  
- **记忆注入策略**：每轮全量注入 MEMORY，缺乏“主动召回 + 精简注入”的 MemoryLoader 层；  
- **会话与记忆边界**：consolidation 是单向“会话 → MEMORY”，回读时无法按 query/类别/时间做细粒度选择。

---

## 4. 目标记忆架构（Zeroclaw 风格）

本节给出面向 `agent-diva` 的目标架构，尽可能在不破坏现有使用体验的前提下，引入 Zeroclaw 风格记忆层。

### 4.1 顶层设计目标

- **分层清晰**：
  - 会话层（Session history）：负责“这轮对话里的最近若干轮历史”；  
  - 记忆层（Memory store）：负责长期事实、偏好与事件；  
  - Prompt 层（Context builder）：负责在每轮请求时，以最少 tokens 注入最有价值的上下文。

- **注入精简**：
  - 每次只注入 **少量高相关记忆**（例如 3~7 条）；  
  - Memory 一律通过 MemoryLoader 召回，而非全量堆入。

- **存储可演进**：
  - 初期可以基于 SQLite 实现本地 `brain.db`；  
  - 将来可以扩展为远程向量库或多租户存储，而不影响上层接口。

### 4.2 目标架构分层示意

```mermaid
flowchart TD
    subgraph SessionLayer[会话层]
        SMan[SessionManager\nsessions/<safe_key>.jsonl]
    end

    subgraph MemoryLayer[记忆层]
        MemStore[MemoryStore\n(e.g. SqliteMemory)]
        Loader[MemoryLoader\n(query -> Memory context)]
    end

    subgraph PromptLayer[Prompt 构建层]
        CB[ContextBuilder]
    end

    SMan --> CB
    MemStore --> Loader --> CB
```

关键点：

- `SessionManager` 仍负责会话历史与 consolidation；  
- 新增 `MemoryStore` 与 `MemoryLoader` 两个抽象：  
  - consolidation 输出不再只写 Markdown，而是（或同时）写入 `MemoryStore`；  
  - `ContextBuilder` 不再直接塞入整个 MEMORY，而是通过 `MemoryLoader` 获取少量记忆文本。

---

## 5. 数据模型与接口设计

本节按照从下到上的顺序，描述预期的数据模型与接口。

### 5.1 MemoryEntry 与 MemoryCategory

在 `agent-diva-core` 中引入类似 Zeroclaw 的基础类型：

- `MemoryEntry`：
  - `id: String`（UUID 或稳定 key）；  
  - `key: String`（逻辑键，如 `user_lang`、`project_main_repo`）；  
  - `content: String`（纯文本内容）；  
  - `category: MemoryCategory`；  
  - `session_key: Option<String>`（与会话绑定的记忆，可选）；  
  - `created_at` / `updated_at`。

- `MemoryCategory`（建议初始值）：
  - `Core`：长期事实、偏好与配置（如用户语言偏好、主要项目）；  
  - `Daily`：每日摘要/日志；  
  - `Conversation`：与特定会话强相关的事实；  
  - `System`：内部用途（consolidation 生成的 technical entry 等）；  
  - `Custom(String)`：未来扩展。

### 5.2 MemoryStore 抽象

在 `agent-diva-core` 中引入 `MemoryStore` trait，覆盖最小必要操作：

- `store(entry: MemoryEntry) -> Result<()>`  
- `recall(query: &str, limit: usize, session_key: Option<&str>) -> Result<Vec<MemoryEntry>>`  
- `get(key: &str) -> Result<Option<MemoryEntry>>`  
- `forget(key: &str)` / `forget_by_session(session_key: &str)`  
- `count()` 等统计接口。

第一版实现建议使用本地 SQLite（例如 `memory/brain.db`），表结构可参考 Zeroclaw，但可以从最小子集开始：

- 单表 `memories`：
  - `id TEXT PRIMARY KEY`  
  - `key TEXT`  
  - `content TEXT`  
  - `category TEXT`  
  - `session_key TEXT NULL`  
  - `created_at INTEGER`  
  - `updated_at INTEGER`

第二阶段再增加：

- FTS5 虚表用于全文检索；  
- 可选 embedding 列与向量检索（如采用本地 embedding 模型或外部服务）。

### 5.3 MemoryLoader 抽象

在 `agent-diva-agent` 中增加 `MemoryLoader` 抽象，负责将 MemoryEntry 转为可注入 prompt 的文本：

- 输入：
  - `current_user_message: &str`；  
  - 可选 `session_key: &str` 与近期 history 片段摘要。

- 行为：
  - 调用 `MemoryStore::recall` 获取候选记忆；  
  - 对非核心记忆做时间衰减（可选）；  
  - 对特定类别（如 Core）加权加分；  
  - 按得分降序，截取前 `limit` 条（如 5 条）；  
  - 过滤过短或噪声记忆。

- 输出：一段 Markdown 文本，例如：

```markdown
## Long-term Memory Context
- user_name: Alice
- user_language: 简体中文
- favorite_tech_stack: Rust + Vue
```

接口可以设计为：

- `build_memory_context(user_message, session_key) -> Option<String>`  
  - 若无足够高相关记忆，则返回 `None`。

### 5.4 与 consolidation 的集成

现有 consolidation 已经在做“从会话历史中总结出长期记忆”的工作，可以做如下调整：

- 将 consolidation 的输出结构调整为：
  - `memory_update: Vec<MemoryEntry>`（而非单一 Markdown 段落）；  
  - `history_entry: String`（仍可追加到 HISTORY.md 中，用于人工回顾）。

- consolidation 在落盘时：
  - 继续维护 `MEMORY.md`（保持向后兼容）；  
  - 同时调用 `MemoryStore::store` 写入结构化记忆；
  - 对于重要偏好类信息（如“用户喜欢用中文回答”）使用稳定 key（如 `user_lang`）。

这样，在过渡阶段：

- 旧的 `MEMORY.md` + 新的 `MemoryStore` 并行存在；  
- Prompt 构建可以逐步从“全量 MEMORY 注入”迁移到“MemoryLoader 召回 + 少量 MEMORY 兜底”。

---

## 6. Prompt 构建与上下文注入策略

在引入记忆层之后，`ContextBuilder` 需要做出相应调整，使得每轮调用的上下文结构更接近 Zeroclaw。

### 6.1 新的上下文组成

目标形态：

1. **System Prompt（静态/慢变部分）**  
   - SOUL / AGENTS / IDENTITY / USER / TOOLS / WORKSPACE / RUNTIME 等；  
   - MEMORY.md（可选，仅摘要或部分片段）；  
   - 尽量缓存静态部分，避免每轮重复拼接。

2. **Memory Context（动态召回部分）**  
   - 由 `MemoryLoader` 基于当前 user message + session_key 召回；
   - 条数与长度均受限（如不超过 500 tokens）。

3. **Session History（短期对话历史）**  
   - 由 `Session::get_history_token_aware(max_tokens)` 返回的最近若干轮；  
   - 建议升级为按 token 预算而非“消息条数”的裁剪。

4. **Current User Message**  
   - 本轮用户输入。

### 6.2 token-aware 的历史裁剪

为避免历史与记忆上下文抢占模型上下文窗口，建议：

- 在 `ContextBuilder` 中引入简单的 token 估算器（按字符数近似即可）；  
- 在构建 messages 时：
  - 为 system + memory context 预留固定 token（例如总 4k 上下文中预留 1k~1.5k）；  
  - 对历史消息从后往前累加，直到达到历史窗口上限；  
  - 对 tool 输出等超长消息可做统一截断。

这一策略与 Zeroclaw 的 `max_messages` + TTL 思路相近，但更精细。

### 6.3 Memory 注入的降级策略

考虑到 MemoryStore 架构引入需要时间，可以采用渐进式注入策略：

1. **阶段 1**：MemoryLoader 返回空时，仍保留原有 `MEMORY.md` 全量注入（向后兼容）；  
2. **阶段 2**：仅在关键对话（如含“总结”、“记住”等关键词）时强制注入更多记忆；  
3. **阶段 3**：完全依赖 MemoryLoader + 少量 MEMORY 兜底。

---

## 7. 与 Reset/Session 机制的关系

虽然本设计主要聚焦记忆架构，但与会话 reset 的交互不可避免，需要提前约定边界。

### 7.1 Reset 对 Session 的影响

- Reset 行为（无论是 Zeroclaw 风格的“清空 history”，还是 OpenClaw 风格的“切换 sessionId + 归档 transcript”）：
  - 只影响 **会话历史桶** 中的 ChatMessage；  
  - 不直接删除或修改 MemoryStore 中的长期记忆。

- 这样可以实现：
  - 用户“清空聊天”之后，新对话不再带入原有历史；  
  - 但系统仍能通过 MemoryLoader 找回长期偏好与关键信息（例如“用户喜欢中文”）。

### 7.2 Reset 对 Memory 的影响（可选策略）

为避免 Memory 无限制膨胀，可以扩展出 reset 相关的可选策略：

- 在特定 reset reason 下（如 `"session-delete"` 而非 `"session-reset"`），调用：
  - `MemoryStore::forget_by_session(session_key)` 清理与会话绑定的 Conversation 记忆；  
  - 或在 consolidation 时将重要事实从 Conversation 提升为 Core，删除其余噪声。

这些策略可以在未来的 reset 能力演进中补充，不作为本记忆架构的强依赖。

---

## 8. 渐进式落地方案

考虑到 `agent-diva` 已经有一套在用的 MEMORY/HISTORY 机制，本记忆架构建议分阶段实施。

### 8.1 Phase 1：引入 MemoryStore 与 MemoryLoader（最小可行）

- 在 `agent-diva-core` 增加：
  - `MemoryEntry` / `MemoryCategory` 类型；  
  - `MemoryStore` trait 及 `SqliteMemoryStore` 实现（仅表结构 + 简单全文索引）。

- 在 `agent-diva-agent` 增加：
  - `MemoryLoader` 接口与默认实现（基于 `MemoryStore::recall` + 简单得分排序）；  
  - `ContextBuilder` 使用 MemoryLoader 生成 `## Long-term Memory Context` 段落，并插入到 system prompt 中。

- consolidation 仍然只写入 `MEMORY.md`，不改写流程。

### 8.2 Phase 2：consolidation → MemoryStore 的双写

- 修改 consolidation 逻辑：
  - LLM 输出不再是单纯 Markdown，而是可以映射成多个 `MemoryEntry`；  
  - 写 `MEMORY.md` 的同时，调用 `MemoryStore::store`；  
  - 在 MemoryLoader 中优先使用 `MemoryStore` 的结构化记忆，必要时再回退读取 `MEMORY.md`。

- 在这一阶段，可以逐步减少对原始 MEMORY.md 全量注入的依赖。

### 8.3 Phase 3：Hybrid 检索与高级特性

- 为 `SqliteMemoryStore` 增加：
  - FTS5 虚表 + 触发器；  
  - embedding 列与向量检索（可选）。

- 在 MemoryLoader 中引入：
  - 时间衰减；  
  - 类别加权（Core boost）；  
  - 最小相关度阈值；  
  - 过采样（over fetch）与重排。

这一阶段完成后，`agent-diva` 的记忆层将非常接近 Zeroclaw 的“混合检索 + 精简注入”模型。

---

## 9. 风险与注意事项

- **迁移风险**：  
  - 从纯 Markdown MEMORY 迁移到 SQLite MemoryStore 时，需要避免历史记忆丢失；  
  - 建议提供一次性导入脚本，将现有 MEMORY.md 条目写入 `memories` 表。

- **复杂度控制**：  
  - 初期不必一次性上 embedding 与外部向量库；  
  - 先用 SQLite + FTS5 完成“结构化存储 + 关键词检索”，在此基础上评估是否需要语义检索。

- **性能与资源占用**：  
  - MemoryStore 读写与检索应使用后台线程池或异步阻塞封装，避免阻塞主 agent loop；  
  - 需要为 SQLite 访问添加合适的连接池与超时控制。

- **安全与隐私**：  
  - MemoryStore 中建议对敏感字段做加密或最小化存储；  
  - 在日志与调试输出中避免直接打印记忆内容。

---

## 10. 结论

本设计以 Zeroclaw 的记忆与上下文管理理念为蓝本，为 `agent-diva` 提出了一套 **分层清晰、可渐进演进的记忆架构**：

- 会话层继续使用现有 `SessionManager` 与 consolidation 流程；  
- 新增 MemoryStore 与 MemoryLoader，将长期记忆从“Markdown 文本堆叠”升级为“可检索的结构化知识库”；  
- `ContextBuilder` 通过少量高相关记忆段落增强 system prompt，而非全量注入 MEMORY；  
- 该架构与未来的 Session reset/Gateway/多通道部署兼容，可在不牺牲 Zeroclaw 风格高性能的前提下，逐步吸收 OpenClaw 的长程会话治理能力。

