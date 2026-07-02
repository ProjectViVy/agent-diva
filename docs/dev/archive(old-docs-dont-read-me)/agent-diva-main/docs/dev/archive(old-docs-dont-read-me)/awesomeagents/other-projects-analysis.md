# 其他项目深度分析：openfang / memtle / agent-diva-nano

> 调研日期：2026-06-01
> 目标：从三个兄弟/相关项目中提取设计亮点，为 agent-diva 的演进提供参考。

---

## 目录

- [一、openfang — Agent 操作系统](#一openfang--agent-操作系统)
- [二、memtle — 本地优先记忆宫殿](#二memtle--本地优先记忆宫殿)
- [三、agent-diva-nano — 轻量 Agent 启动线](#三agent-diva-nano--轻量-agent-启动线)
- [四、横向对比](#四横向对比)
- [五、对 agent-diva 的综合启示](#五对-agent-diva-的综合启示)

---

## 一、openfang — Agent 操作系统

### 1. 项目定位

openfang 是一个 **Agent Operating System**，由 RightNow AI 开发，定位为 7×24 自主运行的 Agent 平台，而非简单的聊天框架。

| 指标 | 值 |
|------|-----|
| 版本 | 0.6.9 (pre-1.0) |
| 代码量 | ~137K LOC，14 个 Rust crate |
| 测试 | 2,696+，零 clippy warning |
| 产物 | 单二进制 ~32MB（LTO） |
| 协议 | Apache-2.0 OR MIT |
| 仓库 | `https://github.com/RightNow-AI/openfang` |

**与 agent-diva 的关系**：同为 Rust Agent 框架，但定位更重——openfang 追求"OS 级"完整性（40 通道适配器、53+ 内置工具、16 层安全体系），而 agent-diva 追求模块化"大脑"定位。二者可互为参考。

### 2. Agent Loop 机制

核心文件：`crates/openfang-runtime/src/agent_loop.rs`

**双模式**：
- `run_agent_loop`（同步完成模式）
- `run_agent_loop_streaming`（流式模式，通过 `StreamEvent` channel）

**循环结构**（10 步）：

```
1. 记忆召回（向量相似度优先，回退到 LIKE 文本搜索）
2. 系统提示构建（基础 prompt + 记忆段落注入）
3. 会话修复（session_repair::validate_and_repair — 丢弃孤立消息、合并连续消息）
4. 上下文溢出恢复（recover_from_overflow pipeline）
5. LLM 调用（call_with_retry — 处理限流、过载、熔断、自动 fallback 模型链）
6. 工具执行（execute_tool — 循环守卫、超时包装、上下文预算动态截断）
7. 幽灵动作检测（LLM 声称已发消息但未调用工具 → 重新提示）
8. 空响应处理（累积文本作为 fallback）
9. 循环守卫（SHA256 工具调用哈希 → 检测无限循环 → 熔断）
10. 最大迭代限制（默认 50）
```

**生命周期钩子**：`BeforePromptBuild`、`BeforeToolCall`、`AfterToolCall`、`AgentLoopEnd`

**Thinking Block 保留**：推理模型（Anthropic extended thinking、Gemini 2.5+、DeepSeek-R1、Qwen3）的思考块跨轮次保留，不会被丢弃。

### 3. 工具链系统

- **53+ 内置工具**：文件系统、Web、Shell、Agent 间通信、共享内存、协作、调度、知识图谱、媒体处理、浏览器自动化（10 个）、Docker、持久进程、Hand 管理、A2A、Canvas、技能自省
- **Schema 兼容**：`normalize_schema_for_provider()` 自动剥离非 Anthropic 提供商不支持的 JSON Schema 特性（anyOf、$ref、$defs）
- **Fallback 分发**：未知工具 → MCP 连接 → 技能注册表，三级回退
- **安全层**：能力门控、审批门、Shell 元字符注入阻断、污染追踪、工作区沙箱、Agent 间调用深度限制（最大 5）

### 4. A2A 能力

**三层多 Agent 架构**：

| 层级 | 机制 | 工具 |
|------|------|------|
| 内部通信 | Agent 间消息/委托/管理 | `agent_send`, `agent_spawn`, `agent_list`, `agent_kill`, `agent_activate` |
| 协作工具 | 共享任务队列 + 事件总线 | `task_post`, `task_claim`, `task_complete`, `task_list`, `event_publish`, `agent_find` |
| 外部协议 | Google A2A 协议（JSON-RPC 2.0） | `a2a_discover`, `a2a_send`，Agent Cards at `/.well-known/agent.json` |

另外支持 MCP 双向：既可作为 MCP client 连接外部 MCP server，也可作为 MCP server 被连接。

### 5. 记忆/存储系统

`MemorySubstrate` 统一 API，组合 5 个专用存储：

| 存储 | 用途 | 后端 |
|------|------|------|
| StructuredStore | KV 键值对（Agent 状态/配置） | SQLite |
| SemanticStore | 情景记忆（LIKE 搜索 + 可选 Qdrant 向量嵌入） | SQLite |
| KnowledgeStore | 知识图谱（实体/关系/置信度/图遍历） | SQLite |
| SessionStore | 会话管理（标签、跨通道规范会话、LLM 摘要压缩） | SQLite + JSONL |
| UsageStore | Token 用量和成本计量 | SQLite |
| ConsolidationEngine | 记忆整合（衰减率） | — |

全部基于 SQLite WAL 模式。可选 HTTP memory backend（`http-memory` feature）将语义存储路由到外部网关。

### 6. 独特设计亮点

#### Hands — 自主能力包
openfang 的标志性创新。`HAND.toml` 定义一个领域完整的 Agent 配置，包含工具白名单、技能白名单、MCP 服务器白名单、需求检查（PATH 二进制、环境变量、API key）、可配置设置、仪表盘指标 schema、生命周期管理。内置 7 个 Hand：Clip（视频）、Lead（潜客）、Collector（OSINT）、Predictor（超级预测）、Researcher（深度研究）、Twitter、Browser。

#### 16 层纵深防御安全体系
WASM 双计量沙箱、Merkle 哈希链审计、信息流污染追踪、Ed25519 签名 Agent 清单、SSRF 防护、密钥清零、OFP 互认证（HMAC-SHA256）、能力门控、安全头、健康端点脱敏、子进程沙箱、提示注入扫描、循环守卫、会话修复、路径遍历防护、GCRA 限流。

#### 幽灵动作检测
LLM 声称已执行通道动作（发送、发布、邮件）但未调用任何工具 → 自动检测并重新提示要求真实工具调用。

#### 工具调用文本恢复
Groq/Llama、DeepSeek 等模型有时将工具调用输出为文本（`<function=name>{json}`）而非 tool_calls 字段 → 自动识别并提升为正式工具调用。

#### 上下文预算系统
`context_budget.rs` 动态计算每个上下文窗口的 token 预算，按比例截断过大的工具结果。

#### 40 通道适配器
Telegram、Discord、Slack、WhatsApp、Signal、Matrix、Email、Microsoft Teams 等，支持每通道模型覆盖、DM/群组策略、限流。

---

## 二、memtle — 本地优先记忆宫殿

### 1. 项目定位

memtle 是一个 **本地优先的 AI 助手记忆宫殿**。单个静态二进制（~13MB），基于嵌入式 SQLite（via turso），无需 Python、ChromaDB 或 API key。

| 指标 | 值 |
|------|-----|
| 版本 | 0.1.2 |
| 语言 | Rust (edition 2024, MSRV 1.88) |
| 关键词 | mcp, memory, ai, sqlite, llm |
| Feature flags | `cli`（clap CLI）、`mcp`（JSON-RPC 2.0 MCP server） |

**为什么存在**：Python 前身使用 ChromaDB + SQLite，在多 MCP client 并发时出现写入丢失（SQLite 锁问题）。Rust 重写用 BM25 关键词倒排索引替代 ChromaDB 语义搜索，在 turso 层解决并发问题，且从 ~100MB Python 环境缩减到 ~13MB 单二进制。

**与 agent-diva 的关系**：memtle 不是 Agent 框架，而是 **记忆后端**。它通过 MCP 工具或 Rust `MemtleToolkit` API 为 Agent 提供记忆能力。可作为 agent-diva 记忆系统的替代/补充方案。

### 2. Agent Loop 机制

memtle 本身**没有 Agent Loop**——它是记忆后端。但 `example/` 目录包含一个完整的自包含 Agent 运行时示例：

- `example/src/agent_loop.rs` — `NanoAgentLoop`：监听 MessageBus → process_turn → ContextBuilder 构建消息 → provider.chat_stream() → 流式处理 → 工具调用循环（最多 12 次）→ FinalResponse
- `example/src/agent.rs` — `Agent` / `AgentBuilder`：builder 模式构建 Agent，支持 send()、send_stream()、reload_tools()、cancel_session()、stop()
- 会话历史按 `channel:chat_id` 键维护，上限 20 条消息

### 3. 工具链系统

**两套工具系统**：

#### 核心 MCP 工具（memtle crate 本身）
- 32 个工具，静态定义在 `tool_definitions.json`（编译时 `include_str!`）
- 单一 `match` 语句分发：`memtle_status`、`memtle_search`、`memtle_add_drawer`、`memtle_kg_add`、`memtle_diary_write`、`memtle_traverse` 等
- 强类型 Args/Output 结构体（`tools/types.rs`，323 行）
- `MemtleToolkit` 封装：`call_json(name, args)` 动态 JSON 分发 + 类型化方法

#### 示例 Agent 工具
- `Tool` trait + `ToolRegistry`（`example/src/tool.rs`）
- `ToolkitTool` 适配器：将 32 个 memtle 工具自动包装为 `Tool` 实现（`example/src/memtle_tools.rs`）

### 4. A2A 能力

无多 Agent 协议，但有支持协作的机制：

- **Agent 日记**：每个 Agent 写入 `wing_{agent_name}/diary`，按 `added_by` 字段隔离，支持异步跨 Agent 通信
- **Tunnels（跨 Wing 连接）**：自动隧道（出现在多个 Wing 的 Room 自动关联）+ 显式隧道（Agent 创建命名链接），支持 BFS 遍历
- **Source Adapter 系统**：`SourceAdapter` trait，第三方适配器可从任意来源摄入内容
- **Hooks**：Claude Code / Codex CLI 集成，Stop hook 自动保存对话记录

### 5. 记忆/存储系统

**单 SQLite 文件**：`$XDG_DATA_HOME/memtle/palace.db`

**6 张表**：

| 表 | 用途 |
|----|------|
| `drawers` | 内容块：wing, room, content, source_file, chunk_index, added_by, SHA256 确定性 ID |
| `drawer_words` | 倒排索引：word → drawer_id → count（BM25 关键词搜索） |
| `entities` | 知识图谱节点：name, type, properties (JSON) |
| `triples` | 知识图谱边：subject, predicate, object, valid_from, valid_to, confidence（时间边界事实） |
| `compressed` | AAAK 压缩版本 |
| `explicit_tunnels` | 跨 Wing 链接 |

**记忆层级**：WING（人/项目）→ ROOM（子主题）→ DRAWER（800 字符块，100 字符重叠）

**关键子系统**：
- BM25 倒排索引搜索（无嵌入向量）
- Jaccard 相似度去重（阈值 0.85）
- 查询净化器（4 步剥离系统提示污染）
- 唤醒上下文：L0（身份，~100 token）+ L1（核心故事，15 个最近 drawer，~3200 字符）
- 全局实体注册表 + 可选 Wikipedia 研究
- WAL 审计日志（`write_log.jsonl`）

### 6. 独特设计亮点

#### AAAK 方言 — 有损符号压缩
将文本编码为实体代码、主题关键词、关键句子、情感信号、重要性标志。格式：`0:ENTITIES|topics|"quote"|emotions|FLAGS`。**~30 倍压缩比，LLM 可直接阅读无需解码器**。38 种情感代码，专有名词频率提升的主题提取。

#### 确定性 ID
`drawer_{wing}_{room}_{sha256(wing + \x1f + room + \x1f + content)[..24]}`。相同内容永远产生相同 ID，主键冲突即去重，`add_drawer` 天然幂等。

#### 时间边界知识图谱
每个三元组有 `valid_from` 和 `valid_to`。失效时不删除，而是设置 `valid_to`。`as_of` 查询返回特定时刻为真的事实。`kg_timeline` 提供事实时间线。

#### TigerBeetle 风格代码规范
- `dead_code = "deny"`, `unsafe_code = "deny"`, `unwrap_used = "deny"`, `expect_used = "deny"`
- 函数上限 ~70 行，超出自动分解为命名辅助函数
- "Always say why" 注释规则
- 每个重要后置条件都有 `debug_assert!`

#### 对话格式解析器
6 个解析器覆盖主流 AI 工具：Claude Code JSONL、Claude.ai JSON、Codex CLI JSONL、ChatGPT 导出 JSON、Slack 导出 JSON、Gemini CLI JSONL。

#### MCP 安全
- 1 MiB 请求帧硬上限防 OOM
- 错误消毒：内部路径和数据库细节永不泄露，仅 `"public": true` 的错误转发
- 输入验证：拒绝路径遍历、空字节、非字母数字字符
- 控制字符剥离防终端转义注入

---

## 三、agent-diva-nano — 轻量 Agent 启动线

### 1. 项目定位

agent-diva-nano（v0.4.11）是 Agent Diva 生态的**轻量独立库**，为主 monorepo 之外构建 Agent 运行时提供最小入口。

| 指标 | 值 |
|------|-----|
| 版本 | 0.4.11 |
| 作者 | mastwet (projectViVY Team, undefine foundation) |
| 定位 | 轻量 starter line |

**目标受众**：
- 想要小型 Agent Diva 入口的库消费者
- 不需要完整 manager-backed CLI 的模板/starter 项目
- 需要直接控制 provider 配置和工具组装的实验

**Crate 依赖**：agent-diva-core、agent-diva-agent、agent-diva-providers、agent-diva-tools、agent-diva-tooling、agent-diva-files（可选）

**与 agent-diva 的关系**：agent-diva-nano 是 agent-diva 的精简版，复用核心 crate 但提供独立的轻量 API。它是 agent-diva 模块化的最佳实践案例。

### 2. Agent Loop 机制

**双循环模式**，通过 `AgentLoopMode` 枚举选择：

```rust
pub enum AgentLoopMode {
    #[default]
    Standard,  // 委托给上游 agent_diva_agent::AgentLoop（完整循环）
    Nano,      // 使用自有的 NanoAgentLoop（轻量循环）
}
```

**NanoAgentLoop 结构**：
```
NanoAgentLoop {
    bus: MessageBus,
    provider: Arc<dyn LLMProvider>,
    workspace: PathBuf,
    model: String,
    config: NanoLoopConfig,
    sessions: SessionManager,
    tools: ToolRegistry,
    context: NanoContextBuilder,
    cancelled_sessions: HashSet<String>,
    runtime_control_rx: Option<mpsc::UnboundedReceiver<NanoRuntimeControlCommand>>,
}
```

**主循环**（`tokio::select!` 多路复用）：
1. 运行时控制命令（CancelSession、Stop、ReloadTools）
2. MessageBus 入站消息 → handle_inbound()

**轮次处理**：
1. 构建 session key（`channel:chat_id`）
2. SessionManager 获取/创建会话
3. NanoContextBuilder 构建 LLM 消息（系统提示 + SOUL 身份 + 工具描述 + 历史窗口 + 当前消息）
4. provider.chat_stream() 流式调用
5. 处理流：累积文本/推理/工具调用 delta → 工具执行 → 循环（简化实现）

### 3. 工具链系统

**ToolAssembly builder 模式**：

```rust
ToolAssembly::new()
    .filesystem(true)
    .shell(true)
    .web(true)
    .with_tool(Arc::new(my_custom_tool))
    .add_mcp_server("name", config)
    .with_subagent_spawner(spawner)
    .restrict_to_workspace(true)
    .build()
```

**内置工具预设**：`none()`、`minimal()`（仅文件系统）、`default()`（文件系统+Shell）、`all()`（全部）

**运行时工具热替换**：`agent.reload_tools(new_registry)` 通过 `NanoRuntimeControlCommand::ReloadTools` 通道在不停止 Agent 的情况下完全替换工具注册表。`dynamic_tools.rs` 示例演示了三阶段工具演化。

**Tool trait**（re-export from agent-diva-tooling）：
```rust
fn name(&self) -> &str;
fn description(&self) -> &str;
fn parameters(&self) -> Value;  // JSON Schema
async fn execute(&self, args: Value) -> Result<String, ToolError>;
```

### 4. A2A 能力

通过 `ToolAssembly::with_subagent_spawner(Arc<dyn SubagentSpawner>)` 注入子 Agent 生成器，启用 `spawn` 工具。`SubagentSpawner` trait 定义在上游 agent-diva-agent crate 中，agent-diva-nano 暴露注入点但不直接实现。

### 5. 记忆/存储系统

- **SessionManager**：按 `channel:chat_id` 键管理会话，每轮追加用户消息和助手响应
- **Memory Window**：`NanoLoopConfig.memory_window`（默认 10）控制注入 LLM 上下文的历史消息数
- **SOUL.md 身份系统**：从工作区根目录读取 `SOUL.md` 注入系统提示。`SoulConfig` 支持启用/禁用、最大字符截断（默认 4000）、一次性引导、变更通知、频繁变更检测、边界确认提示
- **FileManager**：`{workspace}/.agent-diva/files` 结构化文件存储

### 6. 独特设计亮点

#### 双循环架构
Standard 模式（完整功能）和 Nano 模式（轻量/可定制）共存，运行时选择，builder API 切换零成本。

#### 运行时工具热替换
不停机完全替换工具注册表。通过 `NanoRuntimeControlCommand` 通道实现，是动态工具演化的基础设施。

#### SOUL.md 身份持久化
文件驱动的 Agent 人格系统。`SoulGovernanceSettings` 包含频率变更检测和边界确认——灵魂文件可随时间演化，系统监控过度变更。

#### 三层公开 API
1. **一次性**：`chat("message", &config)`
2. **流式**：`chat_stream("message", &config)` → `mpsc::UnboundedReceiver<AgentEvent>`
3. **完全控制**：`Agent::new(config).build().await?.start().await?` + `send()` / `send_stream()` / `reload_tools()` / `cancel_session()`

#### Provider 自动解析
`resolve_provider_name()` 根据模型字符串自动检测 LLM provider（如 `openrouter/anthropic/claude-sonnet-4` → `openrouter`），未知格式回退到 `openai`。

#### Gateway 示例
完整 HTTP API（POST `/api/chat`、GET `/api/health`、GET `/api/tools`），基于 axum，`Arc<Agent>` 并发访问，可直接作为生产服务后端。

#### TUI 示例
完整终端 UI：会话管理、模型切换（`/model`）、首次配置向导、实时流式显示。

---

## 四、横向对比

### 架构定位

| 维度 | openfang | memtle | agent-diva-nano | agent-diva |
|------|----------|--------|-----------------|------------|
| 定位 | Agent OS | 记忆后端 | 轻量 Agent 库 | Agent 大脑框架 |
| 代码量 | ~137K LOC / 14 crate | 单 crate | 单 crate（依赖 5 上游 crate） | 多 crate workspace |
| Agent Loop | 完整（50 迭代，10 步） | 无（仅示例） | 双模式（Standard/Nano） | 完整（消息总线驱动） |
| 工具数 | 53+ 内置 | 32 MCP 工具 | 继承上游 + builder 组装 | 可扩展注册表 |
| 通道数 | 40 | 0（MCP 协议） | 0（MessageBus 抽象） | 8+ |

### 记忆系统

| 维度 | openfang | memtle | agent-diva-nano | agent-diva |
|------|----------|--------|-----------------|------------|
| 后端 | SQLite WAL | SQLite (turso) | SessionManager（JSONL） | JSONL + MEMORY.md |
| 语义搜索 | 向量嵌入 + LIKE | BM25 倒排索引 | 无 | 无 |
| 知识图谱 | ✅ 实体/关系/图遍历 | ✅ 时间边界三元组 | ❌ | ❌ |
| 记忆整合 | ✅ ConsolidationEngine（衰减率） | ✅ AAAK 压缩 | ❌ | ❌ |
| 跨 Agent 记忆 | ✅ 共享 memory_store | ✅ Wings/Tunnels/Diary | ❌ | ❌ |

### A2A 能力

| 维度 | openfang | memtle | agent-diva-nano | agent-diva |
|------|----------|--------|-----------------|------------|
| 内部 Agent 间 | ✅ agent_send/spawn/kill | ❌（日记隔离） | ✅ SubagentSpawner | ✅ SubagentManager |
| 外部协议 | ✅ Google A2A | ❌ | ❌ | ❌ |
| MCP | ✅ 双向 | ✅ server | ❌（可选注入） | ✅ server |
| 任务队列 | ✅ SQLite task queue | ❌ | ❌ | ❌ |
| 事件总线 | ✅ event_publish | ❌ | ❌ | ✅ MessageBus |

### 安全

| 维度 | openfang | memtle | agent-diva-nano | agent-diva |
|------|----------|--------|-----------------|------------|
| 安全层数 | 16 层纵深防御 | MCP 安全（4 项） | 继承上游 | 基础 |
| 沙箱 | WASM 双计量 | ❌ | ❌ | ❌ |
| 审计 | Merkle 哈希链 | WAL 审计日志 | ❌ | ❌ |
| 审批门 | ✅ 敏感工具审批 | ❌ | ❌ | ❌ |

---

## 五、对 agent-diva 的综合启示

### 🔴 高优先级借鉴

#### 1. 记忆系统升级（来源：openfang + memtle）

agent-diva 当前使用 JSONL + MEMORY.md，缺乏语义搜索和知识图谱。两个项目提供了不同路线：

- **openfang 路线**：SQLite WAL + 向量嵌入 + 知识图谱 + 记忆整合引擎。重量级但功能完整。
- **memtle 路线**：SQLite (turso) + BM25 倒排索引 + 时间边界知识图谱 + AAAK 压缩。轻量级且无外部依赖。

**建议**：先采用 memtle 的 BM25 倒排索引方案（无需向量数据库），后续按需升级到 openfang 的向量嵌入方案。知识图谱功能可直接参考 memtle 的时间边界三元组设计。

#### 2. 工具热替换（来源：agent-diva-nano）

agent-diva-nano 的 `reload_tools()` 通过控制通道实现不停机工具替换。agent-diva 当前缺乏此能力。

**建议**：在 AgentLoop 中引入类似的 `RuntimeControlCommand` 通道，支持工具注册表热替换。

#### 3. 上下文溢出恢复（来源：openfang）

openfang 的 `recover_from_overflow` pipeline 在每次迭代中防止上下文窗口耗尽。agent-diva 的 context builder 缺乏此机制。

**建议**：在 context builder 中实现类似的溢出恢复策略：检测 token 超限 → 按优先级截断历史消息/工具结果。

### 🟡 中优先级借鉴

#### 4. 幽灵动作检测（来源：openfang）

LLM 声称已执行动作但未调用工具——openfang 自动检测并重新提示。这对 agent-diva 的通道响应可靠性很有价值。

#### 5. SOUL.md 身份系统（来源：agent-diva-nano）

文件驱动的 Agent 人格，支持变更检测和边界确认。比硬编码系统提示更灵活，且可版本控制。

#### 6. 确定性 ID + 幂等操作（来源：memtle）

`sha256(wing + \x1f + room + \x1f + content)` 生成内容地址 ID，天然去重。适用于 agent-diva 的文件附件系统和记忆存储。

#### 7. 对话格式解析器（来源：memtle）

6 个解析器覆盖主流 AI 工具的对话格式。agent-diva 的迁移模块（agent-diva-migration）可扩展支持更多来源。

### 🟢 低优先级 / 长期参考

#### 8. Hands 能力包（来源：openfang）

`HAND.toml` 定义领域完整的 Agent 配置。agent-diva 的技能系统（Markdown-based skills）可以借鉴其需求检查和生命周期管理。

#### 9. AAAK 压缩方言（来源：memtle）

30 倍有损压缩，LLM 可直接阅读。适用于大量历史记忆的上下文注入场景。

#### 10. Google A2A 协议（来源：openfang）

跨框架 Agent 互操作标准。agent-diva 未来若需与其他 Agent 系统集成，可参考其实现。

#### 11. 16 层安全体系（来源：openfang）

WASM 沙箱、污染追踪、Merkle 审计链等。agent-diva 若面向生产环境，安全加固路线图可参考此架构。

---

### 可直接复用的模块

| 模块 | 来源 | 复用方式 |
|------|------|----------|
| BM25 倒排索引 | memtle `palace/search.rs` | 直接集成到 agent-diva 记忆系统 |
| 时间边界知识图谱 | memtle `tools/kg.rs` | 作为独立 crate 引入 |
| 确定性 ID 计算 | memtle `tools/dispatch.rs` | 复用哈希算法 |
| Schema 兼容层 | openfang `normalize_schema_for_provider()` | 集成到 agent-diva-providers |
| NanoContextBuilder | agent-diva-nano `internal/context.rs` | 参考其 prompt 组装逻辑 |
| RuntimeControlCommand | agent-diva-nano `nano_loop.rs` | 引入 AgentLoop 控制通道 |

---

*本文档基于对三个项目源码的深度阅读生成。每个项目的完整分析可参考各自的 README 和 CLAUDE.md。*
