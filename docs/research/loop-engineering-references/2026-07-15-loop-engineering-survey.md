# Loop Engineering 调研报告（2026-07-15）

> 调研目标：为 agent-diva 下一阶段的自主活动（Autonomous Activity）大型更新寻找 Loop Engineering 实现参考。
> 调研日期：2026-07-15
> 调研人：松本（AGENT-Matsumoto）
> 本地仓库：`C:\Users\Administrator\Desktop\morediva\.workspace\loops\`

---

## 一、调研范围

本次共调研 11 个来源，分为三类：

| 类别 | 来源 | 本地路径 | 研究方式 |
|---|---|---|---|
| 概念/宣言 | 橙皮书 | `loops/orange-book/` | 直接读 |
| 实践库 | cobusgreyling/loop-engineering | `loops/cobusgreyling/` | 直接读 |
| AI 伴侣实现 | Alife | `alife/source/` | 直接读源码 |
| 工程型 Loop Engine | atomic | `loops/originals/atomic/` | 子代理 + 验证 |
| 工程型 Loop Engine | openharness | `loops/originals/openharness/` | 直接读 |
| 工程型 Loop Engine | oh-my-opencode | `loops/originals/oh-my-opencode/` | 直接读 |
| 工程型 Loop Engine | loops | `loops/originals/loops/` | 直接读 |
| 工程型 Loop Engine | ralph-loop | `loops/originals/ralph-loop/` | 子代理 + 验证 |
| 工程型 Loop Engine | openclaw | `loops/originals/openclaw/` | 子代理 |
| 工程型 Loop Engine | loopwright | `loops/originals/loopwright/` | 直接读 |
| 工程型 Loop Engine | opencode | `loops/originals/opencode/` | 未读 |

---

## 二、核心认知修正：Diva 不是做工程 Loop，而是做 Alife 愿景 + 工程 Loop Engine 底座

调研过程中发现，11 个来源的目标差异极大：

| 类型 | 代表 | 目标 | 对 Diva 的意义 |
|---|---|---|---|
| **愿景型 AI 伴侣** | Alife | 让 AI 成为持续存在的自主体，探索世界、陪伴用户、主动行动 | **Diva 想成为的最终形态** |
| **工程型 Loop Engine** | atomic, openharness, loopwright, cobusgreyling | 解决软件开发中的具体循环问题（PR、测试、review、部署） | **Diva 需要的基础设施** |
| **多 channel Gateway** | openclaw | 让 AI 接入 Discord/Slack/Telegram/QQ/微信等 | **Diva 的触手** |

**结论**：Diva 的志向是 Alife 的愿景，但要实现这个愿景，必须自己搭建或复用一套工程 Loop Engine。不能把工程 Loop Engine 当成最终目标，也不能把 Alife 的 SystemEvent 当成唯一机制。

---

## 三、橙皮书核心观点（本次调研的指引）

### 3.1 Loop Engineering 的层级跃迁

| 层级 | 特征 |
|---|---|
| 提示工程 | 一次对话，一次结果 |
| 上下文工程 | 让 agent 能读到足够背景 |
| 工具工程（Harness） | 给 agent 配工具、环境 |
| **Loop Engineering** | **让系统自己循环：发现→委派→验证→记忆→决策** |

### 3.2 六大构件

1. **Automation** — 触发器（cron / 事件 / 通知）
2. **Worktree** — 隔离执行空间
3. **Skills** — 可复用的领域知识包
4. **Connectors** — 连接外部工具（MCP / API）
5. **Sub-agents** — 分工（实现者 / 验证者 / 规划者）
6. **Memory** — 跨轮次的持久状态

### 3.3 四个成本陷阱

- **Verification debt** — 不做验证，错误积累
- **Comprehension rot** — 上下文越来越长，AI 越来越糊涂
- **Token blowout** — 长期运行成本爆炸
- **Cognitive surrender** — 人放弃审查，让 agent 自己跑

这对 Diva 的提醒：自主活动必须同时带记忆压缩和成本/治理 kill switch。

---

## 四、各项目详细提炼

### 4.1 cobusgreyling/loop-engineering — 最实用的工程化约定

- 7.7k stars，提供 L1/L2/L3 分级、可运行 patterns、CLI 工具
- 核心文件约定：
  - `LOOP.md` — 声明本仓库有哪些 loop，级别、触发频率、验证方式
  - `STATE.md` — 当前 loop 状态、待办、忽略
  - `loop-constraints.md` — 安全约束
  - `loop-run-log.md` — 每次运行日志
  - `loop-budget.md` — token 预算、kill switch
  - `AGENTS.md` — agent 行为协议
- 三级自主度：L1 观察、L2 执行低风险修复、L3 完全自主
- 七种生产模式：Daily Triage、PR Babysitter、CI Sweeper、Dependency Sweeper、Changelog Drafter、Post-Merge Cleanup、Issue Triage

**对 Diva 的价值**：直接借鉴文件约定和 L1/L2/L3 分级，作为 Diva 自主活动的治理框架。

### 4.2 Alife — 愿景型 AI 伴侣的参考实现

Alife 是之前做 Diva 时调研的"自主能力"项目（参考 `agent-diva/DECISION.md`），其自主活动系统由 `Alife.Function.SystemEvent` 实现：

- 基础周期：90 秒，随机偏移 ±30 秒
- 指数退避：乘数 3，最大翻倍 4 次
- 用户打断后重置计时器
- 提供 `Awake` / `Await` 工具让 AI 主动控制报点
- Prompt 引导 AI 自由决定下一步：继续工作、主动找用户、学习、社交等

**Memory 模块**：每次 AI 回复后触发压缩，把聊天记录结构化保存，支持关键词和向量搜索。

**Skill 模块**：每个 skill 是目录 + `SKILL.md`，按需 `StudySkill(name)` 加载。

**Developer 模块**：支持模块重载、角色重载、重启 activity，是"自我升级"的基础。

**对 Diva 的价值**：Alife 是 Diva 最终体验的最直接参考。Diva 的"自由循环"应该长得像 Alife 的 SystemEvent，但要在其基础上加入 Goal/Planner、Verifier、Laputa 记忆、Mentle 治理。

### 4.3 atomic — 最工程化的可验证工作流

- `workflow()` 定义 + `ctx.stage/task/parallel/chain` 执行原语
- `Durable backend`：stage 可重放，用 `replayKey` 查 backend，命中则不再调 LLM
- `ralph-review-gate` / `goal-runner`：Worker → 3 reviewer 并行 → reducer 决策
- Subagent：single/parallel/chain/dynamic/management 五种模式
- 后台运行器：独立 Node 进程，状态机 `pending → running → complete/failed/paused/detached`
- 通过 `events.jsonl` + `status.json` 实时观测
- Extension：`permission-gate`、`dirty-repo-guard`、`git-checkpoint`

**对 Diva 的价值**：直接对应 Diva 的 plan/stage/spawn/governance 四层，是 Loop Engine 执行层最完整的工程参考。

### 4.4 openharness — 最完整的 Repo Autopilot

- `RepoAutopilotStore` 维护任务队列
- 状态机：`queued → accepted → preparing → running → verifying → pr_open → waiting_ci → repairing → completed/merged/failed`
- 默认 12 轮，3 次尝试，human gate
- CI 轮询 20 秒，超时 30 分钟
- `verification_policy.yaml` 配置 gate

**对 Diva 的价值**：状态机、重试/回退策略、CI 轮询可直接借鉴。

### 4.5 oh-my-opencode — 子代理委派与背景任务

- **Atlas hook**：主 orchestrator，强制 "delegate → verify → coordinate"
- **BackgroundManager**：启动后台 session，轮询（2s），检测 stability（3 次空闲）
- **ConcurrencyManager**：按 provider/model 限制并发
- `.sisyphus/` 目录保存计划与 notepad
- 系统 prompt 强制：orchestrator 不能直接写代码，必须 `delegate_task`

**对 Diva 的价值**：spawn 后如何管理子代理生命周期；Atlas 的"委派→验证→协调"协议是 Diva agent 层的最佳实践。

### 4.6 loops — 双 agent 对练

- 主 agent 做任务，另一个 agent review
- 通过 `bridge.jsonl` 直接互发消息
- 用 `doneSignal` 判断是否完成
- `maxIterations` 限制循环次数
- `worktree.ts` 用 git worktree 隔离

**对 Diva 的价值**：最简单的 actor-critic 循环原型，适合快速验证。

### 4.7 ralph-loop — 轻量 shell 循环

- 外部 bash 循环 + Docker sandbox
- 任务清单：`tasks.json`
- 结束信号：`<promise>COMPLETE/BLOCKED/DECIDE/TASK-ID:DONE</promise>`
- 人工干预：`STEERING.md`
- 历史：`HISTORY_DIR/ITERATION-${SESSION_ID}-${i}.txt`
- 每个 iteration 完全重启外部 agent 进程

**对 Diva 的价值**：任务清单持久化、单任务单轮、steering、标准化结束标签。子代理报告已写入 `agent-diva/docs/research/ralph-loop-analysis.md`。

### 4.8 openclaw — 多 channel gateway

- Gateway 集中 + channel 插件长连接
- 外部事件 → channel transport → spool → session → agent turn → outbound
- Cron 服务 + heartbeat + channel health monitor
- 插件 manifest-first
- Telegram 等高并发平台用 worker thread 做网络拉取，先写 spool 再进入主进程

**对 Diva 的价值**：如果 Diva 要支持 QQ/微信/Discord 多 channel，这是最直接的参考。

### 4.9 loopwright — 软件工厂

- Goal → reviewed plan → scheduled tasks → actor-critic → worktree → integration → publish
- 每个 task 独立 git worktree，成功后 commit 成 branch
- 最后合并、验证、推 PR
- SQLite store 持久化

**对 Diva 的价值**：完整的"从目标到 PR"流水线，适合 Diva 处理代码相关自主任务。

---

## 五、11 个来源对比矩阵

| 项目 | 触发 | 计划 | 执行 | 验证 | 状态 | 人机边界 | 类型 |
|---|---|---|---|---|---|---|---|
| 橙皮书 | 概念 | 概念 | 概念 | 概念 | 概念 | 概念 | 概念 |
| cobusgreyling | cron/事件 | LOOP.md | 外部 agent | 人工/自动 | STATE.md | L1/L2/L3 | 实践库 |
| Alife | 定时报点 | 无 | 单 AI 自由决定 | 无 | Memory 压缩 | 用户打断 | 愿景型 |
| atomic | 命令/事件 | Workflow stage | Subagent 并行 | Review gate | Artifact + checkpoint | human gate | 工程型 |
| openharness | Issue/手动/ohmo | Autopilot queue | Agent 12 轮 | Verification policy | Registry + journal | 默认 human gate | 工程型 |
| oh-my-opencode | 用户/Atlas | Boulder plan | delegate_task | Atlas 强制验证 | .sisyphus/ | orchestrator 协议 | 工程型 |
| loops | 用户启动 | PLAN.md | 双 agent | 对偶 review | run manifest | 人只启动 | 工程型 |
| ralph-loop | 用户启动 | PROMPT.md | 单 agent 迭代 | 标签检查 | tasks.json + LOG | BLOCKED/DECIDE | 工程型 |
| openclaw | channel 事件 | 无 | session.runAgentTurn | 无 | SQLite + spool | 实时交互 | Gateway |
| loopwright | 用户/CLI | 评审 plan | actor-critic | mechanical gate + integration | SQLite store | needs_human | 工程型 |
| opencode | ? | ? | ? | ? | ? | ? | 未读 |

---

## 六、对 Diva 的总体建议

### 6.1 分层架构

```
┌─────────────────────────────────────┐
│  Diva 愿景层（Alife 模式）            │
│  - 自主活动、自由循环、陪伴、探索       │
│  - 主动报点、记忆、情感、好奇心        │
├─────────────────────────────────────┤
│  Loop Engine 执行层（工程模式）       │
│  - trigger / goal / agent / verify   │
│  - memory / decide / governance      │
│  - 从 atomic、openharness、loopwright 借鉴 │
├─────────────────────────────────────┤
│  Channel / Gateway 接入层            │
│  - QQ、微信、Discord、Telegram 等     │
│  - 从 openclaw 借鉴                   │
├─────────────────────────────────────┤
│  Tool / Sandbox / Spawn 层           │
│  - 调用工具、子代理、沙箱执行           │
│  - 从 oh-my-opencode、ralph-loop 借鉴 │
└─────────────────────────────────────┘
```

### 6.2 两种循环模式

**自由循环（Free Loop）** — 默认态，对应 Alife 的 SystemEvent：
- 定时报点
- AI 自己决定下一步
- 可能产生新的目标

**目标循环（Goal Loop）** — 工作态，对应工程 Loop Engine：
- 用户/事件/AI 产生目标
- 进入 plan → execute → verify → complete
- 完成后回到自由循环

### 6.3 关键优先实现

1. **Loop Engine 原型**：`agent-diva-loop` crate，包含 `ActivityQueue`、`CronScheduler`、`SpawnExecutor`
2. **子代理输出契约**：标准结束信号（参考 ralph-loop `<promise>` 和 atomic `status.json`）
3. **Channel Health Monitor**：参考 openclaw，管理多 channel 账户生命周期
4. **文件化运行状态**：每个 activity 一个目录，`manifest.json` + `transcript.jsonl`
5. **L1/L2/L3 分级治理**：从 L1 开始上线，逐步升级到 L2/L3

---

## 七、相关文件

- 子代理报告：`agent-diva/docs/research/ralph-loop-analysis.md`
- 本报告：`agent-diva/docs/research/loop-engineering-references/2026-07-15-loop-engineering-survey.md`
- 架构草案：`agent-diva/docs/research/loop-engineering-references/2026-07-15-diva-loop-architecture-draft.md`
- 本地源码：`C:\Users\Administrator\Desktop\morediva\.workspace\loops\`

---

*报告字数：约 2400 字（中文）*
