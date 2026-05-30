# Claude Code AutoDream 到 Agent-Diva 的迁移调研

> 状态：调研与方案文档。本文只固化结论，不实现代码。
> 调研对象：`.workspace/claude-code/src/services/autoDream`、Claude Code 直接调用链、Agent-Diva 当前记忆/心跳/节律接缝、`newedge` 设计文档。

## 1. 结论摘要

Claude Code 中确实存在 AutoDream，实现位置是 `.workspace/claude-code/src/services/autoDream/`。它不是泛泛的“自动压缩”，而是一套后台记忆整理机制：每个主会话回合结束后 opportunistic 检查是否满足门控，满足后用 forked subagent 运行 memory consolidation；同时提供手动 `/dream` 入口。

对 Agent-Diva，AutoDream 的迁移可能性很大，但应该迁移运行时骨架，不应该照搬 Claude Code 的产品语义。

建议目标形态：

```text
RhythmTrigger
  -> AutodreamWorker
  -> Context / evidence pre-compact when needed
  -> daily / weekly / monthly rhythm
  -> LearningCandidate / Laputa delta / SOP or Skill candidate
  -> Journal
  -> Chat re-wakeup
```

核心判断：

- Claude Code 有 AutoDream，前面“没有直接实现”的判断是错误结论，已纠正。
- Claude Code 的 AutoDream 是 auto-memory consolidation，不是 Agent-Diva 所需的完整节律梦境系统。
- Agent-Diva 的 autodream 应以 NewEdge/Laputa 语义为主：节律整理、日报/周报/月报、Journal、learning candidates、SOP/Skill 候选、SOUL/MEMORY 演化候选、Mentle 深层整理。
- 最值得迁移的是触发、锁、后台子 agent、最小权限、进度可观测和 extract/consolidate 分层。
- 不建议照搬 `/dream` 的乐观更新时间戳行为；Agent-Diva 应只在任务成功后更新 checkpoint。

## 2. 调研纠错记录

初始判断曾认为 `.workspace/claude-code` 没有直接叫 `autodream` 的实现，只存在 scheduled tasks、daemon/background、auto/manual compact 等相邻机制。该判断不完整。

用户指出存在路径：

```text
.workspace/claude-code/src/services/autoDream/autoDream.ts
```

重新调研后确认：

- AutoDream 实现存在。
- 配套文档存在：`.workspace/claude-code/docs/features/auto-dream.md`。
- 配套配置、锁、提示词、任务 UI、手动 `/dream` 入口均存在。

后续所有结论以重新调研和子 agent 专项调研为准。

## 3. Claude Code AutoDream 实现事实

### 3.1 核心文件

Claude Code AutoDream 的核心文件：

- `.workspace/claude-code/src/services/autoDream/autoDream.ts`
- `.workspace/claude-code/src/services/autoDream/config.ts`
- `.workspace/claude-code/src/services/autoDream/consolidationLock.ts`
- `.workspace/claude-code/src/services/autoDream/consolidationPrompt.ts`
- `.workspace/claude-code/src/skills/bundled/dream.ts`
- `.workspace/claude-code/src/tasks/DreamTask/DreamTask.ts`
- `.workspace/claude-code/src/query/stopHooks.ts`
- `.workspace/claude-code/src/utils/backgroundHousekeeping.ts`

### 3.2 自动触发入口

自动链路不是独立 cron，而是每轮结束后的 opportunistic check：

```text
backgroundHousekeeping.init()
  -> initAutoDream()

turn stop hooks
  -> executeAutoDream(...)
  -> gate checks
  -> tryAcquireConsolidationLock()
  -> runForkedAgent(...)
```

关键点：

- 初始化：`initAutoDream()`。
- 执行入口：`executeAutoDream()`。
- 调用位置：`stopHooks.ts` 在 turn end 后 fire-and-forget 调用。
- 自动触发不阻塞主回答。

### 3.3 自动门控

Claude Code AutoDream 的默认门控：

```text
enabled gate:
  !KAIROS
  !remote mode
  auto memory enabled
  auto dream enabled

time gate:
  hours since lastConsolidatedAt >= minHours
  default minHours = 24

scan throttle:
  default 10 minutes

session gate:
  sessions touched since lastConsolidatedAt >= minSessions
  default minSessions = 5
  current session excluded

lock gate:
  memory/.consolidate-lock
```

这说明它事实上是“节律触发 + 累积量触发”的混合模型，而不是简单定时器。

### 3.4 手动触发入口

手动入口是 `/dream`，实现位于：

```text
.workspace/claude-code/src/skills/bundled/dream.ts
```

手动 `/dream` 与自动 AutoDream 复用 `buildConsolidationPrompt()`，但行为不同：

- `/dream` 在主循环运行。
- `/dream` 拥有完整工具权限。
- 自动 AutoDream 使用 forked agent，并限制工具权限。
- `/dream` 会在构建 prompt 前记录一次 consolidation 时间戳。

最后一点不建议照搬到 Agent-Diva。对长期服务型 agent 来说，“用户手动尝试整理”不应等同于“整理成功完成”。

### 3.5 锁与并发控制

Claude Code 使用 memory 目录内的 `.consolidate-lock`：

- 文件内容：持锁 PID。
- 文件 mtime：`lastConsolidatedAt`。
- 活进程持锁时拒绝并发整理。
- stale lock 可恢复。
- forked agent 失败或被 kill 时回滚 mtime。

这个设计很适合迁移，因为它低成本、跨进程、易检查，也避免多个后台整理任务互相覆盖。

Agent-Diva 可借鉴，但 checkpoint 应从单一 lock 扩展为 domain-aware checkpoint：

```text
.agent-diva/autodream/state.json
.agent-diva/autodream/autodream.lock
.laputa/rhythm/daily/YYYY-MM-DD.md
.laputa/rhythm/weekly/YYYY-WNN.md
.laputa/rhythm/monthly/YYYY-MM.md
```

### 3.6 后台子 agent 执行

自动 AutoDream 使用 `runForkedAgent()`：

- `querySource = "auto_dream"`
- `forkLabel = "auto_dream"`
- `skipTranscript = true`
- 使用 cache-safe params 复用主会话 cache 相关参数。
- 通过 `onMessage` watcher 折叠后台 agent 输出，更新 DreamTask UI。

这对应 Agent-Diva 的迁移目标：

```text
AutodreamWorker
  -> spawn controlled background agent
  -> read sessions / history / evidence / rhythm
  -> write only allowed output paths
  -> emit progress events
  -> support cancel
```

## 4. 与 Claude Code 其他模块的关系

### 4.1 与 extractMemories 的关系

Claude Code 中 `extractMemories` 与 `autoDream` 都挂在 turn end：

- `extractMemories` 面向当前回合抽取。
- `autoDream` 面向跨 session 整理、去重、修剪和索引更新。
- 两者共享 `createAutoMemCanUseTool()` 的权限模型。

这对 Agent-Diva 很重要。Agent-Diva 不应把每轮抽取、上下文压缩、日报生成、SOP 候选、长期记忆重写混成一个巨大任务。

建议拆分：

```text
turn-end extraction:
  sync_turn / lightweight candidate capture

threshold consolidation:
  current session segment summary

rhythm autodream:
  daily / weekly / monthly review

candidate promotion:
  user or policy confirmed write to L2/L3/Laputa/Mentle
```

### 4.2 与 DreamTask UI 的关系

Claude Code 把 AutoDream 暴露为后台任务：

- 注册 DreamTask。
- 展示最近 assistant turn。
- 展示 tool use count。
- 记录 touched files。
- 允许用户 kill。
- 完成后在主 transcript 里插入 memory saved 类系统消息。

Agent-Diva 应做类似的可观测性，但 UI 应映射到 Diva 的 Chat / Journal 设计：

- Chat 中显示 `ReviewCard` 或后台任务提示。
- Journal 展示完成后的 rhythm entry。
- GUI 提供“取消”“查看日志”“重新唤醒 autodream”“回到对话继续”。

### 4.3 与 memory 目录的关系

Claude Code AutoDream 以 auto-memory 目录为权威写入边界：

```text
memory/
  MEMORY.md
  topic files
  .consolidate-lock
```

Agent-Diva 不能照搬成单一 memory 目录，因为 NewEdge 已经明确存储边界：

```text
.agent-diva/
  runtime state
  plans/
  autodream state

.laputa/
  identity
  rhythm/
  inbox/
  sop/
  MEMORY.md
  relationships.md
  expectations.md

Mentle
  optional deep fact / evidence / graph backend
```

## 5. Agent-Diva 现有接缝

Agent-Diva 当前并非从零开始。已有接缝包括：

- `MemoryProvider::system_prompt_block()`：启动或 prompt 注入。
- `MemoryProvider::prefetch()`：live turn 召回，只读。
- `MemoryProvider::sync_turn()`：turn 后持久化。
- `MemoryProvider::on_session_end()`：session 结束后的 rhythm/shutdown hook。
- `agent-diva-agent/src/consolidation.rs`：达到窗口后总结旧会话段。
- `agent-diva-core/src/heartbeat/service.rs`：周期心跳和 `trigger_now()` 手动触发模型。
- NewEdge 文档已定义 offline path：`session/history evidence -> daily rhythm / autodream -> LearningCandidate -> user or policy decision`。

所以迁移 AutoDream 时不需要先改造整个 AgentLoop。第一阶段应把它做成 MemoryProvider 边界外侧的 runtime worker。

## 6. 原文档中 Autodream 的地位

`docs/dev/laputa-new-architecture.md` 对 autodream 的定位很高：

- 新 Laputa = 极简 7 文件身份管理 + 三轴主体性 + 进阶心跳子代理委派 + autodream 每日整理。
- mentle 日常只暴露 4 个简化工具，但 autodream 可使用全量 30+ 能力。
- autodream = 每日深度整理：日报 + SOP/Skill 产出 + SOUL 演化 + MEMORY 压缩。

`docs/dev/genericagent/newedge/architecture.md` 进一步约束：

- 在线路径保持轻量。
- 离线路径承担整理、去重、归档和学习候选处理。
- daily rhythm 是 autodream 的主要输出，不是装饰性报告。
- daily rhythm 可以产出 L2/L3/Laputa delta 候选，但不能无审计地重写 L2/L3。
- Laputa rhythm 可通过 `MemoryProvider.on_session_end()` 或未来 autodream worker 触发。
- Plan Mode 不能直接写 daily/monthly report；计划产出 evidence，rhythm/autodream 决定如何形成 journal record。

`docs/dev/genericagent/newedge/ui-design.md` 则定义产品承载：

- Journal 是 archive / review / re-wakeup surface。
- Autodream 可由 rhythm schedule、user request、Journal re-wakeup 启动。
- Autodream output 必须可审计。
- 旧 Journal entry 不应静默改写，修正应形成 follow-up 或 amendment record。
- Autodream 产生的 memory write 仍需经过 learning candidates 和用户确认。

因此，Agent-Diva 的 autodream 不是后台整理小功能，而是长期连续性、主体性、学习闭环和 Journal 产品体验的核心机制之一。

## 7. 迁移方案建议

### 7.1 不建议直接复制 Claude Code AutoDream

不应直接复制的部分：

- Claude Code 的 `stopHooks` / Ink REPL / task store / bundled skill 体系。
- GrowthBook 实验键名和设置结构。
- KAIROS 排除语义。
- 基于 transcript mtime 数量的 session gate。
- `/dream` 先写 consolidation timestamp 的乐观 stamp。
- 只整理 `MEMORY.md` 和主题记忆文件的窄语义。

### 7.2 建议迁移的骨架

建议直接借鉴或改写的机制：

- turn-end 后台检查一次，而不是新开重型 cron。
- 时间门 + 累积量门 + 扫描节流 + 锁门。
- 自动触发 + 手动触发共用同一 worker。
- 文件锁 + checkpoint。
- 后台子 agent / worker 隔离。
- 最小权限：读 evidence，写 rhythm/candidate 输出。
- 任务进度可观测和可取消。
- extract 与 consolidate 分层。
- 失败不破坏主 session persistence。

### 7.3 Agent-Diva 目标架构

建议新增概念：

```text
AutodreamConfig
AutodreamTrigger
AutodreamJob
AutodreamCheckpoint
AutodreamWorker
AutodreamProgressEvent
AutodreamOutput
```

建议触发源：

```text
TurnEndThreshold
SessionEnd
HeartbeatRhythm
ManualDreamCommand
JournalRewake
PlanCompleted
```

建议输出：

```text
.laputa/rhythm/daily/YYYY-MM-DD.md
.laputa/rhythm/weekly/YYYY-WNN.md
.laputa/rhythm/monthly/YYYY-MM.md
.laputa/inbox/learning-candidates.jsonl
.laputa/sop/*.candidate.md
.agent-diva/autodream/events.jsonl
.agent-diva/autodream/state.json
```

### 7.4 上下文压缩策略

需要做上下文压缩，但不能把“压缩”误认为 autodream 本体。

建议规则：

- Autodream 启动前先估算 evidence / session / history 规模。
- 超过阈值时先生成 source capsule。
- source capsule 必须保留 evidence refs。
- 后续 rhythm report 基于 capsule + refs 生成。
- 原始 session/history/evidence 不删除。
- 压缩失败不能阻断 session persistence，只影响本次 autodream。

建议流程：

```text
collect evidence refs
  -> if oversized: compact to source capsule
  -> run autodream worker
  -> write rhythm entry
  -> emit learning candidates
  -> show Journal / Chat card
```

## 8. 分阶段实施建议

### P0：只做文档与边界确认

- 固化本文结论。
- 明确 autodream 与 consolidation、heartbeat、Journal、Plan Mode、Mentle 的边界。
- 不改运行时代码。

### P1：Autodream 状态与锁

- 新增 `AutodreamCheckpoint`。
- 新增锁文件与成功 checkpoint。
- 不运行 LLM，只验证门控和并发。

### P2：手动触发最小闭环

- 提供 `/dream` 或 manager/GUI action。
- 手动触发 worker。
- 输出一份 `.laputa/rhythm/daily/*.md` 草稿。
- 成功后更新 checkpoint。

### P3：自动触发

- turn-end 或 session-end 检查。
- 默认关闭。
- 支持时间门、dirty counter/session counter、扫描节流。
- 失败只记录事件，不影响主会话。

### P4：Journal 与 Chat re-wakeup

- Journal 展示 rhythm outputs。
- Journal 操作回到 Chat。
- `ReviewCard` / `JournalRefCard` 承载用户确认和后续行动。

### P5：LearningCandidate 与 Mentle 深层整理

- Autodream 只产出候选。
- 用户确认或策略确认后写 L2/L3/Laputa/Mentle。
- Mentle full/custom 能力只在 autodream worker 中按配置开放。

## 9. 最终建议

Agent-Diva 应该实现自己的 `AutodreamWorker`，但以 Claude Code AutoDream 作为工程参考。

一句话方案：

```text
复制骨架，不复制语义；接入 Diva rhythm，不接管 AgentLoop；先手动，后自动；先产出 Journal，再推动候选学习。
```

优先级最高的设计约束：

- Autodream 是离线路径，不阻塞在线对话。
- Autodream 输出必须可审计。
- Journal 是展示与重新唤醒界面，不是执行器。
- Memory writes 必须经过 candidate/confirmation。
- 成功后才更新 checkpoint。
- 失败不破坏 session persistence。
- Plan Mode 产出 evidence，autodream/rhythm 决定是否形成长期节律记录。
