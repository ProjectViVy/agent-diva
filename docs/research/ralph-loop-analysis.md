# ralph-loop 核心循环实现提炼（供 agent-diva 下一阶段参考）

## 一、项目位置与关键文件

```
C:\Users\Administrator\Desktop\morediva\.workspace\loops\originals\ralph-loop
├── ralph.sh                       # 主入口：循环、执行、监控、状态检查
├── .agent/
│   ├── PROMPT.md                  # 每次迭代发送给 LLM agent 的指令
│   ├── STEERING.md                # 人工优先干预文件，agent 每轮先检查
│   ├── tasks.json                 # 任务列表（当前仓库为空 `[]`）
│   ├── prd/PRD.md                 # 产品需求文档
│   ├── logs/LOG.md                # 进度日志（由 agent 写入）
│   ├── history/                   # 每轮原始输出（已去 ANSI）
│   └── screenshots/               # 任务截图
└── scripts/lib/
    ├── agents.sh                  # 支持的多 agent CLI 与 sandbox 命名
    ├── output.sh                  # 流输出解析（JSON → 文本）与 ANSI 清洗
    ├── preview.sh                 # 终端滚动预览（ANSI 光标控制）
    ├── promise.sh                 # 语义标签：COMPLETE / BLOCKED / DECIDE / TASK-{ID}:DONE
    ├── spinner.sh                 # 动画 spinner + 当前步骤识别
    ├── timing.sh                  # 每轮/每步骤耗时统计
    ├── preflight.sh               # 启动前检查（git 仓库、必需文件、history 目录）
    └── args.sh                    # 命令行参数解析（--max-iterations, --agent, --once）
```

## 二、ralph.sh 核心循环逻辑

`ralph.sh` 的循环主体在 `for i in $(seq 1 $MAX_ITERATIONS)` 内，关键步骤：

1. **构造 Prompt**：把 `PROJECT_ROOT=$SCRIPT_DIR` 与 `.agent/PROMPT.md` 拼接为环境变量 `PROMPT_CONTENT`。
2. **启动 Agent 子进程**：通过 `script -q "$OUTPUT_FILE" bash -c "$AGENT_COMMAND"` 在伪 TTY 中运行 `sbx run --name <sandbox> <agent> .`，实现与 Docker Sandboxes 的交互隔离。
   - 对 Claude 默认使用 `--output-format stream-json --verbose -p "$PROMPT_CONTENT"`。
3. **流式监控**：循环读取 `OUTPUT_FILE` 的新增字节，调用 `parse_json_content` 解析 JSON 流，得到可读文本后：
   - 追加到 `FULL_OUTPUT_FILE`；
   - 通过 `update_spinner_step` 与 `update_preview_line` 更新终端 spinner 和滚动预览。
4. **Agent 退出后**：
   - 保存清洗后的原始输出到 `.agent/history/ITERATION-${SESSION_ID}-${i}.txt`；
   - 提取 `{"type":"result"}` 作为 `FINAL_SUMMARY`；
   - 关闭 spinner 与预览区，展示摘要；
5. **状态判定**：扫描输出是否包含：
   - `<promise>COMPLETE</promise>` → 退出码 `0`；
   - `<promise>BLOCKED:reason</promise>` → 退出码 `2`；
   - `<promise>DECIDE:question</promise>` → 退出码 `3`；
   - 否则打印本轮耗时并进入下一轮；达到 `MAX_ITERATIONS` 退出码 `1`。

核心代码片段：

```bash
# 启动命令
AGENT_COMMAND=$(build_agent_command "$RALPH_AGENT" "$RALPH_SANDBOX_NAME")
script -q "$OUTPUT_FILE" bash -c "$AGENT_COMMAND" >/dev/null 2>&1 &
AGENT_PID=$!

# 监控输出
while kill -0 "$AGENT_PID" 2>/dev/null; do
  # tail -c 读取新增字节，逐行解析 JSON 并更新 spinner/preview
  ...
  sleep 0.2 || true
done

# 状态判定
if has_complete_tag "$OUTPUT" || has_complete_tag "$FINAL_SUMMARY"; then
  exit $EXIT_COMPLETE
fi
if has_blocked_tag "$OUTPUT" || has_blocked_tag "$FINAL_SUMMARY"; then
  exit $EXIT_BLOCKED
fi
if has_decide_tag "$OUTPUT" || has_decide_tag "$FINAL_SUMMARY"; then
  exit $EXIT_DECIDE
fi
```

## 三、状态记录与检查机制

### 3.1 任务状态

- `tasks.json` 保存所有任务及 `passes: bool`；
- `tasks/TASK-{ID}.json` 保存单任务详细步骤（当前仓库仅 `.gitkeep`，说明需先用 prd-creator 生成）；
- `PROMPT.md` 明确要求：每轮只完成一个任务，提交后输出 `<promise>TASK-{ID}:DONE</promise>` 并立即 STOP；所有任务完成输出 `<promise>COMPLETE</promise>`。

### 3.2 过程记录

- `LOG.md`：由 agent 写入日期、摘要、截图路径；
- `STRUCTURE.md`：记录目录结构变化；
- `HISTORY_DIR`：每轮原始输出快照，文件名带 `SESSION_ID` 避免多运行覆盖；
- `STEERING.md`：人工注入的优先工作，agent 每轮先检查，若存在则先完成其中的关键项再继续任务流。

### 3.3 迭代元数据

- 迭代次数、单轮耗时、平均耗时、与上轮差值（Δ）按步骤分类统计（Thinking / Planning / Reading code / Implementing / Testing / Linting / Typechecking / Committing 等），在 `timing.sh` 中通过 `detect_step` 正则匹配输出内容识别。

## 四、与 agent-diva 现有能力的映射

| ralph-loop 概念 | ralph-loop 实现 | agent-diva 对应 / 可复用 | 差异 / 可借鉴点 |
|---|---|---|---|
| **长循环** | bash `for` 循环 + `MAX_ITERATIONS` | `agent_loop.rs` 中的 `AgentLoop`，已有 `max_iterations` | ralph 的“每轮完全重启一次外部 agent 进程”适合跨轮状态隔离；diva 目前是同进程内 turn 循环，可考虑增加“每轮 fork/spawn 独立子 agent 执行单个任务”的隔离模式。 |
| **任务队列** | `.agent/tasks.json` + `tasks/TASK-{ID}.json` | 当前 diva 更偏向 prompt 内嵌任务或计划工具；`.agent` 目录是 diva 已有工作目录（`.agent/tasks.json`）的沿用 | 可借鉴 ralph 的“任务 JSON + 单任务单文件”结构，便于长期任务列表持久化与优先级排序。 |
| **Prompt 模板** | `.agent/PROMPT.md` 作为每轮固定指令 | diva 的 Mask / Soul / SystemPrompt 机制 | ralph 的 PROMPT.md 是“任务驱动型”硬约束，可引入到 diva 的专用任务循环 mask 或子 agent 模板中。 |
| **人工干预** | `.agent/STEERING.md` 优先检查 | diva 的 runtime_control 或 `cancelled_sessions` | 可增加“每轮优先读取 steering 文件”的钩子，让外部操作者随时注入高优先级任务。 |
| **语义标签** | `<promise>` 标签（COMPLETE/BLOCKED/DECIDE/TASK-DONE） | diva 的 `SubAgentResult` / `SubAgentStatus` / tool 返回结果 | ralph 的 XML 标签是一种跨进程/跨 agent CLI 的轻量契约，diva 子 agent 输出也可约定标准化标签，便于循环解析。 |
| **子进程执行** | `script -q + sbx run --name <sandbox> <agent>` | `SubagentManager::spawn` / `run_supervised` / `spawn_batch`；`SpawnTool` | ralph 使用 Docker Sandboxes 的 `sbx` 做外部隔离；diva 的 `agent-diva-sandbox` 是进程级沙箱，二者隔离层级不同。 |
| **Sandbox 命名** | `ralph-<agent>-<project>-<hash8>` | diva 的 `SandboxManager` / `SandboxPolicy` | 可借鉴“确定性命名”策略，让 diva 的子 agent 容器/进程也按项目+agent 命名，便于复用与清理。 |
| **流输出解析** | `output.sh` 解析 `stream-json` 与 ANSI 清洗 | diva 的 provider 层已有流式处理 | 若 diva 支持调用外部 agent CLI，可复用类似的 JSONL 解析与 ANSI 清洗逻辑。 |
| **步骤识别** | `timing.sh` 用正则识别当前步骤并统计耗时 | diva 的 AuditSink / ProviderTap / ToolTap 观测 | ralph 的步骤识别是在外层控制脚本中做轻量推断；diva 可结合工具调用元数据做更精确的阶段追踪。 |
| **通知** | 检测到 BLOCKED/DECIDE 时播放声音、弹通知 | diva 的 channel 通知机制 | 可借鉴“当子 agent 需要人决策时主动通知用户”的 UX。 |

## 五、对 agent-diva 下一阶段的设计建议

1. **引入“任务循环”模式**：在 `AgentLoop` 中增加一种可选的 `task-loop` 模式，每轮把一个 `TaskItem` 作为独立上下文交给外部或内部子 agent，执行后检查 `<promise>` 标签，再决定下一轮。
2. **子 agent 输出契约**：给 `SubagentManager` / `SpawnTool` 约定类似 `<promise>` 的结构化结束信号，方便外层循环统一处理 COMPLETE/BLOCKED/DECIDE。
3. **持久化任务目录**：参考 `.agent/tasks.json` + `tasks/TASK-{ID}.json` 在 diva 中落地任务队列文件，与 `RunStore` 或计划工具打通。
4. **Steering 钩子**：在 `agent_loop.rs` 的每轮起始或子 agent 执行前，检查 `STEERING.md` 或一个配置化的高优先级任务源，实现人工随时纠偏。
5. **确定性隔离**：`build_sandbox_name` 的命名方式可用于 diva 的 sandbox/子进程执行器，保证同一项目+同一任务多次执行可复用同一沙箱，便于缓存登录态与依赖。

## 六、结论

ralph-loop 的核心价值在于：**一个轻量、可 hack 的外部 shell 循环，把 LLM agent CLI 当作“一次性的任务执行容器”，通过文件系统（tasks.json、PROMPT.md、STEERING.md、LOG.md）和语义标签（promise）完成状态机控制**。agent-diva 的现有架构在 `SubagentManager`、`SpawnTool` 和 `agent-diva-sandbox` 上已有更工程化的基础，下一阶段可以吸收 ralph 的“任务清单持久化 + 单任务单轮 + 人工 steering + 标准化结束标签”思想，补齐 diva 在长时间、多任务、可监督循环场景下的设计。
