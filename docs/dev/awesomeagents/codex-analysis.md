# OpenAI Codex CLI 架构深度分析

> 调研时间：2026-06-01
> 目标项目：`.workspace/codex/`（OpenAI Codex CLI，Rust 实现）
> 调研目的：理解 Codex 作为 agent 运行时的架构设计，为 agent-diva 的编排层提供参考

---

## 目录

1. [项目概览](#1-项目概览)
2. [Agent Loop 机制](#2-agent-loop-机制)
3. [工具链系统](#3-工具链系统)
4. [A2A 能力（多 Agent 协作）](#4-a2a-能力多-agent-协作)
5. [配置和扩展](#5-配置和扩展)
6. [与 Claude Code 的对比](#6-与-claude-code-的对比)
7. [对 agent-diva 的启示](#7-对-agent-diva-的启示)

---

## 1. 项目概览

### 1.1 项目结构

Codex CLI 是一个 Rust monorepo，位于 `codex-rs/` 目录下。核心 crate 包括：

| Crate | 路径 | 职责 |
|---|---|---|
| `codex-cli` | `codex-rs/cli/` | CLI 入口，子命令分发 |
| `codex-core` | `codex-rs/core/` | **核心运行时**：agent loop、会话管理、上下文管理 |
| `codex-tools` | `codex-rs/tools/` | 工具定义与规范 |
| `codex-protocol` | `codex-rs/protocol/` | Op/Event 协议定义 |
| `codex-arg0` | `codex-rs/arg0/` | 单二进制多行为分发 |
| `codex-mcp` | `codex-rs/mcp/` | MCP 客户端集成 |
| `codex-tui` | `codex-rs/tui/` | 终端 UI（ratatui） |
| `codex-app-server` | `codex-rs/app-server/` | IDE 扩展的 JSON-RPC 服务器 |
| `codex-state` | `codex-rs/state/` | SQLite 持久化 |
| `codex-thread-store` | `codex-rs/thread-store/` | 会话存储与回放 |

### 1.2 关键设计模式

- **单二进制多路复用**：通过 `arg0_dispatch` 机制，同一个二进制根据调用名称（`codex`、`codex-linux-sandbox`、`codex-apply-patch`）执行不同行为
- **事件驱动架构**：UI 层（TUI、app-server、exec）通过类型化的 `Op`/`Event` 通道与 core 通信
- **可插拔沙箱**：平台特定的沙箱执行（macOS Seatbelt、Linux Landlock+bubblewrap、Windows 受限令牌）统一在 `SandboxManager` 接口之后

---

## 2. Agent Loop 机制

### 2.1 核心抽象

```
Codex (高层会话句柄)
  └── Session (运行时上下文)
        ├── ContextManager (对话历史 + token 追踪)
        ├── ModelClient (模型 API 通信)
        ├── ToolRouter (工具分发)
        ├── AgentControl (子 agent 管理)
        └── Mailbox (agent 间通信)
```

- **`Session`**：一个 agent 实例的完整运行时，持有会话 ID、事件通道、agent 状态、MCP 连接、邮箱等。同一时刻最多运行一个任务，可被用户输入中断
- **`CodexThread`**：围绕 `Codex` 的薄封装，提供双向消息流，暴露 `submit(Op)` 接口
- **`Codex`**：高层会话句柄，通过 `Codex::spawn()` 创建，负责组装配置、认证、模型、环境、技能、插件、MCP、对话历史和 agent 控制平面

### 2.2 Turn 循环（`run_turn`）

核心 agent 循环位于 `core/src/session/turn.rs`，流程如下：

```
用户输入 → 预采样压缩检查 → 上下文更新 → 构建工具路由
    → 采样循环（发送到模型）
        → 模型返回工具调用 → 执行工具 → 结果回传 → 继续循环
        → 模型返回文本消息 → Turn 结束
    → 自动压缩（token 超限时）
```

关键特性：
1. **预采样压缩**：在发送到模型之前，如果对话历史接近 token 限制，先运行压缩
2. **上下文片段注入**：模块化的上下文片段（用户指令、权限说明、环境信息、技能列表等）根据会话状态动态注入
3. **并行工具执行**：单次模型响应中的多个工具调用可通过 `ToolCallRuntime` 并发执行
4. **自动压缩**：Turn 执行过程中如果 token 超限，自动内联压缩历史后继续

### 2.3 Op/Event 协议

UI 与 core 之间通过类型化协议通信：

**Op（UI → Session）**：
- `UserInput` / `UserTurn` — 用户消息
- `Interrupt` — 中断当前 turn
- `ReviewDecision` — 审批/拒绝工具执行
- `SessionConfigUpdate` — 运行时配置变更

**Event（Session → UI）**：
- `TurnStarted` / `TurnComplete` — Turn 生命周期
- `AgentMessage` / `AgentReasoning` — 模型输出
- `ExecApprovalRequest` — 审批请求
- `ContextCompacted` — 历史压缩通知
- `TokenCount` — 用量追踪

这种设计使得 TUI、app-server、exec 模式可以共享同一个 core 实现。

### 2.4 上下文管理

**ContextManager** 维护对话记录为 `Vec<ResponseItem>`，追踪：
- Token 用量和分布
- 历史版本号（压缩/回滚时递增）
- 引用上下文快照（用于差异比较，生成最小化的上下文更新）

**上下文片段**（`core/src/context/`）是模块化的注入单元：
- `UserInstructions` — AGENTS.md 内容
- `PermissionsInstructions` — 沙箱/审批模式信息
- `EnvironmentContext` — 工作目录、OS、平台信息
- `AvailableSkillsInstructions` — 已发现的技能
- `CollaborationModeInstructions` — Plan/Execute 模式
- `SubagentNotification` — 子 agent 通知

### 2.5 模型客户端

**ModelClient** 处理与模型 API 的通信：
- 支持 **SSE（HTTP 流式）** 和 **WebSocket** 传输，自动降级
- WebSocket 预热：通过 `response.create` + `generate=false` 预连接
- 支持通过 `/responses/compact` 端点进行远程压缩
- 支持通过 `/memories/trace_summarize` 进行记忆摘要

### 2.6 会话持久化

- **SQLite 持久化**：通过 `codex-state` crate 存储会话元数据
- **Rollout 文件**：对话历史以文件形式存储，支持回放
- **会话恢复**：`codex resume <SESSION_ID>` 或 `codex resume --last`
- **会话分叉**：`codex fork <SESSION_ID>` 从已有历史创建新会话

---

## 3. 工具链系统

### 3.1 工具架构

工具定义在 `codex-tools` crate 中，由 `ToolRouter` 分发。`ToolSpec` 定义了工具类型：

| 类型 | 说明 |
|---|---|
| `Function` | 标准函数调用工具（大多数内置工具） |
| `Namespace` | 命名空间工具组（MCP 工具） |
| `ToolSearch` | 动态工具发现 |
| `LocalShell` | 原生 shell 执行（非函数调用） |
| `WebSearch` | 内置网页搜索 |
| `ImageGeneration` | 内置图像生成 |

### 3.2 内置工具

| 工具 | 说明 |
|---|---|
| `shell` / `exec_command` | 在沙箱 PTY 中执行 shell 命令 |
| `apply_patch` | 应用 unified diff 修改文件 |
| `update_plan` | 追踪多步骤任务计划 |
| `view_image` | 查看工作区中的图片 |
| `js_repl` | 通过 Node.js 执行 JavaScript |
| `list_dir` | 列出目录内容 |
| `request_user_input` | 向用户提问 |
| `request_permissions` | 请求提升权限 |
| `tool_search` | 搜索可用工具 |
| `tool_suggest` | 向用户建议工具 |
| `unified_exec` | 实验性统一执行工具 |

### 3.3 多 Agent 工具

| 工具 | 说明 |
|---|---|
| `spawn_agent` / `spawn_agents_on_csv` | 生成子 agent |
| `wait_agent` | 等待子 agent 完成 |
| `close_agent` | 终止子 agent |
| `send_input` / `send_message` | 向子 agent 发送消息 |
| `list_agents` | 列出活跃 agent |
| `followup_task` | 创建后续任务 |
| `resume_agent` | 恢复已完成的 agent |

### 3.4 权限模型

分层权限体系：

```
SandboxPolicy（全局策略）
  ├── ReadOnly — 只读
  ├── WorkspaceWrite — 工作区可写
  └── DangerFullAccess — 完全访问

AskForApproval（审批策略）
  ├── Never — 从不询问
  ├── OnRequest — 工具请求时
  ├── UnlessTrusted — 除非已信任
  ├── OnFailure — 失败时
  └── Granular — 细粒度

FileSystemSandboxPolicy — 细粒度文件系统读/写/拒绝
NetworkSandboxPolicy — 网络访问控制
PermissionProfile — 组合权限配置，可由用户授予
```

审批流程：
1. 工具处理器检查当前策略下是否允许执行
2. 如需审批，发送 `ExecApprovalRequestEvent` 到 UI
3. UI 呈现给用户（或 Guardian 审查者）
4. 用户回复 `ReviewDecision`（批准、批准-会话级、拒绝）
5. 授予的权限可作为"粘性"授权持久化

### 3.5 MCP 集成

- **MCP 客户端**（`codex-mcp/`）：连接外部 MCP 服务器，发现和调用工具
- **MCP 服务器**（`mcp-server/`）：Codex 自身可作为 MCP 服务器，让其他 agent 将 Codex 当作工具使用
- 支持 OAuth 认证的 MCP 服务器
- 技能可声明 MCP 依赖，自动安装

### 3.6 与 Claude Code 工具链对比

| 维度 | Codex CLI | Claude Code |
|---|---|---|
| 文件读写 | `shell` + `apply_patch`（通过 shell 或 unified diff） | `Read` / `Write` / `Edit`（专用工具） |
| Shell 执行 | `shell`（沙箱 PTY） | `Bash` / `PowerShell` |
| 搜索 | `shell` + grep/find 或 `web_search` | `Grep` / `Glob` / `WebSearch` |
| 代码搜索 | 依赖 shell 组合 | `Agent` (Explore) 专用搜索 agent |
| 工具发现 | `tool_search` / `tool_suggest` | MCP 动态发现 |
| 图像生成 | 内置 `ImageGeneration` | 无内置 |
| JS 执行 | 内置 `js_repl` | 无内置 |
| 子 agent | 原生 `spawn_agent` 等工具 | `Agent` 工具（通用子 agent） |
| MCP | 客户端 + 服务器双角色 | 仅客户端 |
| 沙箱 | 平台级（Seatbelt/Landlock/Windows） | 无内置沙箱，依赖用户审批 |

---

## 4. A2A 能力（多 Agent 协作）

### 4.1 子 Agent 系统

Codex 内置了完整的多 agent 系统：

**核心组件**：
- **`AgentRegistry`**：所有 agent 共享的注册表，追踪活跃 agent、强制线程限制、管理 agent 昵称
- **`AgentControl`**：控制平面句柄，用于生成和消息传递。父线程和所有子 agent 共享
- **`Mailbox`**：agent 间通信通道，使用无界 mpsc 通道 + 单调序列号 + `watch` 通知

**内置角色**：
- `default` — 标准 agent
- `explorer` — 快速代码库探索（降低推理强度）
- `worker` — 执行/生产工作（带所有权规则）

用户可通过 `agent_roles` 配置自定义角色，每个角色有独立的配置层（模型、推理强度等）。

### 4.2 Spawn 流程

当模型调用 `spawn_agent` 时：
1. `AgentControl::reserve_spawn_slot()` 检查线程限制
2. 从昵称池预留一个昵称
3. 通过 `run_codex_thread_interactive()` 生成新的 `CodexThread`
4. 子 agent 获得自己的 `Config`（可被角色修改）、`Session`、`AgentControl`（从父级共享）
5. 父级收到子线程 ID，可通过 mailbox 通信

### 4.3 限制

- `agent_max_threads`（默认 6）：最大并发子 agent 数
- `agent_max_depth`（默认 1）：最大嵌套深度（根=0，首次 spawn=1）
- `agent_job_max_runtime_seconds`：子 agent 最大运行时间

### 4.4 与 Claude Code 的 A2A 对比

| 维度 | Codex CLI | Claude Code |
|---|---|---|
| 子 agent 生成 | 原生工具 `spawn_agent` | `Agent` 工具 |
| agent 间通信 | Mailbox（mpsc 通道） | 无直接通信，通过父级中转 |
| 角色系统 | 内置角色 + 自定义角色 | 无内置角色（子 agent 类型固定） |
| 并发限制 | 可配置（默认 6 线程） | 无显式限制 |
| 嵌套深度 | 可配置（默认 1 层） | 无显式限制 |
| agent 恢复 | `resume_agent` 工具 | 不支持 |
| agent 列表 | `list_agents` 工具 | 不支持 |

**结论**：Codex 的 A2A 能力显著强于 Claude Code。它有完整的 agent 生命周期管理（生成、通信、等待、恢复、终止），而 Claude Code 的子 agent 更像是"fire-and-forget"的任务委托。

---

## 5. 配置和扩展

### 5.1 配置体系

分层配置，通过 `ConfigBuilder` 加载（优先级从低到高）：

1. 内置默认值
2. 全局用户配置 `~/.codex/config.toml`
3. 项目配置 `.codex/config.toml`
4. 命名配置文件（profiles）
5. 云需求（企业托管约束）
6. CLI 覆盖（`-c key=value`）
7. Harness 覆盖（调用方编程覆盖）

**关键配置项**：
- `model` / `model_provider` — 模型选择
- `approval_policy` — 审批策略
- `sandbox_mode` — 沙箱级别
- `mcp_servers` — MCP 服务器连接
- `agents` — 多 agent 配置（`max_threads`、`max_depth`、`job_max_runtime_seconds`）
- `[features]` — 功能标志
- `personality` — 模型人格

### 5.2 技能系统

- `.codex/skills/` 目录下的 Markdown 文件定义技能
- 技能可声明 MCP 依赖
- 通过 `AvailableSkillsInstructions` 上下文片段注入模型上下文

### 5.3 扩展机制

- **MCP 服务器**：通过配置添加外部工具
- **自定义 agent 角色**：在 config.toml 中定义
- **App Server**：IDE 扩展通过 JSON-RPC 连接
- **Hook 系统**：会话钩子可注入额外上下文

---

## 6. 与 Claude Code 的对比

### 6.1 架构差异

| 维度 | Codex CLI | Claude Code |
|---|---|---|
| **语言** | Rust | TypeScript (Node.js) |
| **运行时模型** | 事件驱动（Op/Event 通道） | 交互式 REPL |
| **UI 层** | TUI (ratatui) + App Server + Exec | CLI (ink/React) + Web + IDE 扩展 |
| **模型通信** | SSE + WebSocket（自动降级） | HTTP streaming |
| **沙箱** | 平台级（Seatbelt/Landlock/Windows） | 无内置沙箱 |
| **持久化** | SQLite + Rollout 文件 | JSONL 文件 |
| **配置格式** | TOML | JSON (settings.json) |
| **单二进制** | 是（arg0 分发） | 否（Node.js 脚本） |

### 6.2 工具能力差异

| 能力 | Codex CLI | Claude Code |
|---|---|---|
| 文件操作 | 通过 shell + apply_patch | 专用 Read/Write/Edit 工具 |
| 代码搜索 | 依赖 shell | 专用 Grep/Glob + Explore agent |
| 图像生成 | 内置 | 无 |
| JS 执行 | 内置 js_repl | 无 |
| 网页搜索 | 内置 | 内置 |
| MCP 角色 | 客户端 + 服务器 | 仅客户端 |
| 工具发现 | tool_search 动态发现 | 静态注册 |

### 6.3 编排难度对比

| 维度 | Codex CLI | Claude Code |
|---|---|---|
| 作为子 agent 调用 | `exec` 模式（非交互） | CLI 模式 |
| 输入输出协议 | Op/Event（结构化） | stdin/stdout（文本流） |
| 会话管理 | resume/fork 原生支持 | 无原生会话管理 |
| 进程控制 | Interrupt 原生支持 | 需外部 kill |
| 结果提取 | EventMsg 结构化解析 | 需解析文本输出 |
| 编排复杂度 | **低**（协议清晰） | **中**（需文本解析） |

### 6.4 适用场景分析

**优先选择 Codex 的场景**：
- 需要平台级沙箱隔离（安全敏感环境）
- 需要多 agent 并行协作（复杂任务分解）
- 需要结构化的 agent 编排（作为子 agent 被调用）
- 需要会话恢复/分叉（长时间任务）
- 需要 MCP 服务器能力（让其他 agent 调用自己）
- 需要 JS REPL（前端/Node.js 开发）

**优先选择 Claude Code 的场景**：
- 需要更精细的文件操作（Read/Write/Edit 工具更高效）
- 需要深度代码搜索（Grep/Glob + Explore agent 组合）
- 需要丰富的技能生态（oh-my-claudecode 等）
- 需要与现有 TypeScript/Node.js 工具链集成
- 需要更灵活的子 agent 类型（多种专用 agent）

---

## 7. 对 agent-diva 的启示

### 7.1 值得借鉴的设计

#### 7.1.1 Op/Event 协议

Codex 的 `Op`/`Event` 协议是其最大的架构亮点。agent-diva 的 message bus 已经有类似概念（inbound/outbound），但可以进一步规范化：

- 将 agent 交互协议标准化为类型化的 Op/Event
- 使得不同的前端（CLI、GUI、API）可以共享同一个 agent core
- 便于录制和回放（类似 Codex 的 rollout 机制）

#### 7.1.2 分层沙箱

Codex 的 `SandboxPolicy` + `AskForApproval` + `PermissionProfile` 三层权限模型值得借鉴：

- **全局策略**：定义默认安全级别
- **审批策略**：定义何时需要用户确认
- **权限配置**：细粒度的文件系统/网络权限

agent-diva 可以在 tool 层引入类似的权限模型，而不是完全依赖 LLM 的判断。

#### 7.1.3 Agent 角色系统

Codex 的内置角色（explorer、worker）+ 自定义角色机制很有价值：

- **explorer**：降低推理强度，快速探索代码库
- **worker**：标准执行角色，带所有权规则

agent-diva 可以为不同的子 agent 任务定义角色模板，而不是每次都从零配置。

#### 7.1.4 上下文片段机制

Codex 的 `ContextFragment` 设计使得上下文注入模块化：

- 每个片段独立管理自己的内容和更新逻辑
- 支持差异比较，只发送变化的部分
- 便于扩展新的上下文类型

### 7.2 场景选择策略

在 agent-diva 的编排层中，建议按以下策略选择子 agent：

| 场景 | 推荐 | 原因 |
|---|---|---|
| 日常代码编辑 | Claude Code | 文件操作工具更精细 |
| 代码搜索/探索 | Claude Code (Explore) | Grep/Glob + 专用 agent |
| 安全敏感操作 | Codex | 平台级沙箱隔离 |
| 多任务并行 | Codex | 原生多 agent 支持 |
| 长时间任务 | Codex | 会话恢复/分叉 |
| 需要 MCP 服务 | Codex | 可作为 MCP 服务器 |
| 复杂推理 | Claude Code (Opus) | Claude 模型推理能力更强 |
| 快速执行 | Codex | exec 模式开销更低 |

### 7.3 集成到 agent-diva 的最佳实践

#### 7.3.1 作为子 agent 调用

```rust
// 推荐使用 exec 模式（非交互）
// 输入：通过 stdin 或临时文件传递任务描述
// 输出：解析 EventMsg 结构化事件

// 命令行模式
codex exec --model o3 "任务描述"

// 或通过 MCP 协议
// agent-diva 作为 MCP 客户端连接到 codex mcp-server
```

#### 7.3.2 会话管理

利用 Codex 的 resume/fork 能力：
- 长任务中断后可恢复（`codex resume`）
- 从历史会话分叉新任务（`codex fork`）
- agent-diva 可存储 session ID 用于后续引用

#### 7.3.3 权限预配置

通过 config.toml 预配置权限，避免运行时审批中断：

```toml
# .codex/config.toml
sandbox_mode = "workspace-write"
approval_policy = "never"  # 或 "on-request"
```

#### 7.3.4 与 Claude Code 的互补

```
agent-diva 编排层
  ├── Claude Code — 精细文件操作、代码搜索、复杂推理
  ├── Codex CLI — 沙箱执行、多 agent 并行、MCP 服务
  └── 其他 agent — 按需扩展
```

### 7.4 总结

Codex CLI 的核心优势在于：
1. **结构化的 agent 协议**（Op/Event）— 便于编排
2. **平台级沙箱** — 安全性更强
3. **原生多 agent 系统** — 并行能力更强
4. **会话持久化** — 支持恢复和分叉

Claude Code 的核心优势在于：
1. **精细的文件操作工具** — 编辑效率更高
2. **强大的代码搜索** — Grep/Glob + Explore 组合
3. **丰富的技能生态** — oh-my-claudecode 等
4. **灵活的子 agent 类型** — 更多专用 agent

对于 agent-diva，建议：
- **默认使用 Claude Code** — 其工具链更适合日常编码任务
- **Codex 作为特定场景的补充** — 需要沙箱、并行、MCP 服务时
- **借鉴 Codex 的协议设计** — 规范化 agent-diva 的 agent 间通信
- **借鉴 Codex 的角色系统** — 为不同任务定义 agent 角色模板
