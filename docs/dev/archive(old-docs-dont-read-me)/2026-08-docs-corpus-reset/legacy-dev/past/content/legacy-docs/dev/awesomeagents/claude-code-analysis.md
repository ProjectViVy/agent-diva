# Claude Code Agent 运行时架构深度分析

> 基于 `.workspace/claude-code`（v2.6.6 源码）和 `.workspace/learn-claude-code`（20 章教学项目）的调研
>
> 调研日期：2026-06-01
>
> 目标：理解 Claude Code 作为 agent 运行时的内部机制，为 agent-diva 编排 Claude Code 子 agent 提供架构依据

---

## 核心结论

**Claude Code = 一个 while True 循环 + 完整的 harness 层。**

Agency（感知、推理、行动的能力）来自模型训练，不是来自代码。Claude Code 的价值在于它是一个极其精良的 harness——给模型提供了工具、知识、上下文管理和权限边界，然后让路。理解这个 harness 的每一个组件，是高效编排 Claude Code 作为子 agent 的前提。

```
Claude Code = one agent loop
            + tools (bash, read, write, edit, glob, grep, browser...)
            + on-demand skill loading
            + context compaction
            + subagent spawning
            + task system with dependency graphs
            + async mailbox team coordination
            + worktree-isolated parallel execution
            + permission governance
            + hooks extension system
            + memory persistence
            + MCP external capability routing
```

> 来源：learn-claude-code/README.md

---

## 1. Agent Loop 机制

### 1.1 核心循环

Claude Code 的 agent loop 是一个 `while(true)` 异步生成器（`src/query.ts` 中的 `queryLoop()`），实现了经典的 think-act-observe 循环：

```
用户输入
  → UserPromptSubmit hooks
  → cron/background 通知注入
  → context compact（四层压缩管线）
  → memory + skills + MCP 状态组装 system prompt
  → LLM streaming API call
  → has tool_use block?
      否 → Stop hooks → 返回文本
      是 → PreToolUse hooks + permission check
          → TOOL_HANDLERS / MCP handlers / background dispatch
          → PostToolUse hooks
          → tool_result 追加到 messages
          → 下一轮循环
```

> 来源：learn-claude-code/s20_comprehensive/README.md，s01_agent_loop/README.md

**关键设计决策：**

| 决策 | 说明 |
|------|------|
| 终止信号 | 不依赖 `stop_reason == "tool_use"`，而是检查响应中是否实际出现 tool_use block |
| 工具并发 | 只读工具（read, glob, grep）并发执行；写入工具（write, edit, bash）串行执行 |
| 上下文压缩 | 每轮 LLM 调用前运行四层压缩管线，便宜的先跑贵的后跑 |
| 错误恢复 | 16+ 种 reason/transition code，精确指数退避 + 抖动 |

> 来源：claude-code/src/query.ts, src/services/tools/toolOrchestration.ts

### 1.2 工具编排

工具执行由 `runTools()` 生成器（`src/services/tools/toolOrchestration.ts`）管理：

- 将工具调用分区为**并发批次**（只读操作）和**串行批次**（写入操作）
- 每个工具调用经过 `runToolUse()`（`src/services/tools/toolExecution.ts`）处理：权限检查 → 执行 → 分析 → 错误处理
- PreToolUse/PostToolUse hooks 在工具执行前后注入（`src/services/tools/toolHooks.ts`）

> 来源：claude-code/src/services/tools/toolOrchestration.ts, toolExecution.ts, toolHooks.ts

### 1.3 上下文管理策略

Claude Code 的上下文管理是多层协同的系统：

**CLAUDE.md 分层注入：**
- 全局 `~/.claude/CLAUDE.md` — 用户级指令
- 项目 `CLAUDE.md` — 项目级指令（checked into repo）
- `.claude/` 目录下的 rules、agents、skills 等

**System Prompt 组装（`src/context.ts`）：**
- 静态 sections（始终加载）vs 动态 sections（状态依赖）
- 三层缓存：lodash memoize → section registry cache → API-level cache
- `mcp_instructions` 是唯一的 volatile section（使用 `DANGEROUS_uncachedSystemPromptSection()`）
- 标准交互模式 system prompt 约 20-30KB

> 来源：learn-claude-code/s10_system_prompt/README.md（分析了 constants/prompts.ts 914 行）

**四层压缩管线（`src/services/compact/`）：**

| 层级 | 名称 | 作用 | 成本 |
|------|------|------|------|
| L1 | snip_compact | 裁掉无关的旧对话中间部分 | 0 API |
| L2 | micro_compact | 旧 tool_result 替换为占位符 | 0 API |
| L3 | tool_result_budget | 大结果落盘，只保留摘要 | 0 API |
| L4 | autoCompact | LLM 生成摘要（仅在 token 超阈值时触发） | 1 API |

> 来源：learn-claude-code/s08_context_compact/README.md

### 1.4 会话管理和恢复

Claude Code 的会话系统（`src/utils/session*.ts`）包括：

- **sessionState** — 会话状态管理
- **sessionStorage** — 会话持久化（JSONL 格式）
- **sessionRestore** — 会话恢复
- **sessionTranscript** — 会话转录
- **sessionMemory** — 会话记忆（区分于用户记忆）

会话可通过 `--resume` 或 `--fork` 恢复/分叉。`-p` (print) 模式下会话是非交互式的，适合被编排。

---

## 2. 工具链系统

### 2.1 内置工具清单（50+）

Claude Code 在 `packages/builtin-tools/` 中实现了 50+ 个内置工具：

**核心文件操作：**
- `FileReadTool`, `FileWriteTool`, `FileEditTool` — 文件读写编辑
- `GlobTool`, `GrepTool` — 搜索和导航

**Shell 执行：**
- `BashTool` — Shell 命令执行
- `PowerShellTool` — PowerShell 执行（Windows）

**Web 能力：**
- `WebFetchTool`, `WebSearchTool`, `WebBrowserTool` — Web 获取/搜索/浏览器

**任务管理：**
- `TodoWriteTool`, `TaskCreateTool`, `TaskGetTool`, `TaskListTool`, `TaskUpdateTool`, `TaskStopTool`, `TaskOutputTool`

**Agent 协作：**
- `AgentTool` — 子 agent 生成
- `TeamCreateTool`, `TeamDeleteTool`, `SendMessageTool` — 团队协调
- `EnterWorktreeTool`, `ExitWorktreeTool` — Git worktree 隔离

**计划模式：**
- `EnterPlanModeTool`, `ExitPlanModeTool`, `VerifyPlanExecutionTool`

**后台任务：**
- `MonitorTool`, `CronCreateTool`, `CronDeleteTool`, `CronListTool`

**MCP 集成：**
- `MCPTool`, `ListMcpResourcesTool`, `ReadMcpResourceTool`, `McpAuthTool`

**其他：**
- `SkillTool` — 技能执行
- `LSPTool` — 语言服务器集成
- `NotebookEditTool` — Jupyter notebook
- `PushNotificationTool` — 推送通知
- `WorkflowTool` — 工作流执行
- `SnipTool` — 上下文裁剪
- `SearchExtraToolsTool` — 动态工具发现

> 来源：claude-code/packages/builtin-tools/src/tools/ 目录

### 2.2 权限模型

Claude Code 的权限系统是多层的：

**权限模式（PermissionMode）：**
- `default` — 默认模式，需要用户审批
- `plan` — 计划模式，只允许只读操作
- `auto` — 自动模式，减少审批提示
- `bypassPermissions` — 跳过所有权限（危险）
- `bubble` — 权限冒泡到父 agent

**权限行为（PermissionBehavior）：**
- `allow` — 允许
- `deny` — 拒绝
- `ask` — 询问用户
- `passthrough` — 传递给下一层

**权限规则来源（8 个配置源）：**
userSettings, projectSettings, localSettings, flagSettings, policySettings, cliArg, command, session

**YoloClassifier：** 在 auto 模式下，工具调用 + 上下文会被发送给一个分类器 LLM 来自动判断是否允许执行。

**权限冒泡：** 子 agent 的权限对话框会冒泡到父终端。

> 来源：learn-claude-code/s03_permission/README.md（分析了 types/permissions.ts, utils/permissions/permissions.ts, yoloClassifier.ts）

### 2.3 MCP 集成

MCP（Model Context Protocol）是 Claude Code 扩展工具能力的标准协议：

**6 种传输类型：** stdio, sse, http, ws, sse-ide, sdk

**工具池合并：** 每轮循环时 `assemble_tool_pool()` 将内置工具和 MCP 工具合并。MCP 工具命名格式为 `mcp__server__tool`，避免不同 server 的工具名冲突。

**完整 OAuth 2.0 + PKCE 流：** 支持 token 刷新和跨应用访问。

**Channel 通知：** MCP server 可以向 agent 推送消息。

**连接生命周期：** 终端错误处理、401 处理、可配置超时、stdio 断开时 SIGINT → SIGTERM → SIGKILL 递进。

> 来源：learn-claude-code/s19_mcp_plugin/README.md（分析了 services/mcp/client.ts, auth.ts, config.ts）

### 2.4 Skills vs Slash Commands

**Skills：** Markdown 格式的能力描述文件，按需加载。Skill 的 name 在 SYSTEM prompt 中列出，body 在模型请求时按需注入。支持两层注入策略以节省 token。

**Slash Commands：** 以 `/` 开头的用户可调用命令，映射到具体的工具或工作流。

**关键区别：** Skills 是给模型看的能力描述（让它知道"我能做什么"），Slash Commands 是给用户触发的入口。

> 来源：learn-claude-code/s07_skill_loading/README.md

---

## 3. A2A（Agent-to-Agent）能力

### 3.1 子 Agent 机制

Claude Code 通过 `AgentTool` 实现子 agent 生成。核心设计：

```python
def spawn_subagent(description: str) -> str:
    # 子 Agent 的工具：基础工具，但没有 task（禁止递归）
    sub_tools = [bash, read_file, write_file, edit_file, glob]
    messages = [{"role": "user", "content": description}]  # 全新 messages[]

    for _ in range(30):  # safety limit
        response = client.messages.create(
            model=MODEL, system=SUB_SYSTEM,
            messages=messages, tools=sub_tools, max_tokens=8000,
        )
        messages.append({"role": "assistant", "content": response.content})
        if response.stop_reason != "tool_use":
            break
        # ... 执行工具，结果追加到 messages ...

    return extract_text(messages[-1]["content"])  # 只返回最终文本
```

**关键设计决策：**

| 决策 | 选择 | 原因 |
|------|------|------|
| 上下文隔离 | 全新 `messages[]` | 子 Agent 的中间过程不污染主 Agent 的上下文 |
| 只回传结论 | `extract_text(last_message)` | 不回传整个 messages 列表 |
| 禁止递归 | 子 Agent 无 task 工具 | 防止子 Agent 再 spawn 新的子 Agent |
| 安全策略不跳过 | 子 Agent 工具调用也走 PreToolUse hook | 上下文隔离不代表权限隔离 |

> 来源：learn-claude-code/s06_subagent/README.md

**Claude Code 的子 Agent 执行模型（7 种 Task 类型）：**

| Task 类型 | 说明 |
|-----------|------|
| `LocalAgentTask` | 本地 agent 任务 |
| `RemoteAgentTask` | 远程 agent 任务 |
| `InProcessTeammateTask` | 进程内队友 |
| `LocalShellTask` | 本地 shell 任务 |
| `LocalWorkflowTask` | 本地工作流任务 |
| `DreamTask` | 后台 dream 任务 |
| `MonitorMcpTask` | MCP 监控任务 |

> 来源：claude-code/src/tasks/ 目录

### 3.2 多 Agent 编排：Teams

Claude Code 支持 Lead + 多个 Teammate 的协作模式：

**子 Agent vs 队友对比：**

| 维度 | 子 Agent (s06) | 队友 (s15) |
|------|---------------|------------|
| 生命周期 | 一次性，用完销毁 | 多轮（idle loop） |
| 通信 | 只回传结论 | 异步收件箱，随时通信 |
| 上下文 | 完全隔离 | 通过消息共享信息 |
| 数量 | 一个主 Agent + 偶尔子 Agent | 一个 Lead + 多个队友 |

**MessageBus（文件收件箱）：**
- 每个 Agent 有一个 `.jsonl` 邮箱
- 发消息 = 往对方的文件里 append 一行 JSON
- 读消息 = 读文件 + 删除（消费式）
- 真实 CC 使用 `proper-lockfile` 防并发写冲突
- 邮箱路径：`~/.claude/teams/{team}/inboxes/`

> 来源：learn-claude-code/s15_agent_teams/README.md

### 3.3 Coordinator 模式

Claude Code 支持 Coordinator/Worker 架构：
- Coordinator 负责任务分解和分发
- Worker 负责具体执行
- 工具过滤：Coordinator 和 Worker 有不同的可用工具集
- 权限同步：`src/utils/swarm/permissionSync.ts`

> 来源：claude-code/src/coordinator/coordinatorMode.ts, docs/agent/coordinator-and-swarm.mdx

### 3.4 Worktree 隔离

每个子 agent 可以在独立的 git worktree 中工作，避免文件冲突：
- `EnterWorktreeTool` — 创建/进入 worktree
- `ExitWorktreeTool` — 退出 worktree
- 自动清理未修改的 worktree

> 来源：learn-claude-code/s18_worktree_isolation/README.md

---

## 4. Hooks 事件系统

### 4.1 完整 Hook 事件列表（27 种）

Claude Code 实际有 **27 种 hook 事件**，远超教学版的 4 种：

**工具相关：**
- `PreToolUse` — 工具执行前
- `PostToolUse` — 工具执行后
- `PostToolUseFailure` — 工具执行失败后
- `PermissionRequest` — 权限请求时
- `PermissionDenied` — 权限被拒绝时

**会话相关：**
- `SessionStart` — 会话开始
- `SessionEnd` — 会话结束
- `Setup` — 设置阶段
- `ConfigChange` — 配置变更
- `CwdChanged` — 工作目录变更
- `InstructionsLoaded` — 指令加载完成

**用户交互：**
- `UserPromptSubmit` — 用户输入提交后
- `Elicitation` — 向用户提问时
- `ElicitationResult` — 用户回答后
- `Notification` — 通知时

**子 Agent：**
- `SubagentStart` — 子 agent 启动
- `SubagentStop` — 子 agent 停止

**压缩：**
- `PreCompact` — 压缩前
- `PostCompact` — 压缩后

**团队：**
- `TeammateIdle` — 队友空闲时
- `TaskCreated` — 任务创建
- `TaskCompleted` — 任务完成

**Worktree：**
- `WorktreeCreate` — worktree 创建
- `WorktreeRemove` — worktree 移除

**其他：**
- `Stop` — 循环即将退出时
- `StopFailure` — 停止失败时
- `FileChanged` — 文件变更

> 来源：learn-claude-code/s04_hooks/README.md（分析了 toolHooks.ts 650 行, hooks.ts, stopHooks.ts, coreTypes.ts）

### 4.2 Hook 类型（6 种执行方式）

| 类型 | 说明 |
|------|------|
| `command` | 执行 shell 命令 |
| `prompt` | 注入 prompt 到对话 |
| `agent` | 启动子 agent |
| `http` | 发送 HTTP 请求 |
| `callback` | 调用注册的回调函数 |
| `function` | 调用运行时函数 |

> 来源：claude-code/src/schemas/hooks.ts

### 4.3 HookResult 字段（14 个）

Hook 可以返回丰富的控制信号：

- `blockingError` — 阻断错误信息
- `permissionBehavior` — 覆盖权限决策
- `updatedInput` — 修改工具输入
- 还有 11 个其他字段用于精细控制

**关键不变量：** hook 的 `allow` 不能绕过 settings.json 中的 deny/ask 规则。`stopHookActive` 机制防止 Stop hook 无限循环。

> 来源：learn-claude-code/s04_hooks/README.md

### 4.4 Hook 配置示例

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": {
          "toolName": "Bash"
        },
        "hooks": [
          {
            "type": "command",
            "command": "python3 /path/to/validate.py",
            "timeout": 5000
          }
        ]
      }
    ]
  }
}
```

> 来源：claude-code/src/schemas/hooks.ts, docs/extensibility/hooks.mdx

---

## 5. 配置和扩展能力

### 5.1 Settings.json 分层配置

Claude Code 的配置来自 8 个来源，按优先级从高到低：

1. **CLI 参数** (`--allowedTools`, `--disallowedTools`)
2. **Session 级别**（运行时动态设置）
3. **Policy Settings**（企业策略）
4. **Flag Settings**（功能标志）
5. **User Settings** (`~/.claude/settings.json`)
6. **Project Settings** (`.claude/settings.json`)
7. **Local Settings** (`.claude/settings.local.json`)
8. **Command**（命令级覆盖）

> 来源：learn-claude-code/s03_permission/README.md

### 5.2 自定义 Subagent 定义

通过 `.claude/agents/` 目录定义自定义 agent：

```yaml
# .claude/agents/reviewer.md
---
name: code-reviewer
description: Expert code review specialist
model: opus
tools:
  - Read
  - Grep
  - Glob
  - Agent
---

You are a code review expert. Focus on...
```

> 来源：claude-code/.claude/agents/ 目录

### 5.3 插件系统

Claude Code 支持 MCP 插件和 marketplace：
- `src/utils/plugins/mcpPluginIntegration.ts` — MCP 插件集成
- `src/utils/plugins/schemas.ts` — 插件 schema 定义
- 支持 MCP Bundle（`.mcpb`）格式

### 5.4 IDE 集成

- VS Code 扩展
- JetBrains 扩展
- Web 应用（claude.ai/code）
- Chrome 扩展（`packages/@ant/claude-for-chrome-mcp/`）

---

## 6. 错误恢复机制

Claude Code 有 16+ 种 reason/transition code，远超教学版的 3 种：

### 6.1 三种主要恢复路径

| 模式 | 触发 | 恢复动作 |
|------|------|---------|
| 输出截断 | `max_tokens` | 升级 8K→64K，续写提示，最多 3 次 |
| 上下文超限 | `prompt_too_long` | reactive compact → 重试 |
| 临时故障 | 429/529 | 指数退避 + 抖动，连续 529 可切换备用模型 |

### 6.2 指数退避公式

```
delay = min(500 * 2^(attempt-1), 32000) + random(0~25%)
```

### 6.3 递减收益检测

连续 3 次续写（continuation）的 token 增量 < 500 时，自动停止续写——避免无意义的重试。

> 来源：learn-claude-code/s11_error_recovery/README.md（分析了 query.ts 1729 行, services/api/withRetry.ts 822 行）

---

## 7. 记忆系统

### 7.1 四类记忆

| 类型 | 回答什么 | 示例 |
|------|---------|------|
| `user` | 你是谁 | "用 tab 不用空格" |
| `feedback` | 怎么做事 | "别 mock 数据库" |
| `project` | 正在发生什么 | "auth 重写是合规驱动" |
| `reference` | 东西在哪找 | "pipeline bug 在 Linear INGEST" |

### 7.2 记忆选择机制

Claude Code 使用 **LLM（Sonnet）本身** 来做记忆选择，而不是 embedding 向量相似度。

### 7.3 提取机制

记忆提取通过 forked agent 执行，具有受限权限：`skipTranscript: true`, `maxTurns: 5`。

### 7.4 Dream 整合

Dream 整合使用四层门控：
- 时间门（24 小时）
- 扫描节流
- 会话门（5 个转录）
- 锁门

> 来源：learn-claude-code/s09_memory/README.md（分析了 memdir/memdir.ts, findRelevantMemories.ts, services/extractMemories/, services/autoDream/）

---

## 8. 作为被编排子 Agent 的能力

### 8.1 Print Mode (-p) vs Interactive Mode

| 特性 | Print Mode (-p) | Interactive Mode |
|------|----------------|------------------|
| 交互性 | 非交互，适合管道 | 交互式 REPL |
| 权限 | 可配合 `--allowedTools` | 需要用户审批 |
| 输出 | stdout 文本 | TUI 渲染 |
| 会话 | 无持久会话 | 支持会话恢复 |
| 适用场景 | 被其他 agent 编排 | 人类直接使用 |

### 8.2 结构化输出

Claude Code 支持：
- `--output-format json` — JSON 输出
- `--output-format stream-json` — 流式 JSON
- JSON Schema 约束（通过 SDK）

### 8.3 成本和性能控制

| 参数 | 作用 |
|------|------|
| `--max-turns` | 限制最大循环轮数 |
| `--max-budget` | 限制 token 预算 |
| `--model` | 指定模型 |
| `--effort` | 控制推理深度 |
| `--timeout` | 超时控制 |

### 8.4 会话恢复和 Fork

- `--resume <session-id>` — 恢复之前的会话
- `--fork <session-id>` — 从某个会话分叉新会话
- 会话数据存储在 `~/.claude/projects/` 下

---

## 9. 对 agent-diva 编排的启示

### 9.1 当前 agent-diva 的编排方式

agent-diva 通过 `delegate_task` 调用 Claude Code 作为子 agent。这是一个合理的起点，但可以优化。

### 9.2 优化建议

#### 9.2.1 利用 Print Mode 的非交互特性

```bash
claude -p "任务描述" --output-format json --max-turns 30
```

- 使用 `-p` 模式避免交互阻塞
- 使用 `--output-format json` 获取结构化输出，便于 agent-diva 解析
- 设置 `--max-turns` 防止子 agent 无限循环

#### 9.2.2 精细化权限控制

```bash
claude -p "任务描述" \
  --allowedTools "Read,Write,Edit,Bash,Glob,Grep" \
  --disallowedTools "Agent,TeamCreate,SendMessage"
```

- 通过 `--allowedTools` 精确控制子 agent 可用的工具
- 禁用 `Agent` 工具防止递归 spawn
- 禁用团队工具避免子 agent 创建自己的团队

#### 9.2.3 利用 Hooks 实现监控和编排

通过配置 hooks，agent-diva 可以：
- 在 `PreToolUse` 拦截危险操作
- 在 `PostToolUse` 收集执行结果
- 在 `Stop` 时触发后续任务
- 在 `SubagentStop` 时协调子 agent 完成

#### 9.2.4 上下文隔离策略

- 每次调用 Claude Code 子 agent 时使用独立的工作目录（通过 worktree 隔离）
- 利用 CLAUDE.md 传递任务上下文，而不是通过命令行参数
- 子 agent 的中间过程天然隔离（Claude Code 内置的子 agent 机制）

#### 9.2.5 成本控制

- 使用 `--model` 指定适合任务复杂度的模型（简单任务用 haiku，复杂任务用 opus）
- 使用 `--max-budget` 设置 token 上限
- 利用 Claude Code 内置的压缩机制减少 token 消耗

#### 9.2.6 错误恢复协作

- Claude Code 内置的错误恢复机制（重试、退避、模型切换）可以自动处理大部分临时故障
- agent-diva 层面只需处理 Claude Code 无法恢复的情况（超时、预算耗尽）

#### 9.2.7 利用 MCP 扩展能力

- 通过 MCP 为 Claude Code 子 agent 提供 agent-diva 特有的工具
- 子 agent 可以通过 MCP 调用 agent-diva 的消息总线、通道系统等
- 命名空间隔离：`mcp__agent-diva__send_message`

### 9.3 编排架构建议

```
agent-diva (Rust)
  ├── delegate_task → Claude Code (print mode)
  │     ├── --allowedTools "Read,Write,Edit,Bash,Glob,Grep"
  │     ├── --max-turns 30
  │     ├── --output-format json
  │     └── hooks → HTTP callback to agent-diva
  │           ├── PreToolUse → 安全审计
  │           ├── PostToolUse → 结果收集
  │           └── Stop → 任务完成通知
  │
  ├── MCP Server (agent-diva 提供)
  │     ├── send_message — 发送消息到通道
  │     ├── get_context — 获取项目上下文
  │     └── delegate_subtask — 委派子任务
  │
  └── 会话管理
        ├── --resume 恢复长期任务
        └── --fork 从成功会话分叉新任务
```

### 9.4 关键注意事项

1. **不要信任 Claude Code 的 `stop_reason`**：Claude Code 内部以实际出现的 tool_use block 为继续信号，不是 `stop_reason == "tool_use"`
2. **子 agent 的权限冒泡**：如果 Claude Code 子 agent 触发权限请求，会冒泡到父终端，需要在 print mode 下预配置好权限
3. **MCP 工具的命名空间**：使用 `mcp__server__tool` 格式避免工具名冲突
4. **上下文窗口管理**：长任务需要利用 Claude Code 的压缩机制，或手动分割任务
5. **Worktree 隔离**：并行子 agent 必须使用不同的 worktree，避免文件冲突

---

## 附录：关键源码文件索引

### Claude Code 源码（.workspace/claude-code/）

| 文件 | 作用 |
|------|------|
| `src/query.ts` | 核心 agent loop（queryLoop 异步生成器） |
| `src/QueryEngine.ts` | 查询编排器 |
| `src/Tool.ts` | Tool 类型定义 |
| `src/tools.ts` | 工具注册表 |
| `src/context.ts` | 上下文组装 |
| `src/schemas/hooks.ts` | Hook Zod schema |
| `src/types/hooks.ts` | Hook 类型定义 |
| `src/types/permissions.ts` | 权限类型定义 |
| `src/utils/hooks.ts` | Hook 执行引擎 |
| `src/services/tools/toolOrchestration.ts` | 工具编排 |
| `src/services/tools/toolExecution.ts` | 工具执行 |
| `src/services/tools/toolHooks.ts` | 工具 hook 集成 |
| `src/services/compact/compact.ts` | 上下文压缩 |
| `src/services/compact/autoCompact.ts` | 自动压缩 |
| `src/coordinator/coordinatorMode.ts` | Coordinator 模式 |
| `packages/builtin-tools/src/tools/AgentTool/` | 子 agent 工具 |
| `packages/mcp-client/` | MCP 客户端 |

### learn-claude-code 教学分析（.workspace/learn-claude-code/）

| 章节 | 主题 | 关键分析 |
|------|------|---------|
| s01_agent_loop | Agent Loop | while True + stop_reason |
| s02_tool_use | 工具使用 | TOOL_HANDLERS 分发 |
| s03_permission | 权限系统 | 4 种行为、8 个配置源、YoloClassifier |
| s04_hooks | Hook 系统 | 27 种事件、6 种类型、14 字段 HookResult |
| s06_subagent | 子 Agent | 全新 messages[]、上下文隔离 |
| s07_skill_loading | 技能加载 | 两层注入、按需加载 |
| s08_context_compact | 上下文压缩 | 四层管线、便宜先跑 |
| s09_memory | 记忆系统 | LLM 选择、Dream 整合 |
| s10_system_prompt | System Prompt | 三层缓存、20-30KB |
| s11_error_recovery | 错误恢复 | 16+ code、指数退避 |
| s15_agent_teams | Agent 团队 | MessageBus、文件收件箱 |
| s18_worktree_isolation | Worktree 隔离 | 任务目录绑定 |
| s19_mcp_plugin | MCP 插件 | 6 种传输、OAuth 2.0 |
| s20_comprehensive | 综合 | 全部 19 机制归位 |

---

*本文档基于 .workspace/claude-code v2.6.6 源码和 .workspace/learn-claude-code 20 章教学项目编写。*
