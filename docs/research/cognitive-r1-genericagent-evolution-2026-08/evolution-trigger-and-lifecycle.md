# Evolution Trigger and Lifecycle（GenericAgent）

- 基线：`ee5a474` @ `.workspace/genericagent`
- 证据：`源码事实` / `推断` / `建议`

## 1. 两层结构

| 层 | 职责 | 入口 |
| --- | --- | --- |
| 外层任务源 | 何时产生一条 query | `GenericAgent.put_task` / `run`（`agentmain.py`） |
| 内层 Agent Loop | 单 query 多 turn 工具循环 | `agent_runner_loop`（`agent_loop.py`）+ `GenericAgentHandler`（`ga.py`） |

**源码事实：** 无独立“任务完成状态机”；结束条件是 `next_prompt` 为空 / `should_exit` / `max_turns`。记忆写盘是模型调工具的副作用。

## 2. 触发地图

### 2.1 用户命令

- CLI `input` / 前端 → `put_task(source='user')`
- 每条任务新建 handler；`working={}`，可继承上一 handler 的 `key_info` 并附加 “N 个对话前” 提示
- `sys_prompt = sys_prompt.txt + Date + get_global_memory()`
- `max_turns` 默认 180

### 2.2 任务完成

| 条件 | 机制 |
| --- | --- |
| 最终自然语言无工具 | `do_no_tool` → `next_prompt=None` → `CURRENT_TASK_DONE` |
| `ask_user` | `should_exit=True` |
| `max_turns` | `MAX_TURNS_EXCEEDED` |
| 空响应×3 | 退出 |

**源码事实：** **没有**任务结束自动调用 `start_long_term_update`。  
**Prompt-only：** schema 写 “15+ turns 必须调用”；模型可忽略。

### 2.3 Turn 节奏（硬编码注入）

`turn_end_callback` 使用 **elif 互斥**（同一 turn 只命中一条主提示）：

| 条件 | 注入 | 硬/软 |
| --- | --- | --- |
| `turn % 175 == 0`（非 plan） | 催 `ask_user` | 硬注入 / 软执行 |
| `turn % 7 == 0` | 催 `update_working_checkpoint` | 硬注入 / 软执行 |
| `turn % 25 == 0` | 催写文件 checkpoint | 硬注入 / 软执行 |
| `turn % 10 == 0` | `get_global_memory()` 重注 L1 | 硬 |
| 奇数 turn | `<history>` 最近 30 摘要 | 硬 |
| `turn % 4 == 1` 且历史>30 | `<earlier_context>` 折叠 | 硬 |
| plan 每 5 turn（≥10） | 催读 plan 文件 | 硬注入 |

**注意：** turn=70 只走 `%7`，**不会**同时 `%10` 重注 L1。

### 2.4 Reflect / Scheduler / Autonomous / Goal

| 源 | 文件 | INTERVAL | 行为 |
| --- | --- | --- | --- |
| Reflect 框架 | `agentmain --reflect` | 脚本定 | `check()`→`put_task(source='reflect')` |
| Scheduler | `reflect/scheduler.py` | 120s | sche_tasks + **L4 12h** |
| Autonomous | `reflect/autonomous.py` | 1800s | 固定文案“离开30分钟”——**无真实离开检测** |
| Goal mode | `reflect/goal_mode.py` | 5s | 预算循环；prompt 要求不写全局记忆 |
| Checklist / team | `checklist_master` / `agent_team_worker` | 60s | BBS 驱动 |

### 2.5 Project Mode

- `plugins/project_mode.py` 在 `agent_before` 注入项目规则 + `project_memory.md` 指针
- 激活：`handler.enter_project_mode(name)`（实例属性，本地 HEAD）
- **并行命名**的项目 L1/L2，不是全局 L1/L2

## 3. 记忆工具精确行为

### 3.1 `update_working_checkpoint`

```text
working['key_info'] / working['related_sop'] 全量替换
passed_sessions = 0
不写盘；经 _get_anchor_prompt 再注入
```

### 3.2 `start_long_term_update`

```text
1. 构造蒸馏 next_prompt（Action-Verified 纪律 + L2/L3 路由）
2. next_prompt += get_global_memory()
3. tool_result = "This is L0:\n" + memory_management_sop.md
4. 不写任何记忆文件
```

- 参数：schema `properties: {}`
- 实际写入：后续 `file_patch` / `file_write`
- `--no-user-tools`：从 schema 移除本工具与 `ask_user`（UltraPlan 子代理默认）

### 3.3 L0 注入路径

```text
do_start_long_term_update
 ├─ tool_result: L0 全文
 └─ next_prompt: 蒸馏规则 + L1 结构/索引
```

L0 路径使用裸 `./memory/memory_management_sop.md`（依赖**进程 CWD**=repo root），不用 `handler.cwd`。

## 4. 硬编码 vs Prompt-only

### 硬编码

- 任务队列、reflect 轮询、BANNED_TOOLS、turn 循环  
- working 存储与 anchor 注入、cadence SYSTEM、L0 文件读入  
- project hook、scheduler 时间窗、L4 cron、plan 完成拦截（关键词）

### Prompt-only（可忽略）

- 15+ turns 必须长期结算  
- autonomous 内 skip 长期更新  
- 只记行动验证成功  
- L1≤30 行、最小 patch  
- Goal 不更新全局记忆  
- autonomous “离开 30 分钟”

## 5. 生命周期时序

```text
[Source] user | func | task | reflect
  → put_task → run()
  → inherit key_info
  → agent_runner_loop turns
       optional: update_working_checkpoint
       tools + anchor
       cadence tips
       optional: start_long_term_update → file_patch L2/L3
  → exit
  → history_info 保留；working 丢弃（key_info 可拷贝）
```

## 6. 对 Diva 的含义（建议）

1. 拆 **外触发** 与 **内节奏** 两层，勿混成单一状态机。  
2. Subagent 可复用 “禁 ask_user + 禁长期结算”。  
3. 若要可靠结晶，不能只靠 schema “15+ turns”；需任务完成钩子或硬提示。  
4. `start_long_term_update` 模式是 **打开结算续轮**，不是 sync write API。  
5. Diva 若要 Action-Verified，必须在 apply 门增加代码校验（GA 没有）。
