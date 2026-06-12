# GenericAgent 深度架构分析

> 调研日期：2026-06-01
> 项目位置：`.workspace/GenericAgent/`
> 定位：~3K 行 Python 种子代码、9 个原子工具、~100 行 Agent Loop 的"自进化自治 Agent 框架"

---

## 目录

1. [Agent Loop 机制](#1-agent-loop-机制)
2. [工具链系统](#2-工具链系统)
3. [A2A（Agent-to-Agent）能力](#3-a2aagent-to-agent能力)
4. [记忆与学习](#4-记忆与学习)
5. [与 agent-diva 的对比亮点](#5-与-agent-diva-的对比亮点)
6. [对 agent-diva 的启示](#6-对-agent-diva-的启示)

---

## 1. Agent Loop 机制

### 1.1 主循环驱动方式

**Python Generator 驱动 + 队列消息总线**

核心循环 `agent_runner_loop()` 定义在 `agent_loop.py:42-107`，是一个 **Python 生成器函数**，通过 `yield` 流式输出显示内容。

输入侧采用 `queue.Queue` 消息总线：`GenericAgent.put_task()` (`agentmain.py:107`) 将任务入队，`GenericAgent.run()` (`agentmain.py:128`) 作为消费者线程从队列拉取任务。多前端（CLI、Telegram、WeChat、QQ、DingTalk、Conductor WebSocket、任务文件）全部汇聚到这个队列。

```
用户输入 → put_task(queue) → GenericAgent.run() → agent_runner_loop(generator)
                                                              ↓
前端 ← display_queue ← yield chunks ← LLM.chat() + tool dispatch
```

### 1.2 单次 Turn 处理流程

`agent_loop.py:50-104`，每轮处理如下：

```
1. turn++, 重置工具描述(每10轮), 触发 turn_before/llm_before 钩子
2. client.chat(messages, tools_schema) → 流式/非流式获取 LLM 响应
3. 解析 tool_calls（若无则注入 no_tool 伪调用）
4. 对每个 tool_call:
   a. handler.dispatch(tool_name, args) → 查找 do_{tool_name} 方法
   b. 收集 StepOutcome(data, next_prompt, should_exit)
   c. should_exit → 立即退出
   d. next_prompt 为空 → CURRENT_TASK_DONE
5. 调用 handler.turn_end_callback() 注入工作记忆到下一轮 prompt
6. 将 next_prompt + tool_results 追加到 messages 列表
```

**关键数据结构**：`StepOutcome` (`agent_loop.py:6-10`) — 工具执行的统一返回类型：
- `data`: 工具输出数据
- `next_prompt`: 下一轮注入的 prompt（`None` 表示任务完成）
- `should_exit`: 是否立即终止循环

### 1.3 迭代控制

多层级限制，防止单轮失控：

| 层级 | 位置 | 默认值 | 说明 |
|------|------|--------|------|
| `agent_runner_loop` 参数 | `agent_loop.py:43` | `max_turns=40` | 基础上限 |
| `GenericAgent.run()` 覆盖 | `agentmain.py:149` | `max_turns=80` | 实际运行值 |
| Plan Mode 覆盖 | `ga.py:431` | `max_turns=100` | 规划模式 |
| Plan Mode 硬停 | `ga.py:573` | turn 120 | 强制报告用户 |
| 每 7 轮警告 | `ga.py:565` | — | 危险警告，要求换策略 |
| 每 75 轮检查点 | `ga.py:570` | — | 必须总结并 ask_user |
| Goal Mode | `goal_mode.py:74` | `max_turns=50` + `budget_seconds=1800` | 时间+轮次双限 |
| 空响应重试 | `ga.py:450` | 3 次 | 连续空响应后退出 |

### 1.4 上下文管理

**System Prompt 组装**（`agentmain.py:39` 的 `get_system_prompt()`）：

```
assets/sys_prompt.txt（角色定义）
+ 当前日期
+ get_global_memory() → insight_fixed_structure.txt (L0-L3 索引 + 宪法规则)
                       → global_mem_insight.txt (操作规则、SOP 索引)
+ extra_sys_prompt（LLM 后端附加）
+ peer_hint（跨会话状态提示）
```

**每轮工作记忆注入**（`ga.py:536` 的 `_get_anchor_prompt()`）：

```
<earlier_context> → _fold_earlier() 压缩旧轮次为 "[Agent] (3 turns)" 摘要
<history> → 最近 30 轮的一行摘要
Current turn: N
<key_info> → update_working_checkpoint 工具设置的工作记忆
```

**上下文窗口裁剪**（`llmcore.py:95` 的 `trim_messages_history()`）：
- 字符上限 = `context_win × 3`，目标 60%
- 压缩旧消息的 `<thinking>/<tool_use>/<tool_result>` 标签
- 弹出最旧消息，保持 user 消息交替

### 1.5 错误处理和恢复

| 机制 | 位置 | 说明 |
|------|------|------|
| LLM 重试+退避 | `llmcore.py:360` | 指数退避 1.5x，上限 30s，尊重 `retry-after` |
| MixinSession 故障转移 | `llmcore.py:905` | 多后端自动切换，spring_back 300s 回主 |
| 空响应重试 | `ga.py:450` | 3 次连续空响应后退出 |
| 未知工具处理 | `agent_loop.py:26-29` | 返回错误提示，重置工具描述 |
| bad_json 恢复 | `agent_loop.py:26` | 解析失败注入伪调用，引导 LLM 重试 |
| 用户中止 | `agentmain.py:101` | `stop_sig` + `code_stop_signal` 杀子进程 |
| 文件停止信号 | `agentmain.py:154` | `_stop` 文件触发外部控制 |
| 文件干预 | `ga.py:575-578` | `_keyinfo` / `_intervene` 文件注入上下文 |
| Plan Mode 验证门 | `ga.py:469-501` | 禁止未验证就声明完成 |

---

## 2. 工具链系统

### 2.1 工具注册/发现机制

**静态 Schema + 反射式命名约定**

工具定义在 `assets/tools_schema.json`（`agent_loop.py:42` 传入），遵循 OpenAI Function Calling 格式：

```json
{"type": "function", "function": {"name": "code_run", "description": "...", "parameters": {...}}}
```

工具发现通过 `BaseHandler.dispatch()` (`agent_loop.py:18-29`) 的 **反射机制**：

```python
method_name = f"do_{tool_name}"
if hasattr(self, method_name):
    ret = yield from try_call_generator(getattr(self, method_name), args, response)
```

`GenericAgentHandler`（`ga.py:265`）通过实现 `do_code_run`、`do_file_read` 等方法自动注册为工具处理器。

**Token 优化**：`ToolClient`（`llmcore.py:794`）首次调用后缓存工具 schema，后续轮次仅发送 "Tools: still active" 短提示。

### 2.2 工具调用格式（双协议）

| 协议 | 适用场景 | 位置 |
|------|----------|------|
| **Native Function Calling** | `NativeClaudeSession` / `NativeOAISession` | `llmcore.py:978-1024` |
| **自定义 XML/文本协议** | `LLMSession` / `ClaudeSession` | `llmcore.py:743-852` |

**Native 协议**：schema 直接传入 API `payload["tools"]`，Claude 格式通过 `openai_tools_to_claude()` (`llmcore.py:717`) 转换。

**文本协议**：将完整工具 schema 嵌入 system prompt，要求模型输出 `<tool_use>{"name":"...","arguments":{...}}</tool_use>` 标签，由 `_parse_text_tool_calls()` (`llmcore.py:854`) 解析。

两种协议统一产出 `MockResponse` 对象（`llmcore.py:735`），使 `agent_loop.py` 的调度循环与协议无关。

### 2.3 工具执行隔离

**进程级隔离，无沙箱**

`code_run`（`ga.py:12-94`）：
- Python：写入临时 `.ai.py` 文件，`subprocess.Popen` 执行，注入 `code_run_header.py`
- PowerShell/Bash：`pwsh -NoProfile -NonInteractive -Command` / `bash -c`
- 超时监控线程 + `stop_signal` 列表控制
- Windows 使用 `CREATE_NO_WINDOW` 隐藏窗口

**无容器/VM 沙箱**，代码以当前用户权限运行。文件访问通过 `os.path.abspath(os.path.join(self.cwd, path))` 解析（`ga.py:276`），可到达用户权限范围内的任意路径。

浏览器执行通过 TMWebDriver 的 WebSocket/HTTP 通道到 Chrome 扩展，由浏览器安全模型隔离。

### 2.4 权限控制和安全模型

**6 层轻量级防御**：

| 层 | 机制 | 说明 |
|----|------|------|
| 1 | 工具 Schema 白名单 | 只有 `tools_schema.json` 中的工具暴露给 LLM |
| 2 | LLM 协议约束 | `<thinking>`→`<summary>`→`<tool_use>` 强制结构 |
| 3 | 工具级防护 | `file_patch` 要求唯一匹配；`code_run` 白名单语言 |
| 4 | SOP 定义边界 | Markdown SOP 文件定义操作红线 |
| 5 | 密钥管理 | `memory/keychain.py` XOR 加密 + `SecretStr` 掩码 |
| 6 | Hook 可观测性 | `plugins/hooks.py` 的 `tool_before/after` 事件 + Langfuse 追踪 |

### 2.5 动态工具加载

**有限支持**：
- Agent 可通过 `code_run` 执行 Python 脚本创建新的 `do_*` 方法，但需要重启才能被 `dispatch()` 发现
- 工具 schema 在启动时加载一次（`agentmain.py:17-21`），运行时无热更新机制
- `plugins/hooks.py:46` 的 `discover_and_load()` 支持插件目录动态导入，但用于事件钩子而非工具注册
- `--reflect` 模式支持脚本热重载（`agentmain.py:230-265`），但是自治任务触发而非工具注册

---

## 3. A2A（Agent-to-Agent）能力

GenericAgent 实现了 **4 种不同层级的 A2A 机制**，从简单的文件通信到分布式 BBS 协调。

### 3.1 子 Agent 机制

子 Agent 是完整的 `GenericAgent` 进程实例，通过命令行启动：

```bash
python agentmain.py --task {name} [--input "short text"] [--llm_no N]
```

**启动实现**：`agentmain.py:193-229`
- 创建 `temp/{task_name}/` 任务目录
- 写入 `input.txt` 作为任务 prompt
- 通过 `subprocess.Popen` 以后台进程启动（`--nobg` 标志）
- 父进程获取 PID 后返回

**通信协议**（文件系统）：

| 文件 | 方向 | 用途 |
|------|------|------|
| `input.txt` | 父→子 | 任务 prompt |
| `output{n}.txt` | 子→父 | 每轮输出 |
| `[ROUND END]` 标记 | 子→父 | 轮次结束信号 |
| `reply.txt` | 父→子 | 后续指令 |
| `_stop` | 父→子 | 终止信号 |
| `_keyinfo` | 父→子 | 注入工作记忆 |
| `_intervene` | 父→子 | 纠正指令 |

`consume_file()`（`ga.py:259-263`）实现原子读取+删除。

**上下文继承（fork）**：子 Agent 可通过 `code_run(inline_eval=True)` 写入 `_history.json` 继承父 Agent 的对话历史。

### 3.2 三种通信协议

**协议 A — 文件系统**（sub-agent 模式）：
- 基于 `temp/{task_name}/` 目录的文件读写
- `consume_file()` 提供原子一次性消息传递

**协议 B — HTTP/WebSocket**（conductor 模式，`frontends/conductor.py`）：
- FastAPI 服务器，REST + WebSocket API
- `POST /subagent` 创建子 Agent
- `POST /subagent/{sid}` 操作（keyinfo/input/reply/stop/kill）
- `WebSocket /ws` 实时推送状态

**协议 C — BBS 公告板**（hive 模式，`assets/agent_bbs.py`）：
- FastAPI + SQLite 异步消息板
- `POST /register` 注册 Agent
- `POST /post` 发布任务
- `GET /poll?since_id=N` 轮询新消息
- API Key 访问控制，支持多 board 隔离

### 3.3 任务委托/分发模式

| 模式 | SOP 文件 | 说明 |
|------|----------|------|
| **Plan Mode + 委托标签** | `memory/plan_sop.md` | `[D]`委托/`[P]`并行/`[?]`条件/`[FIX]`修复 |
| **Supervisor 监控** | `memory/supervisor_sop.md` | 监督者轮询 `output.txt`，通过 `_intervene` 干预 |
| **Map 并行** | `memory/subagent.md` 场景2 | N 个独立子任务，每文件一个子 Agent |
| **对抗性验证** | `memory/verify_sop.md` | 强制独立验证子 Agent，目标是证伪 |
| **Conductor 调度** | `frontends/conductor.py` | 调度者永不执行，全部委派子 Agent |

### 3.4 对抗性验证子 Agent

`verify_sop.md` 定义了一个精妙的验证协议：

- 验证者的目标是 **证明交付物不工作**（而非确认工作）
- 接收 `verify_context.json`（任务描述、计划路径、交付物、检查项）
- **故意不接收执行历史**（避免上下文污染）
- 输出 `VERDICT: PASS / FAIL / PARTIAL`
- FAIL 触发修复循环，最多 2 次重试后升级给用户
- 铁律："必须运行"、"必须有工具证据"、"独立验证"

### 3.5 并行 Agent 编排

**Goal Hive 模式**（`memory/goal_hive_sop.md`）：

架构：1 Hive Master + N Workers（≤10，通常 2-4）

```bash
# 启动 BBS
start /b python assets/agent_bbs.py --cwd temp/hive_xxx --port PORT --key BOARD_KEY

# 启动 Worker
start /b python agentmain.py --reflect reflect/agent_team_worker.py \
    --base_url http://127.0.0.1:PORT --board_key BOARD_KEY --name hive-worker-1
```

- Worker 每 60 秒轮询 BBS（`reflect/agent_team_worker.py:6`，`INTERVAL = 60`）
- Master 负责拆分子任务、发布到 BBS、验收结果、寻找改进点
- Master 不允许亲自执行（"不允许亲自干活导致 worker 空转"）

**Conductor 并行**（`frontends/conductor.py`）：
- 每个子 Agent 在独立守护线程中运行（line 136）
- 专用监控线程轮询 display_queue（lines 140-173）
- 自动清理线程终止空闲 1 小时的子 Agent（lines 272-289）
- Conductor 自身通过事件循环串行化决策（lines 300-338）

### 3.6 Agent 发现和注册

| 机制 | 说明 |
|------|------|
| BBS 注册 | Worker 通过 `POST /register` 自注册，`GET /authors` 发现 |
| Conductor 注册 | `subagents: Dict[str, SubAgentState]` 全局字典 |
| 文件系统发现 | 父 Agent 通过 `temp/{task_name}/` 目录约定跟踪 |
| 调度器发现 | 扫描 `sche_tasks/*.json` 发现已启用任务 |

---

## 4. 记忆与学习

### 4.1 短期记忆机制

**三层短期记忆**：

**A. 对话历史（进程内存）**
- `GenericAgent.history`（`agentmain.py:50`）— `[USER]: ...` 字符串列表
- 跨任务累积，进程退出后丢失

**B. 工作记忆（工具驱动的临时便签）**
- `update_working_checkpoint` 工具（`ga.py:438-448`）
- `key_info`（≤200 tokens）+ `related_sop` 指针
- 每轮通过 `_get_anchor_prompt()`（`ga.py:536`）注入 LLM

**C. LLM 后端历史（权威消息列表）**
- `BaseSession.history`（`llmcore.py:527`）
- Claude API 格式（role/content blocks）
- `compress_history_tags()`（`llmcore.py:38`）每 5 轮压缩旧消息
- `trim_messages_history()`（`llmcore.py:95`）按字符上限裁剪

**D. 会话持久化**
- `--task` 模式下可序列化/反序列化 `_history.json`（`agentmain.py:216`）
- `/resume` 命令扫描 `model_responses_*.txt` 恢复会话

### 4.2 长期记忆/持久化 — 四层架构

定义在 `memory/memory_management_sop.md`，这是 GenericAgent 最精妙的设计之一：

```
L1: global_mem_insight.txt（极简索引层，≤30 行，<1K tokens）
    ↓ 场景关键词→记忆定位 映射
L2: global_mem.txt（事实库层，环境性事实：路径、凭证、配置）
    ↓ 详细引用
L3: memory/*.md + memory/*.py（记录库层，SOP + 工具脚本）
    ↓ 历史归档
L4: memory/L4_raw_sessions/（历史会话层，压缩存档）
```

**核心公理**（`memory_management_sop.md:1-14`）：

1. **行动验证原则**："No Execution, No Memory" — 只有工具验证过的结果才能写入
2. **神圣不可删改性**：已验证数据只能压缩/迁移，不能丢失
3. **禁止存储易变状态**：禁止时间戳、PID、Session ID
4. **最小充分指针**：上层只留下层的最短定位标识

**"存在性编码"原则**：L1 只需让 LLM *知道*某能力存在，LLM 可用工具调用获取完整内容。

**记忆写入工具**：
- `update_working_checkpoint`（`ga.py:438`）— 会话内工作记忆
- `start_long_term_update`（`ga.py:505-520`）— 触发长期记忆蒸馏，读取 `memory_management_sop.md` 后提取"行动验证、长期有效"的信息更新 L2/L3

**记忆访问追踪**：`log_memory_access()`（`ga.py:157`）统计文件访问频次到 `memory/file_access_stats.json`。

**记忆清理 SOP**（`memory/memory_cleanup_sop.md`）：基于 ROI 公式 `ROI = (error_prob_without_hint × cost) / per_turn_token_cost` 决定保留/压缩/删除。

### 4.3 技能系统（程序性记忆）

**SOP 文件即技能**：`memory/` 目录下约 20 个 SOP 文件：

| SOP | 用途 |
|-----|------|
| `plan_sop.md` | 多步任务执行协议 |
| `goal_mode_sop.md` | 长时间自治目标追求 |
| `goal_hive_sop.md` | 多 Worker 并行协作 |
| `morphling_sop.md` | 能力吸收/复制方法论 |
| `autonomous_operation_sop.md` | 自治任务规划与执行 |
| `subagent.md` | 子 Agent 文件 I/O 协议 |
| `verify_sop.md` | 对抗性验证协议 |
| `supervisor_sop.md` | 监督者模式 |

**工具脚本即程序性知识**：`memory/` 下的 Python 模块（`keychain.py`、`ocr_utils.py`、`adb_ui.py`、`procmem_scanner.py`）可直接导入使用。

**远程技能搜索引擎**（`memory/skill_search/engine.py`）：
- 通过 `http://www.fudankw.cn:58787` API 语义搜索 105K+ 技能卡片
- `SkillIndex` 数据类包含 37 个字段（clarity、completeness、actionability、autonomous_safe、blast_radius 等）
- 自动检测环境（OS、shell、运行时、模型能力）

### 4.4 自我改进机制

虽然无显式 RL 训练循环，但有多个自我改进闭环：

**A. 自治自改进**（`autonomous_operation_sop.md`）：
1. 批判性审查历史 → 识别失败模式
2. 使用价值公式规划 TODO："AI 训练数据无法覆盖 × 持久未来协作收益"
3. 子 Agent 独立审查评分（防止自评偏差）
4. 执行 → 写报告 → 更新记忆

**B. Goal Mode 持续改进**（`goal_mode.py`）：
- 时间预算制（最少 3 小时），`CONTINUATION_PROMPT`（line 26-43）禁止提前交付
- 要求从不同角度审视：测试、边界 case、性能、安全、文档
- "假装你是第一次看到这个成果的使用者/审阅者/攻击者"

**C. Morphling 能力吸收**（`memory/morphling_sop.md`）：
- 提取目标的测试/基准 → 逐组件 call/rewrite/discard → 对比基准 → 固化为 SOP

**D. 子 Agent 测试循环**（`memory/subagent.md`）：
- 不给提示的子 Agent 导航测试 → 验证 L1 索引质量 → 失败则重组 L1

### 4.5 Trajectory 记录

**A. LLM 响应日志**
- `_write_llm_log()`（`llmcore.py:886`）→ `temp/model_responses/model_responses_<pid>.txt`
- 格式：`=== Prompt ===` / `=== Response ===` 时间戳标记

**B. L4 会话压缩管道**（`memory/L4_raw_sessions/compress_session.py`）：
1. 原始日志 → 压缩（去除冗余 system prompt 和 echo）
2. 提取 `<history>` 块 → 合并去重 → `all_histories.txt`
3. 按月归档为 `YYYY-MM.zip`
4. 由 `scheduler.py` 每 12 小时触发

**C. 显著性挖掘**（`memory/L4_raw_sessions/salient_mining_sop.md`）：
- 从 `all_histories.txt` 增量提取：情绪事件（愤怒、讽刺、惊讶、沮丧）、持续/中断活动
- 产出结构化数据库：活动知识层 + 情绪事件列表 + 增量处理标记

**D. 自治报告历史**（`autonomous_operation_sop/helper.py`）：
- `temp/autonomous_reports/history.txt` 格式：`R1 | 2026-06-01 | engineering | task_name | conclusion`

---

## 5. 与 agent-diva 的对比亮点

### 5.1 GenericAgent 独特设计

| 特性 | GenericAgent 实现 | agent-diva 现状 |
|------|-------------------|-----------------|
| **四层记忆架构** | L1(索引)→L2(事实)→L3(SOP)→L4(归档)，严格公理约束 | JSONL session + MEMORY.md + HISTORY.md |
| **存在性编码** | L1 只存"能力存在"的指针（≤30行），全量在 L3 | 无类似分层 |
| **对抗性验证子 Agent** | 强制独立验证者，目标是证伪，不给执行历史 | 无专门验证协议 |
| **Goal Hive 多 Worker 协作** | BBS 公告板 + Master/Worker 分离 + 时间预算 | SubagentManager 轻量 spawn |
| **Conductor 模式** | 调度者永不执行，全部委派，实时 WebSocket 监控 | delegate_task 通过 CLI 调用 |
| **自进化技能系统** | 每次解题结晶为 SOP，105K 远程技能搜索 | 技能加载为 Markdown 文件 |
| **文件干预机制** | `_keyinfo` / `_intervene` / `_stop` 文件实时注入 | 无类似机制 |
| **双协议工具调用** | Native Function Calling + 自定义 XML 文本协议 | MCP + 内置工具 |
| **Goal Mode 预算制** | 时间+轮次双限，禁止提前交付，持续打磨 | 无类似自治模式 |
| **显著性挖掘** | 从历史会话提取情绪事件和活动模式 | 无类似分析 |

### 5.2 agent-diva 的优势

| 特性 | agent-diva 优势 | GenericAgent 现状 |
|------|-----------------|-------------------|
| **消息总线架构** | 双队列 inbound/outbound，异步解耦 | 单线程 queue.Queue |
| **Channel 抽象** | 统一 ChannelHandler trait，8+ 平台即插即用 | 每平台独立 `frontends/*.py` |
| **Provider 抽象** | 统一 Provider trait + LiteLLM 兼容 | 手动 session 类型选择 |
| **MCP 工具协议** | 标准化 MCP 协议，支持动态工具 | 自定义 XML/文本协议 |
| **类型安全** | Rust 类型系统 + thiserror 错误处理 | Python 运行时类型 |
| **Session 持久化** | JSONL 持久化 + 内置 session manager | `_history.json` 手动序列化 |
| **子 Agent 管理** | SubagentManager 统一生命周期管理 | 手动进程管理 |
| **CLI 统一入口** | `agent-diva` CLI，onboard/gateway/agent/tui | `ga` CLI 分散命令 |
| **Windows 服务** | agent-diva-service 原生服务包装 | 无 |

---

## 6. 对 agent-diva 的启示

### 6.1 高优先级借鉴

#### A. 分层记忆架构

GenericAgent 的 L1→L4 记忆分层是其最有价值的设计。agent-diva 可以借鉴：

- **L1 存在性编码**：MEMORY.md 应该压缩为极简索引（≤30 行），只存"能力存在"的指针，而非全量内容
- **L3 SOP 文件**：将常用操作模式固化为 SOP Markdown，作为程序性记忆
- **L4 会话归档**：自动压缩旧 JSONL session，提取摘要到 `all_histories.txt`
- **记忆写入公理**："No Execution, No Memory" 应写入 CLAUDE.md 作为约束

**实施建议**：在 `agent-diva-agent` 的 context builder 中实现 L1 索引注入，每 N 轮重新注入一次。

#### B. 对抗性验证协议

GenericAgent 的 `verify_sop.md` 值得直接借鉴：

- 完成任务后强制 spawn 独立验证子 Agent
- 验证者目标是证伪，不接收执行历史
- 输出 `VERDICT: PASS/FAIL/PARTIAL`
- FAIL 触发修复循环

**实施建议**：在 `agent-diva-agent` 的 task 完成回调中添加验证步骤，通过 `delegate_task` CLI 调用验证 Agent。

#### C. 文件干预机制

`_keyinfo` / `_intervene` / `_stop` 文件机制简单但有效：

- 运行中的 Agent 每轮检查任务目录中的特殊文件
- 外部进程可实时注入上下文或纠正方向
- 无需 WebSocket 连接，文件系统即可

**实施建议**：在 `agent-diva-agent` 的 agent loop 的 `turn_end_callback` 中检查特殊文件。

### 6.2 中优先级借鉴

#### D. Goal Mode 预算制自治

- 时间预算 + 轮次上限双控
- 禁止提前交付，持续从不同角度打磨
- "假装你是第一次看到这个成果的使用者/审阅者/攻击者"

**实施建议**：作为 `agent-diva-agent` 的一种运行模式，通过 `agent-diva-cli agent --goal` 启动。

#### E. 工具 Token 优化

GenericAgent 的 "last_tools" 缓存（`llmcore.py:794`）：首次发送完整 schema，后续仅 "Tools: still active"。对 agent-diva 的 MCP 工具列表有参考价值——工具描述在长对话中反复发送是显著的 token 浪费。

**实施建议**：在 `agent-diva-providers` 的 context builder 中实现工具描述缓存。

#### F. SOP 驱动的程序性记忆

将常用操作模式（代码审查、测试编写、部署流程等）固化为 SOP 文件，Agent 执行前先 `file_read` 对应 SOP。这比纯靠 system prompt 更高效。

**实施建议**：在 `agent-diva-agent` 的 skill loader 中支持 SOP 文件发现和自动加载。

### 6.3 低优先级但值得了解

#### G. BBS 协作模式

GenericAgent 的 `agent_bbs.py` 是一个轻量级的 Agent 协作公告板，支持多 Worker 异步任务领取。对于需要大规模并行探索的场景（如代码库审计、批量文档处理）有参考价值。

#### H. 显著性挖掘

从历史会话中提取情绪事件和活动模式（`salient_mining_sop.md`），可用于 Agent 行为分析和改进。但实现复杂度较高，优先级低。

#### I. Morphling 能力吸收

系统性地吸收外部项目能力的方法论（提取测试→逐组件处理→对比基准→固化 SOP），适用于 agent-diva 集成新工具/Provider 的场景。

---

## 附录：关键文件索引

| 文件 | 职责 | 关键行号 |
|------|------|----------|
| `agent_loop.py` | 核心 Agent 循环、BaseHandler、StepOutcome | L42-107 (主循环), L16-29 (dispatch) |
| `agentmain.py` | GenericAgent 类、任务队列、System Prompt 组装 | L39 (get_system_prompt), L107 (put_task), L128 (run), L193-229 (--task 模式) |
| `ga.py` | GenericAgentHandler、全部 do_* 工具、工作记忆 | L12-94 (code_run), L265 (handler 类), L438-448 (working_checkpoint), L505-520 (long_term_update), L536 (anchor_prompt), L565-573 (turn warnings) |
| `llmcore.py` | LLM Session 类、SSE 解析、重试、MixinSession | L38-69 (compress), L95-108 (trim), L360 (retry), L717 (schema 转换), L743-852 (ToolClient), L905 (MixinSession), L978-1024 (NativeToolClient) |
| `memory/memory_management_sop.md` | L0 META-SOP：四层架构定义、公理 | 全文 |
| `memory/verify_sop.md` | 对抗性验证协议 | 全文 |
| `memory/goal_hive_sop.md` | 多 Worker Goal Hive 协调 | 全文 |
| `memory/subagent.md` | 子 Agent 协议、文件 I/O、Map 模式 | 全文 |
| `frontends/conductor.py` | Conductor HTTP 服务器 | L54 (subagents dict), L136 (线程启动), L249-270 (system prompt), L300-338 (事件循环) |
| `assets/agent_bbs.py` | BBS 公告板服务器 | 全文 |
| `reflect/agent_team_worker.py` | BBS Worker 反射模块 | L6 (INTERVAL), L20-33 (check), L35-49 (prompt) |
| `reflect/goal_mode.py` | Goal Mode 预算制自治 | L26-43 (CONTINUATION_PROMPT), L62-94 (check) |
| `assets/tools_schema.json` | 9 个工具定义 | 全文 |
| `plugins/hooks.py` | 事件钩子系统 | L46 (discover_and_load) |
