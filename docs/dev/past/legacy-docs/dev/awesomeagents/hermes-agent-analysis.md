# Hermes Agent 运行时能力深度分析

> **分析视角**：从 agent 运行时架构师的角度，将 Hermes 作为"AI agent 框架"进行逆向工程分析。
> **区别于已有文档**：`hermes-learning/` 和 `hermes-integration/` 侧重"如何集成 hermes 学习机制到 diva"，本文侧重"hermes 自身作为 agent 运行时有哪些能力"。
> **分析日期**：2026-06-01

---

## 目录

1. [Agent Loop 机制](#1-agent-loop-机制)
2. [工具链系统](#2-工具链系统)
3. [A2A（Agent-to-Agent）能力](#3-a2aagent-to-agent能力)
4. [技能系统（Skills）](#4-技能系统skills)
5. [记忆与上下文持久化](#5-记忆与上下文持久化)
6. [通道/多平台支持](#6-通道多平台支持)
7. [对 Agent-Diva 的启示](#7-对-agent-diva-的启示)

---

## 1. Agent Loop 机制

### 1.1 核心架构：胖 Agent + 模块提取

Hermes 采用 **"胖 Agent + 模块提取"** 架构。`AIAgent` 类（`run_agent.py:294`）是系统的单一状态中心，承载所有运行时状态（重试计数器、压缩器、工具注册表、会话 DB 等），但实际逻辑通过模块级函数（以 `agent` 为第一个参数）提取到 `agent/` 子模块中。

```
run_agent.py (AIAgent 门面)
    ├── agent/conversation_loop.py   ← run_conversation() 主循环
    ├── agent/tool_executor.py       ← 工具串行/并发执行
    ├── agent/system_prompt.py       ← 3 层系统提示词构建
    ├── agent/context_compressor.py  ← 上下文窗口压缩
    ├── agent/iteration_budget.py    ← 迭代预算控制
    ├── agent/error_classifier.py    ← 错误分类与恢复
    ├── agent/memory_manager.py      ← 记忆编排
    └── agent/skill_commands.py      ← 技能触发
```

**设计模式**：
- **Facade + Delegation**：`AIAgent` 是门面，逻辑分散在子模块
- **`_ra()` 懒引用**：保持测试 mock 能力（`patch("run_agent.X")` 不会因直接 import 失效）
- **状态集中**：所有运行时状态挂在实例上，无 DI 容器

> **代码引用**：`run_agent.py:317-384` — `__init__` 是转发器，实际初始化委托给 `agent.agent_init.init_agent`

### 1.2 主循环五阶段

`run_conversation()`（`conversation_loop.py:351-378`）驱动单轮用户交互，生命周期分为 5 个阶段：

#### 阶段 1 — 初始化（`conversation_loop.py:379-583`）

- 安装安全 stdio（`_install_safe_stdio`）
- 重置每轮重试计数器（约 10 个：`_invalid_tool_retries`、`_empty_content_retries` 等）
- 创建新的 `IterationBudget`（默认 90 次迭代）
- 水合 todo store 和 nudge 计数器（从历史记录恢复，解决 gateway 每消息新建 agent 的问题）
- 系统提示词缓存恢复（`_restore_or_build_system_prompt`）

#### 阶段 2 — 预检压缩（`conversation_loop.py:587-685`）

- 使用 `estimate_request_tokens_rough` 粗估 token 数（含工具 schema）
- 超过压缩阈值时最多执行 3 轮压缩
- 压缩后重置重试计数器

#### 阶段 3 — 插件钩子（`conversation_loop.py:687-721`）

- `pre_llm_call` 钩子：插件可注入上下文到用户消息（**不是系统提示词**，以保持缓存稳定）
- 外部内存 provider 预取（`prefetch_all`，结果缓存到 `_ext_prefetch_cache`）

#### 阶段 4 — 主循环（`conversation_loop.py:796-4540`）

```python
while (api_call_count < agent.max_iterations
       and agent.iteration_budget.remaining > 0) or agent._budget_grace_call:
```

每次迭代的步骤：

| 步骤 | 位置 | 说明 |
|------|------|------|
| 中断检查 | `conversation_loop.py:800-806` | 检查 `_interrupt_requested` 标志 |
| 预算消耗 | `conversation_loop.py:815-821` | `iteration_budget.consume()` 返回 False 时退出 |
| /steer 排空 | `conversation_loop.py:857-905` | 将用户在模型思考期间发送的引导文本注入到最后一条 tool 消息 |
| 消息准备 | `conversation_loop.py:907-1080` | 修复损坏的 tool_call 参数、修复消息序列违规、注入临时上下文、应用 Anthropic 缓存控制标记、规范化空白 |
| API 调用 + 重试 | `conversation_loop.py:1157-1952` | 内层重试循环，支持流式/非流式，`finish_reason == "length"` 最多 3 次续写 |
| 错误分类与恢复 | `conversation_loop.py:2224-2500+` | 8 级优先级错误分类管道 |
| 工具调用处理 | `conversation_loop.py:3459-3883` | 串行/并发执行，guardrail 检查，上下文压缩决策 |

#### 阶段 5 — 收尾（`conversation_loop.py:4550-4703`）

- `transform_llm_output` / `post_llm_call` 插件钩子
- 提取 reasoning、构建返回结果（含 token 统计、成本估算）
- 外部内存同步（`_sync_external_memory_for_turn`）
- **后台审查**（`_spawn_background_review`）：内存审查 + 技能审查，在响应交付后异步运行

### 1.3 3 层系统提示词缓存

这是 Hermes 最精巧的设计之一。系统提示词分为 3 层，以最大化 Anthropic 前缀缓存命中率：

```
┌─────────────────────────────────────────┐
│  Stable 层（几乎不变）                    │
│  - 身份：SOUL.md 或 DEFAULT_IDENTITY     │
│  - 工具引导（按工具存在条件注入）          │
│  - 任务完成引导                           │
│  - 模型族特定引导（Gemini/GPT/Codex）     │
│  - 环境提示（WSL/Termux/Python）          │
│  - 平台提示（Telegram/Discord）           │
├─────────────────────────────────────────┤
│  Context 层（会话级变化）                 │
│  - 调用者提供的 system_message            │
│  - 上下文文件发现（AGENTS.md 等）         │
├─────────────────────────────────────────┤
│  Volatile 层（每轮可能变化）              │
│  - 内存快照（MEMORY.md）                 │
│  - 用户画像（USER.md）                   │
│  - 外部内存 provider 块                  │
│  - 时间戳行（仅日期，不含分钟，保持字节稳定）│
│  - Session ID / Model / Provider         │
└─────────────────────────────────────────┘
```

**关键设计**：时间戳行仅含日期不含分钟，保持字节稳定以维持前缀缓存。整个提示词作为单个缓存块，只在首次构建和上下文压缩后重建。通过 session DB 持久化，跨轮次复用。

> **代码引用**：`agent/system_prompt.py:61-345` — 3 层构建逻辑
> **代码引用**：`agent/system_prompt.py:348-364` — 缓存策略

### 1.4 工具并发/串行执行

工具执行引擎（`agent/tool_executor.py`）支持两种模式：

**并发执行**（`tool_executor.py:110-538`）：
- `ThreadPoolExecutor(max_workers=min(len(runnable), 8))`
- 线程上下文传播：`propagate_context_to_thread` 将 ContextVar 传播到 worker 线程
- 中断扇出：worker 注册 tid 到 `_tool_worker_threads`，中断时广播
- 心跳机制：5 秒轮询 + 每 30 秒发送活动心跳
- 结果保序：`results = [None] * num_tools` 按原始顺序收集

**并发决策逻辑**（`tool_dispatch_helpers.py:103`）：
```python
def _should_parallelize_tool_batch(tool_calls) -> bool:
    # 检查 _NEVER_PARALLEL_TOOLS（delegate_task 等）
    # 检查文件路径冲突（写同一文件的工具不能并行）
    # 检查 MCP 工具的 parallel_safe 标记
```

**串行执行**（`tool_executor.py:542-1008`）：
- 特殊工具直接路由：`todo`、`session_search`、`memory`、`clarify`、`delegate_task`
- 每个工具执行后 drain `/steer`（用户引导文本）

### 1.5 错误分类与恢复

`error_classifier.py` 实现了 8 级优先级的错误分类管道：

```python
@dataclass
class ClassifiedError:
    reason: FailoverReason       # 22 种错误类型
    retryable: bool = True
    should_compress: bool = False
    should_rotate_credential: bool = False
    should_fallback: bool = False
```

**分类优先级**（`error_classifier.py:438-720`）：

| 优先级 | 检查内容 | 示例 |
|--------|---------|------|
| 1 | Provider 特定模式 | 内容策略阻止、thinking 签名、OAuth 1M beta |
| 2 | HTTP 状态码 + 消息细化 | 402 可能是伪装的限速 |
| 3 | 错误码分类 | `error_code` 字段匹配 |
| 4 | 消息模式匹配 | 无状态码时的文本匹配 |
| 5 | SSL/TLS 瞬态错误 | → timeout |
| 6 | 服务器断开 + 大会话 | → context_overflow |
| 7 | 传输错误启发式 | 网络层错误 |
| 8 | 未知 | → 可重试 |

**关键洞察**：某些 402 错误是伪装成支付错误的瞬时限速（"Usage limit, try again in 5 minutes"）。

**恢复策略**（按优先级）：Nous 费用刷新 → 凭证池轮换 → 图片缩小 → 多模态工具内容降级 → OAuth 1M beta 禁用 → 各 provider 认证刷新 → thinking 签名恢复 → 加密内容重放禁用 → 上下文压缩 → fallback

> **代码引用**：`agent/error_classifier.py:438-720` — 分类管道
> **代码引用**：`agent/error_classifier.py:881-907` — 402 消歧义

### 1.6 迭代预算

`IterationBudget`（`agent/iteration_budget.py:17-62`）是线程安全的迭代计数器：

- 父 agent 预算 = `max_iterations`（默认 90）
- 子 agent 独立预算 = `delegation.max_iterations`（默认 50）
- `execute_code` 调用通过 `refund()` 退还（因为它们是廉价的 RPC 调用）
- 总迭代数可超过父预算（父子独立计数）

### 1.7 上下文压缩

`ContextCompressor`（`agent/context_compressor.py:522`）实现了 5 阶段压缩算法：

1. **工具输出修剪**（`context_compressor.py:754-920`）：去重 → 信息性摘要替换 → 截断大型 tool_call 参数
2. **边界确定**（`context_compressor.py:1885-1889`）：头部保护（系统提示词 + 前 N 条消息）+ 尾部保护（基于 token 预算，反向累加）
3. **LLM 摘要生成**（`context_compressor.py:1217-1515`）：结构化模板（Active Task / Goal / Completed Actions / Active State / ...），增量更新
4. **确定性回退**（`context_compressor.py:1001-1188`）：LLM 失败时本地提取关键信息
5. **组装与清理**（`context_compressor.py:1964-2078`）：清理孤立 tool_call/tool_result 对，去除历史图片，反抖动保护（连续 2 次压缩节省 <10% 则停止）

---

## 2. 工具链系统

### 2.1 注册机制：自注册 + AST 发现

Hermes 工具系统采用 **自注册模式** + **AST 自动发现**：

```python
# tools/terminal_tool.py（模块级注册）
registry.register(
    name="terminal",
    toolset="terminal",
    schema=TERMINAL_SCHEMA,
    handler=_handle_terminal,
    check_fn=_check_terminal_available,
    emoji="💻",
)
```

**注册链**：
```
tools/registry.py（ToolRegistry 单例，无循环导入）
    ↑
tools/*.py（模块级 register 调用）
    ↑
model_tools.py（import 时触发 discover_builtin_tools()）
```

**AST 发现**（`tools/registry.py:57-74`）：`discover_builtin_tools()` 使用 AST 解析扫描 `tools/*.py`，仅导入包含顶层 `registry.register(...)` 调用的模块。排除 `__init__.py`、`registry.py`、`mcp_tool.py`。

> **代码引用**：`tools/registry.py:151-519` — ToolRegistry 类
> **代码引用**：`tools/registry.py:234-305` — register() 方法

### 2.2 ToolEntry 元数据

每个工具注册为 `ToolEntry`（`tools/registry.py:77-106`）：

| 字段 | 类型 | 说明 |
|------|------|------|
| `name` | str | 工具名 |
| `toolset` | str | 所属 toolset |
| `schema` | dict | OpenAI function 格式 |
| `handler` | Callable | 处理函数 |
| `check_fn` | Callable | 可用性探针（30s TTL 缓存） |
| `requires_env` | list | 所需环境变量 |
| `is_async` | bool | 是否异步处理器 |
| `max_result_size_chars` | int | 结果大小上限 |
| `dynamic_schema_overrides` | Callable | 运行时动态覆盖 schema |

**check_fn TTL 缓存**（`registry.py:121-148`）：外部状态探测（Docker、Playwright 等）结果缓存 30 秒，避免重复探测。

### 2.3 Toolset 分组系统

`toolsets.py` 定义了分层的工具集分组：

```
_HERMES_CORE_TOOLS（核心工具列表）
    ├── web_search, web_extract, terminal, file_read, ...
    ├── delegate_task, kanban_show, kanban_list, ...
    └── memory, todo, session_search, skills_list, ...

TOOLSETS（静态定义）
    ├── 叶子 toolset：web, terminal, file, vision, ...
    ├── 组合 toolset：debugging = [terminal, process] + includes [web, file]
    ├── 平台 toolset：hermes-discord = _HERMES_CORE_TOOLS + discord 工具
    └── 安全子集：_HERMES_WEBHOOK_SAFE_TOOLS（仅 web_search, web_extract, vision_analyze, clarify）
```

**解析机制**（`toolsets.py:606-677`）：`resolve_toolset()` 递归解析，带环检测。特殊别名 `"all"` / `"*"` 递归解析所有 toolset。

### 2.4 MCP 支持

MCP 工具桥接（`tools/mcp_tool.py`）是 Hermes 工具系统的核心扩展机制：

**三种传输协议**：stdio、HTTP/StreamableHTTP、SSE

**核心类 `MCPServerTask`**（`mcp_tool.py:1096-1860`）：
- 每个 MCP 服务器在一个专用 asyncio Task 中运行
- `_rpc_lock` 串行化客户端 RPC，防止 stdio JSON-RPC 流阻塞
- 支持自动重连（指数退避，最多 5 次重试）和 keepalive（每 180 秒 `list_tools`）

**动态工具刷新**（`mcp_tool.py:1215-1275`）：响应 `notifications/tools/list_changed` 通知，差异计算新增/移除的工具，避免 nuke-and-repave。

**断路器**（`mcp_tool.py:1885-1912`）：连续 3 次失败后断路器打开（60 秒冷却）。

**安全模型**：

| 安全机制 | 位置 | 说明 |
|---------|------|------|
| 环境变量过滤 | `mcp_tool.py:267-313` | `_build_safe_env()` 仅传递安全基线变量，防止泄露宿主 API key |
| 凭证脱敏 | `mcp_tool.py:272-284` | `_sanitize_error()` 替换 GitHub PAT、OpenAI key 等为 `[REDACTED]` |
| Prompt 注入扫描 | `mcp_tool.py:343-385` | 扫描 MCP 工具描述中的注入模式（警告级，不阻断） |
| OSV 恶意软件检查 | `mcp_tool.py:1365-1369` | stdio 模式下启动子进程前检查包是否在 OSV 恶意软件数据库中 |
| URL 校验 | `mcp_tool.py:521-570` | 拒绝非 http(s) 协议 |

**SamplingHandler**（`mcp_tool.py:725-1089`）：处理 MCP 服务器的 `sampling/createMessage` 请求，支持滑动窗口速率限制、模型白名单检查、工具循环治理。

> **代码引用**：`tools/mcp_tool.py:1096-1860` — MCPServerTask 完整实现

### 2.5 Progressive Disclosure — Tool Search

当 MCP/plugin 工具的 schema token 开销超过上下文窗口的阈值（默认 10%）时，Hermes 将它们替换为三个桥接工具（`tools/tool_search.py`）：

```
tool_search  — 关键词搜索延迟工具目录（BM25 算法）
tool_describe — 加载单个工具的完整 JSON schema
tool_call     — 按名称调用延迟工具
```

**分类逻辑**（`tool_search.py:150-209`）：
- `_HERMES_CORE_TOOLS` 中的工具 **永不延迟**
- MCP toolset（`mcp-` 前缀）的工具可延迟
- 非核心的插件工具可延迟

**BM25 检索**（`tool_search.py:347-418`）：标准 BM25 算法对延迟工具目录进行关键词搜索，回退到名称子串匹配。

### 2.6 编排层 — handle_function_call

`model_tools.py` 中的 `handle_function_call()`（`model_tools.py:802-1038`）是工具分发的中枢：

```
1. coerce_tool_args        — 将 LLM 返回的字符串参数强制转换为 schema 声明的类型
2. Tool Search 桥接分发    — tool_search/tool_describe 内联处理；tool_call 递归调用
3. _AGENT_LOOP_TOOLS 拦截  — todo, memory, session_search, delegate_task 由 agent loop 处理
4. 插件 pre_tool_call 钩子
5. ACP 编辑审批守卫
6. registry.dispatch()     — 实际执行
7. post_tool_call 和 transform_tool_result 钩子
```

**工具错误净化**（`model_tools.py:576-599`）：`_sanitize_tool_error` 剥离工具异常中的结构化 framing token（XML 角色标签、CDATA、markdown 代码围栏），防止注入到模型上下文中的错误消息触发角色混淆。截断至 2000 字符。

**异步桥接**（`model_tools.py:42-173`）：`_run_async()` 三种路径——已有事件循环（gateway 内部）、Worker 线程（delegate_task）、主线程。

### 2.7 注册覆写保护

`registry.py:257-289` 实现了工具注册的安全守卫：
- 不同 toolset 之间的同名注册默认被拒绝
- MCP-to-MCP 覆写允许
- 显式 `override=True` 允许插件替换内建工具

---

## 3. A2A（Agent-to-Agent）能力

### 3.1 delegate_task — 运行时派生

`delegate_task`（`tools/delegate_tool.py`）是 Hermes 的核心 A2A 机制，允许父 agent 生成具有隔离上下文的子 agent 实例。

#### 子 Agent 构建

`_build_child_agent()`（`delegate_tool.py:870-1174`）：

```
父 Agent
  │
  ├── 生成唯一 subagent_id
  ├── 工具集继承 + 交集守卫（子不能获得父没有的工具）
  ├── MCP toolset 保留
  ├── 凭证继承或覆盖
  ├── 构建进度回调（CLI 树形显示 + Gateway 批量中继）
  │
  └── 子 Agent（AIAgent 实例）
       ├── 独立上下文（无父历史）
       ├── 独立迭代预算（默认 50）
       ├── 独立终端会话
       └── 受限工具集
```

#### 安全模型

**工具黑名单**（`delegate_tool.py:45-53`）：`DELEGATE_BLOCKED_TOOLS` 永久禁止子 agent 访问：

| 被禁工具 | 原因 |
|---------|------|
| `delegate_task` | 禁止递归委托 |
| `clarify` | 禁止用户交互 |
| `memory` | 禁止写入共享 MEMORY.md |
| `send_message` | 禁止跨平台副作用 |
| `execute_code` | 子 agent 应逐步推理而非编写脚本 |

**深度控制**（`delegate_tool.py:133-138, 394-429`）：
- `MAX_DEPTH = 1`（默认扁平：父 0 → 子 1）
- `delegation.max_spawn_depth` 可配置为 1-3（`_MAX_SPAWN_DEPTH_CAP = 3`）
- `depth >= max_spawn` 时拒绝新委托

#### 角色模型

| 角色 | 能力 | 控制 |
|------|------|------|
| `leaf`（默认） | 不能进一步委托 | 安全默认值 |
| `orchestrator` | 保留 `delegation` toolset，可生成自己的 worker | 受 `_get_orchestrator_enabled()` 全局开关控制 |

#### 运行时状态管理

- `_spawn_paused` 全局暂停标志（`delegate_tool.py:149-167`）
- `_active_subagents` 注册表（`delegate_tool.py:155-156`）— TUI 可观测性
- `interrupt_subagent()`（`delegate_tool.py:188-208`）— 通过设置中断标志传播停止信号
- 心跳线程（`delegate_tool.py:1362-1436`）— 每 30 秒触碰父 agent 活动时间戳，防止 gateway 超时杀死代理
- Gateway RPCs：`delegation.pause`、`delegation.status`、`subagent.interrupt`

#### 文件状态协调

`delegate_tool.py:1726-1754`：子 agent 完成后检查它是否修改了父 agent 之前读取的文件，在摘要中追加：
```
[NOTE: subagent modified files the parent previously read — re-read before editing: ...]
```

#### 超时诊断

`_dump_subagent_timeout_diagnostic()`（`delegate_tool.py:1177-1318`）：当子 agent 在 0 次 API 调用后超时时，将配置、prompt 大小、worker 线程栈转储到日志文件。

### 3.2 Kanban — 结构化多 Agent 协调

Kanban 系统（`tools/kanban_tools.py`）通过 SQLite 看板实现持久化任务路由，是 Hermes 多 agent 编排的核心机制。

#### 架构概览

```
Orchestrator Agent
  │
  ├── kanban_create（扇出任务到看板）
  │
  └── Dispatcher（下一个 tick）
       │
       ├── 派生 Worker A（assigned profile）
       ├── 派生 Worker B（assigned profile）
       └── 派生 Worker C（assigned profile）
            │
            ├── kanban_show（读取任务状态）
            ├── kanban_complete（结构化交接）
            └── kanban_block（请求人工输入）
```

#### 门控机制

两种检查函数（`kanban_tools.py:49-91`）：
- `_check_kanban_mode` — worker 可用（`HERMES_KANBAN_TASK` 环境变量设置）或 profile 显式启用 kanban toolset
- `_check_kanban_orchestrator_mode` — 仅 orchestrator 可用

#### Worker 所有权守卫

`_enforce_worker_task_ownership()`（`kanban_tools.py:132-161`）：worker 进程只能操作自己的任务（由 `HERMES_KANBAN_TASK` 环境变量限定）。拒绝对其他任务的破坏性调用，防止被注入的 worker 破坏同级任务。

#### 核心工具集

**Worker 生命周期工具**：

| 工具 | 功能 | 安全特性 |
|------|------|---------|
| `kanban_show` | 读取任务完整状态 | — |
| `kanban_complete` | 标记完成，支持结构化交接 | `HallucinatedCardsError` 防止幻觉任务 ID |
| `kanban_block` | 阻塞等待人工输入 | — |
| `kanban_heartbeat` | 长操作信号存活 | 同时延长 claim TTL 和记录心跳事件 |
| `kanban_comment` | 追加评论 | 作者从 `HERMES_PROFILE` 派生，不接受调用者覆盖，防止伪造系统指令 |

**Orchestrator 路由工具**：

| 工具 | 功能 |
|------|------|
| `kanban_list` | 列出任务摘要（支持 assignee/status/tenant 过滤） |
| `kanban_create` | 创建子任务（支持 parents 依赖、workspace_kind、triage、goal_mode） |
| `kanban_unblock` | 将阻塞任务移回 ready |
| `kanban_link` | 添加父子依赖边（拒绝循环和自引用） |

#### 自动心跳桥接

`heartbeat_current_worker_from_env()`（`kanban_tools.py:204-260`）：当 agent loop 的 `_touch_activity` 触发时，自动更新 kanban 看板的心跳时间戳（每 60 秒限流一次）。不需要模型显式调用 `kanban_heartbeat` 工具。

#### 依赖门控的自动晋升

当父任务完成时，子任务自动从 `blocked` 晋升为 `ready`，实现流水线式的多 agent 编排。

> **代码引用**：`tools/kanban_tools.py:49-91` — 门控检查
> **代码引用**：`tools/kanban_tools.py:132-161` — Worker 所有权守卫
> **代码引用**：`hermes_cli/kanban_db.py` — SQLite 看板数据库

### 3.3 Cron — 定时 Agent 机制

Cron 系统支持两种执行模式：

**调度器**（`cron/scheduler.py:1857-2035` `tick()`）：
- 每 60 秒由网关后台线程调用
- 文件锁确保同一时间只有一个 tick 运行
- 支持并行执行（`cron.max_parallel_jobs`）
- workdir/profile 作业串行执行（修改进程全局状态），其余并行
- 执行前先 `advance_next_run()` 确保 at-most-once 语义

**两种执行模式**：

| 模式 | 说明 | 位置 |
|------|------|------|
| `no_agent` | 脚本即作业，stdout 直接投递 | `scheduler.py:1239-1322` |
| LLM | 构建 AIAgent，注入 prompt（含脚本输出、context_from 引用、skill 内容） | `scheduler.py:1330-1791` |

**作业存储**（`cron/jobs.py`）：
- `~/.hermes/cron/jobs.json`
- 三种调度类型：once（一次性）、interval（间隔）、cron（cron 表达式）
- 错过作业处理：超 grace 窗口的循环作业快速前进到下次执行，而非立即补执行

**投递目标**（`scheduler.py:386-547`）：
- `"origin"` → 回到创建作业的聊天
- `"local"` → 仅保存到文件
- `"telegram"` → Telegram home channel
- `"all"` → 所有已配置 home channel 的平台
- 支持逗号分隔的多目标投递

**安全**：`CronPromptInjectionBlocked` 异常防止恶意 skill 内容注入到 cron prompt 中。

### 3.4 ACP — Agent Client Protocol

Hermes 实现了 ACP（Agent Client Protocol）适配器：

- `acp_adapter/entry.py` — CLI 入口（stdio JSON-RPC 传输）
- `acp_adapter/server.py` — ACP agent server
- `acp_adapter/session.py` / `tools.py` / `events.py` / `permissions.py` — 会话/工具/事件/权限管理
- `acp_registry/` — ACP 服务注册

### 3.5 A2A 模式总结

Hermes 通过三种机制实现 A2A：

| 机制 | 适用场景 | 复杂度 |
|------|---------|--------|
| `delegate_task` | 运行时派生，单任务或批量（并行）模式 | 低（工具调用） |
| Kanban 看板 | 结构化协调，持久化任务路由，依赖门控 | 中（CLI + 工具） |
| Cron | 定时触发，支持 no_agent 和 LLM 两种模式 | 低（配置驱动） |

---

## 4. 技能系统（Skills）

### 4.1 Skill 格式规范

Skills 是包含 `SKILL.md` 文件的目录，使用 YAML frontmatter（兼容 agentskills.io）：

```
skills/
  my-skill/
    SKILL.md           # 主指令（必需）
    references/        # 支持文档
    templates/         # 模板
    assets/            # 补充文件
```

**Frontmatter 字段**：

| 字段 | 约束 | 说明 |
|------|------|------|
| `name` | max 64 chars | 技能名 |
| `description` | max 1024 chars | 描述 |
| `version` | — | 版本号 |
| `license` | — | 许可证 |
| `platforms` | `[macos, linux, windows]` | 平台限制 |
| `prerequisites` | — | 前置条件 |
| `compatibility` | — | 兼容性 |
| `metadata` | — | tags, related_skills |

### 4.2 加载/匹配/触发机制

**发现**（`agent/skill_utils.py:241-324`）：
- 扫描 `~/.hermes/skills/`（安装时从 bundled `skills/` 种子复制）
- 基于 mtime 的进程内缓存，避免重复 YAML 解析
- 排除目录集：`.git`, `.venv`, `node_modules`, `__pycache__` 等

**平台匹配**（`agent/skill_utils.py:128-169`）：支持 `platforms: [macos, linux, windows]`，Termux 特殊处理。

**触发方式**：
1. **斜杠命令**（`agent/skill_commands.py:263-326`）：`/skill-name` 从 CLI 或 gateway 触发
2. **工具调用**：agent 使用 `skills_list` 和 `skill_view` 工具直接访问
3. **预加载**：CLI `-s` 参数加载多个技能

**名称规范化**（`skill_commands.py:311-313`）：空格和下划线转连字符，剥离非字母数字字符。连字符/下划线可互换解析。

### 4.3 渐进式披露

`skills_tool.py` 实现了 3 层渐进式披露：

| 层级 | 工具 | 返回内容 |
|------|------|---------|
| Tier 1 | `skills_list` | 仅 name + description（元数据） |
| Tier 2-3 | `skill_view` | 完整内容 + 链接文件（按需加载） |

**碰撞检测**（`skills_tool.py:925-990`）：跨目录同名技能拒绝加载，要求使用分类路径。

### 4.4 模板变量替换与 Shell 执行

`agent/skill_preprocessing.py` 提供了技能内容的动态能力：

**模板变量**（`skill_preprocessing.py:37-60`）：
- `${HERMES_SKILL_DIR}` — 技能目录路径
- `${HERMES_SESSION_ID}` — 当前会话 ID

**内联 Shell 执行**（`skill_preprocessing.py:63-98`）：
- 语法：`` !`date +%Y-%m-%d` ``
- 4000 字符输出上限
- 超时默认 10 秒
- 受 `skills.inline_shell` 配置开关控制，默认禁用

### 4.5 技能束（Skill Bundles）

`agent/skill_bundles.py` 支持将多个技能组合为一个束：

- YAML 定义：`~/.hermes/skill-bundles/*.yaml`
- 包含 `name`、`skills` 列表、可选 `instruction`
- 束优先于技能：同名时束赢得斜杠命令
- mtime 感知缓存

### 4.6 Skills Hub — 远程安装

`tools/skills_hub.py` 实现了多源技能安装系统：

**源适配器**（10+ 个）：

| 源 | 说明 |
|----|------|
| `GitHubSource` | Git Trees API 单请求整棵树 + Contents API 降级 |
| `WellKnownSkillSource` | `/.well-known/skills/index.json` 端点 |
| `SkillsShSource` | skills.sh 目录，sitemap 走全量 ~20k+ 目录 |
| `ClawHubSource` | ClawHub API，分页目录走全量 50k+，ZIP 下载 |
| `ClaudeMarketplaceSource` | Claude 市场 |
| `LobeHubSource` | 将 LobeHub agent JSON 转换为 SKILL.md |
| `HermesIndexSource` | 集中式索引，零 API 调用搜索 |
| `OptionalSkillSource` | 仓库内 optional-skills/ 目录 |

**安全机制（深度防御）**：

| 安全层 | 位置 | 说明 |
|--------|------|------|
| 路径规范化 | `skills_hub.py:94-116` | 拒绝绝对路径、`..` 遍历、Windows 驱动器号 |
| 安装路径验证 | `skills_hub.py:164-189` | 逐组件走路径，拒绝符号链接/连接点重定向 |
| 符号链接检测 | `skills_hub.py:154-161` | 安装前遍历隔离区拒绝符号链接 |
| SSRF 防护 | `skills_hub.py:192-226` | `is_safe_url` + `check_website_access` + 重定向跟踪限制（5 次） |
| 隔离区扫描 | `skills_hub.py:3171-3278` | 下载 → 隔离 → 安全扫描 → 安装 |
| 审计日志 | `skills_hub.py:3138-3151` | 记录所有安装/卸载操作 |
| 缓存隔离 | `skills_hub.py:2983-2999` | 缓存目录写入 `.ignore` 文件防止 ripgrep 搜索到未审查内容 |

**ClawHub 信任降级**：ClawHavoc 事件后（2026 年 2 月发现 341 个恶意技能），所有 ClawHub 技能标记为 `community` 信任。

---

## 5. 记忆与上下文持久化

### 5.1 内置记忆 — 冻结快照模式

`tools/memory_tool.py` 实现了双存储记忆系统：

**两套并行状态**：
- `_system_prompt_snapshot`：加载时冻结的快照，用于系统提示注入，会话期间永不变更以保持前缀缓存稳定
- `memory_entries` / `user_entries`：实时状态，工具调用立即修改并持久化到磁盘

**数据流**：
```
1. load_from_disk()     → 读取 MEMORY.md / USER.md，去重，构建冻结快照
2. 快照构建时           → 对每个条目进行威胁扫描（_sanitize_entries_for_snapshot）
3. add/replace/remove   → 文件锁内重新读取磁盘 → 检测外部漂移 → 原子写入
4. format_for_system_prompt() → 返回冻结快照（不反映会话中修改）
```

**安全机制**：

| 机制 | 位置 | 说明 |
|------|------|------|
| 威胁模式扫描 | `memory_tool.py:76-80` | 写入前使用 `tools.threat_patterns` 的 `strict` 范围扫描注入/外泄模式 |
| 外部漂移检测 | `memory_tool.py:515-568` | 两种信号：往返不匹配和单条目超限。检测到漂移时创建 `.bak.<ts>` 备份并拒绝写入 |
| 跨平台文件锁 | `memory_tool.py:209-243` | Unix 用 `fcntl.flock`，Windows 用 `msvcrt.locking` |
| 原子写入 | `memory_tool.py:570-599` | 先写临时文件再 `os.replace`，避免截断竞态 |
| 字符限制 | `memory_tool.py:124` | MEMORY.md 2200 字符，USER.md 1375 字符（按字符计数，模型无关） |

> **代码引用**：`tools/memory_tool.py:113-600` — MemoryStore 完整实现

### 5.2 Memory Manager — 多 Provider 编排

`agent/memory_manager.py` 编排内置 provider + 最多一个外部 provider：

**生命周期**：
```
build_system_prompt()  → 收集所有 provider 的静态块
prefetch_all()         → 每轮 API 调用前收集上下文
sync_all()             → 每轮完成后持久化
handle_tool_call()     → 按工具名路由到正确 provider
```

**生命周期钩子**：
- `on_turn_start` — 轮次开始
- `on_session_end` — 会话结束
- `on_session_switch` — `/resume`、`/branch`、`/reset`、压缩时触发
- `on_pre_compress` — 压缩前提取洞察
- `on_memory_write` — 镜像内置记忆写入到外部 provider
- `on_delegation` — 子代理完成时通知

**StreamingContextScrubber**（`memory_manager.py:62-225`）：有状态的流式文本清洗器，处理 `<memory-context>` 标签跨越 chunk 边界的情况。

### 5.3 Memory Provider 接口

`agent/memory_provider.py` 定义了 `MemoryProvider` ABC：

**核心抽象方法**：
- `name` — provider 名称
- `is_available()` — 仅检查配置，不发网络请求
- `initialize()` — kwargs 包含 `hermes_home`、`platform`、`agent_context` 等
- `get_tool_schemas()` — 返回工具 schema

**可选钩子**：
- `on_session_switch(reset)` — 区分真正新对话和恢复
- `on_memory_write(metadata)` — `metadata` 包含 `write_origin`、`execution_context`、`session_id` 等结构化来源信息
- `get_config_schema()` / `save_config()` — `hermes memory setup` 引导配置

**外部 provider 注册**：通过 `plugins/memory/` 目录下的插件注册（Honcho、Hindsight、Mem0 等）。

### 5.4 Session 数据库设计

`hermes_state.py` 中的 `SessionDB` 是 Hermes 的核心持久化层：

**数据模型**：

| 表 | 关键字段 |
|----|---------|
| `sessions` | id, source, user_id, model, system_prompt, parent_session_id, started_at, ended_at, token 计数, 成本追踪, title, handoff_state |
| `messages` | id, session_id, role, content, tool_calls, reasoning 字段, platform_message_id, observed |
| `state_meta` | 键值对存储 |
| `compression_locks` | 防止并发压缩竞态 |

**FTS5 搜索**（`hermes_state.py:307-360`）：
- 双 FTS5 表：`messages_fts`（unicode61 分词器）+ `messages_fts_trigram`（三元组分词器，用于 CJK 子串搜索）
- 触发器自动同步：INSERT/UPDATE/DELETE 时维护索引
- CJK 搜索策略：3+ CJK 字符用 trigram，1-2 字符回退 LIKE
- FTS5 查询消毒：保留引用短语，剥离特殊字符

**并发与写入竞争处理**：

| 机制 | 位置 | 说明 |
|------|------|------|
| WAL 模式 + DELETE 回退 | `hermes_state.py:157-206` | NFS/SMB 文件系统不支持 WAL 时自动降级 |
| 应用级随机抖动重试 | `hermes_state.py:535-585` | 15 次重试，20-150ms 随机退避，打破 SQLite 确定性退避的队列效应 |
| 定期 WAL 检查点 | `hermes_state.py:384` | 每 50 次写入做一次 PASSIVE WAL 检查点 |
| 压缩锁 | `hermes_state.py:965-1068` | 单事务 DELETE 过期 + INSERT OR IGNORE + SELECT 确认，TTL 过期回收崩溃持有者 |

**声明式 schema 协调**（`hermes_state.py:666-708`）：参考 Beets/sqlite-utils 模式——`SCHEMA_SQL` 是唯一真实来源，启动时 diff 现有列并 ADD 缺失列。消除了版本门控迁移链中的列添加。

**生命周期管理**：
- 会话谱系链遍历（`get_compression_tip`）
- 压缩续体会话投影（`list_sessions_rich`）
- 自动维护（`maybe_auto_prune_and_vacuum`）：修剪 + VACUUM + FTS5 段合并

> **代码引用**：`hermes_state.py:363-3674` — SessionDB 完整实现

---

## 6. 通道/多平台支持

### 6.1 Gateway 架构

`gateway/run.py` 中的 `GatewayRunner` 是多平台消息网关的核心：

```
GatewayRunner
  ├── adapters: Dict[Platform, BasePlatformAdapter]
  ├── _agent_cache: OrderedDict[str, (AIAgent, config_signature)]  ← LRU 128, 1h TTL
  ├── _running_agents: Dict[str, Any]  ← 中断支持
  ├── session_store: SessionStore
  ├── delivery_router: DeliveryRouter
  └── cron_scheduler（每 60 秒 tick）
```

**AIAgent 缓存机制**（`run.py:17248-17316`）：每次消息到达时计算 `_agent_config_signature`（包含 model、runtime、toolsets、ephemeral prompt 等），签名匹配则复用缓存的 AIAgent 实例以保留 prompt cache 命中。

### 6.2 Platform 适配器注册

**PlatformRegistry 单例**（`gateway/platform_registry.py:162-260`）：

```python
@dataclass
class PlatformEntry:
    name: str
    label: str
    adapter_factory: Callable
    check_fn: Callable           # 依赖检查（SDK 是否安装）
    validate_config: Callable    # 配置验证
    is_connected: Callable       # 连接状态检查
    env_enablement_fn: Callable  # 从环境变量读取配置
    apply_yaml_config_fn: Callable  # YAML→环境变量桥接
    standalone_sender_fn: Callable  # 独立发送（cron 场景）
    cron_deliver_env_var: str    # cron 投递的 home channel 环境变量名
```

**适配器创建流程**（`run.py:6393-6472`）：
1. 先查询 `platform_registry`（插件适配器优先）
2. 未命中则进入内置 if/elif 链
3. 插件适配器创建后注入 `gateway_runner` 反向引用

**动态 Platform 枚举**（`gateway/config.py:131-173`）：`_missing_()` 类方法允许动态创建插件平台的伪成员，先扫描 `plugins/platforms/` 目录，再查询 `platform_registry`。

### 6.3 内置平台支持

Hermes 内置 22 个平台：

| 平台 | 类型 | 说明 |
|------|------|------|
| Telegram | 即时通讯 | 最成熟的支持 |
| Discord | 社区 | Guild/Channel/Thread 支持 |
| Slack | 企业 | Workspace/Channel 支持 |
| WhatsApp | 即时通讯 | 通过 WhatsApp Business API |
| Signal | 安全通讯 | — |
| Matrix | 去中心化 | — |
| Mattermost | 企业 | — |
| Feishu/飞书 | 企业 | 中国生态 |
| DingTalk/钉钉 | 企业 | 中国生态 |
| WeCom/企业微信 | 企业 | 中国生态 |
| WeChat/微信 | 即时通讯 | 中国生态 |
| QQ | 即时通讯 | 中国生态 |
| Email | 邮件 | — |
| SMS | 短信 | — |
| Home Assistant | 智能家居 | — |
| Webhook | 通用 | HTTP 回调 |
| API Server | 通用 | REST API |
| BlueBubbles | Apple | iMessage 桥接 |
| Yuanbao/元宝 | AI 平台 | 腾讯 |

### 6.4 BasePlatformAdapter — 适配器基类

`gateway/platforms/base.py` 定义了适配器基类：

**核心抽象方法**：
- `connect()` — 连接平台并开始接收消息
- `disconnect()` — 断开连接
- `send()` — 发送消息到指定 chat_id

**关键机制**：

| 机制 | 位置 | 说明 |
|------|------|------|
| 消息事件标准化 | `base.py:1289-1373` | 所有适配器将平台原始消息转换为 `MessageEvent` |
| 会话中断支持 | `base.py:1678-1681` | `_active_sessions` 跟踪中断 Event，`_session_tasks` 映射 Task |
| 忙状态处理 | `base.py:1682-1691` | queue（排队）和 interrupt（中断）两种模式 |
| 文本防抖 | `base.py:1692` | 快速连续消息的合并 |
| 媒体缓存 | `base.py:560-867` | 内容寻址存储，独立缓存目录 |
| 媒体投递安全 | `base.py:1006-1081` | 拒绝列表 + 可选严格模式 + 最近文件信任窗口 |

**MessageType 枚举**：TEXT、LOCATION、PHOTO、VIDEO、AUDIO、VOICE、DOCUMENT、STICKER、COMMAND

### 6.5 会话生命周期

**SessionSource**（`gateway/session.py:71-155`）：描述消息来源，包含 platform、chat_id、chat_type（dm/group/channel/thread）、user_id、thread_id、guild_id 等。

**会话键构建**（`gateway/session.py:600-665`）— 唯一真相源：
- DM: `agent:main:{platform}:dm:{chat_id}:{thread_id}`
- 群组: `agent:main:{platform}:{chat_type}:{chat_id}:{thread_id}:{participant_id}`（当 `group_sessions_per_user=True` 时隔离参与者）

**SessionResetPolicy**（`config.py:238-277`）：四种模式——daily、idle、both、none。支持按平台和会话类型覆盖。

**SessionStore**（`gateway/session.py:668-1311`）：
- SQLite（SessionDB）存储会话元数据和消息转录
- `sessions.json` 作为 session_key → session_id 的映射索引
- 自动过期：idle 超时、每日重置、或两者结合
- `suspend_session()` / `mark_resume_pending()` 处理网关重启后的会话恢复

### 6.6 消息投递管线

**DeliveryRouter**（`gateway/delivery.py:175-429`）：

```
消息到达
  │
  ├── 解析投递目标
  │   ├── "origin"    → 返回消息来源
  │   ├── "local"     → 保存到本地文件
  │   ├── "telegram"  → Telegram home channel
  │   ├── "telegram:123456" → 指定 Telegram 聊天
  │   └── "all"       → 所有已配置 home channel 的平台
  │
  ├── 静默叙述过滤（防止 bot-to-bot 消息镜像循环）
  │
  └── 超长内容截断到 3800 字符，完整输出保存到文件
```

### 6.7 频道发现与缓存

`gateway/channel_directory.py` 实现了频道自动发现：

- 启动时构建，每 5 分钟刷新，保存到 `~/.hermes/channel_directory.json`
- Discord: 枚举所有 guild 的 text_channels 和 forum_channels
- Slack: 通过 `users.conversations` API 枚举已加入的频道
- 其他平台: 从 `sessions.json` 的 origin 数据中发现
- `resolve_channel_name()` 支持精确匹配、guild 限定匹配、前缀匹配

### 6.8 事件钩子系统

`gateway/hooks.py` 实现了可扩展的钩子系统：

- 从 `~/.hermes/hooks/` 目录发现钩子
- 每个钩子目录包含 `HOOK.yaml`（元数据）+ `handler.py`（处理函数）
- 支持的事件类型：`gateway:startup`、`session:start`、`session:end`、`session:reset`、`agent:start`、`agent:step`、`agent:end`、`command:*`（通配符匹配）
- `emit_collect()` 用于决策型钩子，收集返回值（如命令策略 allow/deny/rewrite）

---

## 7. 对 Agent-Diva 的启示

### 7.1 架构层面

| 启示 | Hermes 实现 | Agent-Diva 现状 | 建议 |
|------|------------|----------------|------|
| **3 层系统提示词缓存** | stable/context/volatile 分层，字节稳定设计 | 缺乏分层缓存策略 | **高优先级**：引入分层系统提示词，最大化 LLM provider 缓存命中 |
| **胖 Agent + 模块提取** | AIAgent 门面 + 子模块函数 | AgentLoop 基于消息总线 | 保持现有消息总线架构，但引入类似的模块提取模式 |
| **迭代预算 + 退还** | consume/refund 模式，execute_code 退还 | 无类似机制 | 引入 IterationBudget，防止无限工具调用循环 |

### 7.2 工具系统

| 启示 | Hermes 实现 | Agent-Diva 现状 | 建议 |
|------|------------|----------------|------|
| **自注册 + AST 发现** | 模块级 register + AST 扫描 | Tool trait + 手动注册 | 考虑类似自注册模式减少样板代码 |
| **Toolset 分组** | 分层 toolset，组合 includes | 无类似机制 | 引入工具集分组，支持平台特定和安全子集 |
| **MCP 支持** | 完整的 MCP 桥接（stdio/HTTP/SSE） | 无 MCP 支持 | **高优先级**：实现 MCP 工具桥接，这是生态扩展的关键 |
| **Progressive Disclosure** | BM25 检索 + 3 个桥接工具 | 无类似机制 | 当工具数量增长时考虑按需加载 |
| **工具并发执行** | 路径冲突检测 + 并发决策 | 纯串行执行 | 引入工具并发执行，降低多工具延迟 |

### 7.3 A2A 能力

| 启示 | Hermes 实现 | Agent-Diva 现状 | 建议 |
|------|------------|----------------|------|
| **delegate_task 安全模型** | 工具黑名单 + 深度控制 + 角色模型 | SubagentManager（轻量 spawn） | 增强子 agent 安全模型，引入工具黑名单和深度控制 |
| **Kanban 看板** | SQLite 持久化 + 依赖门控 + Worker 所有权守卫 | 无类似机制 | 考虑引入结构化任务编排，支持多 agent 协作 |
| **文件状态协调** | 子 agent 修改文件后通知父 agent 重读 | 无类似机制 | 引入文件变更通知机制 |
| **心跳/超时诊断** | 心跳线程 + 超时诊断转储 | 无类似机制 | 引入子 agent 健康监控 |

### 7.4 记忆系统

| 启示 | Hermes 实现 | Agent-Diva 现状 | 建议 |
|------|------------|----------------|------|
| **冻结快照模式** | 系统提示稳定 vs 工具响应实时 | MemoryProvider 接口 | 引入冻结快照，保持前缀缓存稳定 |
| **Memory Provider 接口** | ABC + 完整生命周期钩子 | MemoryProvider trait | 参考其生命周期钩子设计（on_session_switch, on_memory_write 等） |
| **威胁模式扫描** | 写入前扫描注入/外泄模式 | 无类似机制 | 引入记忆写入安全检查 |
| **SessionDB 设计** | SQLite + FTS5 + 声明式 schema 协调 | JSONL 持久化 | 考虑迁移到 SQLite，获得 FTS5 搜索和更好的并发支持 |

### 7.5 通道/多平台

| 启示 | Hermes 实现 | Agent-Diva 现状 | 建议 |
|------|------------|----------------|------|
| **PlatformRegistry** | 插件优先 + 动态枚举 | ChannelHandler trait | 借鉴 `check_fn` / `validate_config` / `is_connected` 三层验证 |
| **会话键构建** | `build_session_key()` 统一 DM/群组/话题/线程 | Session Manager | 参考其 `group_sessions_per_user` 配置 |
| **AIAgent 缓存** | 签名匹配 + LRU 缓存 | 无类似机制 | 引入 per-session agent 缓存，提升 prompt cache 命中率 |
| **事件钩子系统** | HOOK.yaml + handler.py | 无类似机制 | 引入可扩展的钩子系统 |
| **Cron 系统** | no_agent + LLM 两种模式 + context_from 链式引用 | cron 命令 | 参考其 at-most-once 语义和作业链式引用 |

### 7.6 错误处理

| 启示 | Hermes 实现 | Agent-Diva 现状 | 建议 |
|------|------------|----------------|------|
| **错误分类管道** | 8 级优先级 + ClassifiedError 数据类 | 简单 try/catch | **高优先级**：引入结构化错误分类，支持自动恢复策略 |
| **402 消歧义** | 区分支付错误和瞬时限速 | 无类似机制 | 在 provider 层引入错误消歧义 |
| **断路器** | 连续 3 次失败后打开（60 秒冷却） | 无类似机制 | 引入断路器防止重试风暴 |

### 7.7 安全模型

Hermes 的安全设计值得 Agent-Diva 全面借鉴：

| 安全层 | Hermes 实现 | 建议 |
|--------|------------|------|
| 工具黑名单 | `DELEGATE_BLOCKED_TOOLS` | 子 agent 工具访问控制 |
| Worker 所有权守卫 | `_enforce_worker_task_ownership()` | 防止 worker 越权操作 |
| MCP 环境变量过滤 | `_build_safe_env()` | 防止泄露宿主 API key |
| 凭证脱敏 | `_sanitize_error()` | 错误消息中的凭证保护 |
| Prompt 注入扫描 | `_scan_mcp_description()` | 工具描述安全检查 |
| 威胁模式扫描 | `threat_patterns` strict 范围 | 记忆写入安全检查 |
| 路径遍历防护 | `has_traversal_component` + `validate_within_dir` | 文件操作安全 |
| 隔离区扫描 | 下载 → 隔离 → 安全扫描 → 安装 | 插件/扩展安装安全 |

---

## 附录：关键文件索引

### Agent Loop
- `run_agent.py:294` — AIAgent 类定义
- `agent/conversation_loop.py:351` — run_conversation 主循环
- `agent/tool_executor.py:110-538` — 并发执行
- `agent/tool_executor.py:542-1008` — 串行执行
- `agent/system_prompt.py:61-345` — 3 层系统提示词
- `agent/context_compressor.py:522-2078` — 上下文压缩
- `agent/iteration_budget.py:17-62` — 迭代预算
- `agent/error_classifier.py:438-720` — 错误分类管道

### 工具系统
- `tools/registry.py:151-519` — ToolRegistry
- `model_tools.py:802-1038` — handle_function_call
- `toolsets.py:88-551` — Toolset 定义
- `tools/mcp_tool.py:1096-1860` — MCPServerTask
- `tools/tool_search.py:63-510` — Progressive Disclosure

### A2A
- `tools/delegate_tool.py:870-1174` — _build_child_agent
- `tools/kanban_tools.py:49-91` — 门控检查
- `cron/scheduler.py:1857-2035` — tick()
- `cron/jobs.py` — 作业存储

### 技能系统
- `agent/skill_utils.py:88-122` — Frontmatter 解析
- `agent/skill_commands.py:263-326` — 技能扫描
- `tools/skills_tool.py:632-1396` — 渐进式披露
- `tools/skills_hub.py:3607-3631` — 源路由器

### 记忆系统
- `tools/memory_tool.py:113-600` — MemoryStore
- `agent/memory_manager.py:244-641` — MemoryManager
- `agent/memory_provider.py:42-292` — MemoryProvider ABC
- `hermes_state.py:363-3674` — SessionDB

### 通道/多平台
- `gateway/run.py:1663` — GatewayRunner
- `gateway/platform_registry.py:162-260` — PlatformRegistry
- `gateway/platforms/base.py:1647-2009` — BasePlatformAdapter
- `gateway/session.py:600-665` — build_session_key
- `gateway/delivery.py:175-429` — DeliveryRouter
- `gateway/channel_directory.py:60-109` — 频道发现
- `gateway/hooks.py:35-210` — HookRegistry
