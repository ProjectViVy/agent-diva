# Loop Engineering 参考报告：oh-my-opencode 与 loops 的提炼

> 调研目标：为 agent-diva 下一阶段（Track C：Autonomous Activity / Loop Engineering）提供可直接映射的设计参考。
> 资料来源：
> - `C:\Users\Administrator\Desktop\morediva\.workspace\loops\originals\oh-my-opencode`
> - `C:\Users\Administrator\Desktop\morediva\.workspace\loops\originals\loops`

---

## 1. 核心循环结构

两个项目共享同一套“触发 → 计划 → 执行 → 验证 → 记忆 → 决策 → 重复”的骨架，但封装方式不同：

| 阶段 | oh-my-opencode | loops | 可给 diva 的启示 |
|---|---|---|---|
| 触发 | 用户通过 `/refactor`、`/start-work` 或 Sisyphus 自动分类触发 | `loop <prompt>` 或 `PLAN.md` 已存在时自动进入 | 保留用户显式触发，并为 cron/heartbeat 预留入口 |
| 计划 | Prometheus 生成 `.sisyphus/plans/{name}.md`，Ralph Loop 写入 `PLAN.md` | 默认先写 `PLAN.md`，可选 `--review-plan` 让对侧 agent 评审 | 计划文件是循环的“脊柱”，应持久化在 workspace |
| 执行 | Sisyphus 或 background-agent 启动子会话；`call_omo_agent` 并行委派 | `runAgent` 驱动 Claude/Codex 子进程；`worktree` 隔离 | 执行层必须是独立会话/进程，避免阻塞主会话 |
| 验证 | 6 阶段模板要求每一步 `lsp_diagnostics` + 测试 + typecheck；`momus` 评审 | `REVIEW_PASS` / `REVIEW_FAIL` 信号；`review.ts` 解析最终行 | 验证必须结构化，用信号而非自然语言终止 |
| 记忆 | `boulder.json` 记录当前计划、session_ids、进度；background-agent 内存 Map | `manifest.json` + `transcript.jsonl` + `bridge.jsonl` | 运行状态需写入文件，支持跨进程恢复 |
| 决策 | Sisyphus 根据分类和进度决定下一步；`delegate_task` 并行 | 根据 `doneSignal` 和 review 结果决定继续/停止 | 决策由独立 orchestrator 而非执行 agent 自己判断 |
| 重复 | Ralph Loop 100 次迭代；`session.idle` 或 polling 触发了下一轮 | `iterationCooldown` + `maxIterations` 控制循环 | 必须设上限、冷却和 kill switch |

---

## 2. Loop Engineering 构件覆盖度

### 2.1 automation（触发/编排）

oh-my-opencode 的 `sisyphus-prompt.md` 把触发逻辑做成系统 prompt 的“Phase 0 - Intent Gate”：

```markdown
<Behavior_Instructions>
## Phase 0 - Intent Gate (EVERY message)
### Step 0: Check Skills FIRST (BLOCKING)
IF request matches a skill trigger:
  → INVOKE skill tool IMMEDIATELY
  → Do NOT proceed to Step 1 until skill is invoked
```

loops 则通过 CLI 入口和 `PLAN.md` 存在性自动触发计划或执行：

```ts
// src/loop/task.ts
export const resolveTask = async (opts: Options): Promise<string> => {
  const source =
    opts.promptInput ?? (isFile(PLAN_FILE) ? PLAN_FILE : undefined);
  if (!source) throw new Error(MISSING_PROMPT_ERROR);
  if (!opts.promptInput || isMarkdownInput(opts.promptInput)) {
    return await readPrompt(source);
  }
  const task = opts.promptInput;
  await runPlanMode(opts, task);  // 先写 PLAN.md
  return await readPrompt(PLAN_FILE);
};
```

### 2.2 worktree（隔离）

loops 的 `worktree.ts` 用 git worktree 为每次运行创建隔离分支，避免污染主工作区：

```ts
// src/loop/worktree.ts
const buildWorktreeBranch = (base: string, runId: string | number): string =>
  buildLoopName(base, runId);

const buildWorktreePath = (
  repoRoot: string,
  base: string,
  runId: string | number
): string => join(dirname(repoRoot), buildWorktreeBranch(base, runId));

export const maybeEnterWorktree = (opts: Options): void => {
  if (!opts.worktree) return;
  // ... 检查是否已在 worktree 中，创建/复用分支，切换 cwd
};
```

### 2.3 skills（能力注入）

oh-my-opencode 把 skill 作为系统 prompt 的强制前置层：

```markdown
### Step 0: Check Skills FIRST (BLOCKING)
| Skill | When to Use |
|-------|-------------|
| `playwright` | MUST USE for any browser-related tasks |
| `frontend-ui-ux` | Designer-turned-developer who crafts stunning UI/UX |
| `git-master` | 'commit', 'rebase', 'squash', 'who wrote', ... |
```

对应 diva 的 mask/skill 体系，可将 skill 触发做成“在 Loop 启动前强制检查的技能列表”。

### 2.4 connectors（子代理通信）

loops 的 `bridge.jsonl` 实现 Claude ↔ Codex 的跨进程消息总线：

```ts
// src/loop/bridge-store.ts
export interface BridgeMessage extends BridgeBaseEvent {
  kind: "message";
  message: string;
}
export const appendBridgeMessage = (
  runDir: string,
  source: Agent,
  target: Agent,
  message: string
): BridgeMessage => {
  const entry: BridgeMessage = {
    at: new Date().toISOString(),
    id: crypto.randomUUID(),
    kind: "message",
    message,
    signature: bridgeSignature(source, target, message),
    source,
    target,
  };
  appendBridgeEvent(runDir, entry);
  appendRunTranscriptEntry(buildTranscriptPath(runDir), {
    at: entry.at, from: source, message, to: target,
  });
  return entry;
};
```

oh-my-opencode 则通过 `call_omo_agent` 和 `background-agent` 实现同进程内的子代理委派。

### 2.5 sub-agents（子代理）

oh-my-opencode 的 background-agent 是完整的子代理生命周期管理：

```ts
// src/features/background-agent/types.ts
export interface BackgroundTask {
  id: string
  sessionID?: string
  parentSessionID: string
  parentMessageID: string
  description: string
  prompt: string
  agent: string
  status: BackgroundTaskStatus
  queuedAt?: Date
  startedAt?: Date
  completedAt?: Date
  result?: string
  error?: string
  progress?: TaskProgress
  concurrencyKey?: string
  concurrencyGroup?: string
}
```

loops 的 runner 把子代理当外部进程调度：

```ts
// src/loop/runner.ts
const runLegacyAgent = async (
  agent: Agent,
  prompt: string,
  opts: Options,
  sessionId?: string,
  kind: AgentRunKind = "work"
): Promise<RunResult> => {
  const { args, cmd } = buildCommand(agent, prompt, model, sessionId, opts);
  const proc = spawn([cmd, ...args], {
    detached: DETACH_CHILD_PROCESS,
    env: process.env,
    stderr: "pipe",
    stdout: "pipe",
  });
  activeChildren.add(proc);
  // ... 消费 stdout/stderr
};
```

### 2.6 memory（状态/记忆）

oh-my-opencode 用 `boulder.json` 做计划状态：

```json
// .sisyphus/boulder.json
{
  "active_plan": "/absolute/path/to/plan.md",
  "started_at": "ISO_TIMESTAMP",
  "session_ids": ["session_id_1", "session_id_2"],
  "plan_name": "plan-name"
}
```

loops 用 `manifest.json` + `transcript.jsonl`：

```ts
// src/loop/run-state.ts
export interface RunManifest {
  claudeSessionId: string;
  codexThreadId: string;
  createdAt: string;
  cwd: string;
  mode: string;
  pid: number;
  repoId: string;
  runId: string;
  state: RunLifecycleState;  // submitted | working | reviewing | ...
  status: RunStatus;
  tmuxSession?: string;
  updatedAt: string;
}

export type RunTranscriptEntry =
  | RunMessageTranscriptEntry
  | RunResultTranscriptEntry
  | RunReviewTranscriptEntry
  | RunStatusTranscriptEntry;
```

---

## 3. 与 diva 现有架构的映射/借鉴点

| diva 现有模块 | 对应 loops/oh-my-opencode 构件 | 借鉴方式 |
|---|---|---|
| cron / heartbeat | loops 的 `iteration.ts` + `run-state.ts`；oh-my-opencode 的 `background-agent/manager.ts` polling | 把 cron 触发抽象为“LoopScheduler”，负责唤醒、超时、TTL |
| spawn（子代理） | `background-agent` / `runner.ts` | 引入 `BackgroundTask` 状态机，补全 cancel/progress/retry |
| sandbox | loops `--worktree` / `--tmux` | 每次 Loop 可绑定一个 git worktree 或 sandbox 目录作为安全边界 |
| laputa memory | `boulder.json` / `manifest.json` + `transcript.jsonl` | 运行轨迹写入 Mentle 的 runtime state，Laputa 只负责 authority 写入 |
| mask / skill | oh-my-opencode 的 `sisyphus-prompt.md` skills 检查 | Loop 启动前强制加载 skills，与 mask 的 tool policy 联动 |
| behavioral audit | loops 的 `transcript.jsonl`；oh-my-opencode 的 `session.idle` 验证 | 每个 loop 事件都是审计事件，写入 audit log |

**关键借鉴**：oh-my-opencode 的 `background-agent` 是现有 diva `spawn` 的“完整生命周期”版本，它补全了 diva 当前缺失的 cancel/progress/retry。loops 则给出了“跨进程运行状态持久化”的最小可行方案，适合作为 diva 后台 Loop 的骨架。

---

## 4. 最值得 diva 复用的 3 个具体设计/代码片段

### 4.1 设计一：运行状态文件化（loops 的 `manifest.json` + `transcript.jsonl`）

**理由**：diva 当前子代理结果仅存于内存，重启即丢失。loops 用文件作为运行状态脊柱，支持跨进程恢复和审计。

**关键文件**：`src/loop/run-state.ts`

```ts
export interface RunManifest {
  claudeSessionId: string;
  codexThreadId: string;
  createdAt: string;
  cwd: string;
  mode: string;
  pid: number;
  repoId: string;
  runId: string;
  state: RunLifecycleState;  // submitted | working | reviewing | ...
  status: RunStatus;
  tmuxSession?: string;
  updatedAt: string;
}

export type RunTranscriptEntry =
  | RunMessageTranscriptEntry
  | RunResultTranscriptEntry
  | RunReviewTranscriptEntry
  | RunStatusTranscriptEntry;

export const setRunManifestState = (
  manifest: RunManifest,
  state: RunLifecycleState,
  now = new Date().toISOString()
): RunManifest => ({
  ...manifest,
  state,
  status: runStatusFromState(state),
  updatedAt: now,
});
```

**diva 映射**：把每个 Loop 抽象为 `Activity`（类似 RunManifest），状态写入 `workspace/.diva/activities/{activity_id}/manifest.json` 和 `transcript.jsonl`，支持恢复。

---

### 4.2 设计二：并发限制与队列管理（oh-my-opencode 的 `ConcurrencyManager`）

**理由**：diva 当前 `spawn_batch` 有 bug（不写入 `running_tasks`），且没有并发控制。ConcurrencyManager 提供了按 provider/model 的限流和队列。

**关键文件**：`src/features/background-agent/concurrency.ts`

```ts
export class ConcurrencyManager {
  private counts: Map<string, number> = new Map()
  private queues: Map<string, QueueEntry[]> = new Map()

  async acquire(model: string): Promise<void> {
    const limit = this.getConcurrencyLimit(model)
    if (limit === Infinity) return
    const current = this.counts.get(model) ?? 0
    if (current < limit) {
      this.counts.set(model, current + 1)
      return
    }
    return new Promise<void>((resolve, reject) => {
      const queue = this.queues.get(model) ?? []
      queue.push({ resolve, rawReject: reject, settled: false })
      this.queues.set(model, queue)
    })
  }

  release(model: string): void {
    const queue = this.queues.get(model)
    while (queue && queue.length > 0) {
      const next = queue.shift()!
      if (!next.settled) {
        next.resolve()  // 直接转交槽位
        return
      }
    }
    const current = this.counts.get(model) ?? 0
    if (current > 0) this.counts.set(model, current - 1)
  }
}
```

**diva 映射**：在 `subagent.rs` 中引入 `SubAgentConcurrencyManager`，按 provider/model 限流，并修复 `spawn_batch` 的 running_tasks 记录。

---

### 4.3 设计三：验证信号与迭代决策（loops 的 `review.ts`）

**理由**：自然语言“完成了”不可靠，loops 用强信号 `<review>PASS</review>` / `<review>FAIL</review>` 和最终行解析，实现确定性的 verify → decide 闭环。

**关键文件**：`src/loop/review.ts`

```ts
const parseSignal = (line: string): ReviewStatus | undefined => {
  const trimmed = line.trim();
  if (trimmed === REVIEW_PASS || trimmed === QUOTED_REVIEW_PASS) return "pass";
  if (trimmed === REVIEW_FAIL || trimmed === QUOTED_REVIEW_FAIL) return "fail";
  return undefined;
};

const evaluateOutput = (result: RunResult): ReviewCheck => {
  if (result.exitCode !== 0) {
    return { status: "fail", reason: formatFailure(`[loop] review exited with code ${result.exitCode}`) };
  }
  const output = cleanOutput(result);
  const signals = parseSignalSummary(output);
  if (!signals.final) {
    return { status: "fail", reason: formatFailure(signals.finalLine ? REVIEW_MALFORMED_SIGNAL : REVIEW_MISSING_SIGNAL) };
  }
  if (final === "pass" && signals.hasFailSignal) {
    return { status: "fail", reason: formatFailure(REVIEW_MIXED_SIGNALS) };
  }
  // ...
  return { status: "pass", reason: "" };
};
```

**diva 映射**：在 Loop 的 verifier 阶段定义自己的信号，例如 `<diva-verify status="pass">...</diva-verify>`，由 orchestrator 解析，而不是让执行 agent 自己决定何时停止。

---

## 5. 给 diva Track C 的落地建议

1. **先补 Track B**：Loop Engineering 必须建立在 hot reload、audit、Poke 8 事件、prompt injection、PII 这些 P0 之上。没有安全边界，循环就是失控风险。
2. **Loop 状态文件化**：每个 Loop 活动对应一个目录（`workspace/.diva/activities/{id}/`），包含 `plan.md`、`manifest.json`、`transcript.jsonl`。
3. **独立 LoopScheduler**：不要把循环逻辑放在主 chat session 里，用一个独立调度器负责触发、轮询、超时、完成通知。
4. **工具限制继承**：Loop 执行时通过 mask/skill 限制工具集（如禁止无审批的 `write_file`/`terminal`），与 diva 的 `ToolAssembly.with_mask_config()` 联动。
5. **评审信号标准化**：参考 loops 的 `REVIEW_PASS`/`REVIEW_FAIL`，定义 diva 自己的完成/失败/继续信号，让 orchestrator 可程序化决策。

---

*报告字数：约 2300 字（中文）*
