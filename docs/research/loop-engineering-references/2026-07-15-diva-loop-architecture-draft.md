# Diva Loop Engine 架构草案（2026-07-15）

> 版本：v0.1 草案
> 目标：为 agent-diva 下一阶段的自主活动（Autonomous Activity）提供顶层架构设计
> 指引：以 Loop Engineering 橙皮书（alchaincyf/loop-engineering-orange-book）为概念基础，结合本次调研的 atomic、openharness、oh-my-opencode、loops、ralph-loop、openclaw、loopwright、cobusgreyling、Alife 等工程实现参考

---

## 一、设计哲学

### 1.1 Diva 不是工程 Loop Engine，而是有 Loop Engine 底座的 AI 伴侣

工程 Loop Engine（如 atomic、loopwright、openharness）是为了解决软件开发中的具体问题：修复 bug、review PR、跑 CI、合并代码。它们假设目标来自外部输入，验证标准是客观的测试/类型检查/人工审批。

Diva 的愿景更接近 Alife：一个长期存在的 AI 伴侣，能够主动感知、提议、行动、学习。Diva 的"目标"很多时候不是外部给的，而是 AI 自己从记忆、好奇心、用户状态、环境中产生的。

因此，**Diva 的 Loop Engine 必须支持两类循环**：

- **自由循环（Free Loop）**：没有外部目标的默认状态，AI 自己决定做什么。参考 Alife 的 SystemEvent。
- **目标循环（Goal Loop）**：有明确目标，需要计划、执行、验证、完成。参考 atomic、loopwright、openharness。

### 1.2 橙皮书六构件的 Diva 解释

| 橙皮书构件 | Diva 含义 | 关键参考 |
|---|---|---|
| Automation | 触发机制：用户输入、cron、heartbeat、channel 事件、AI 自触发 | Alife SystemEvent、openclaw gateway、openharness autopilot |
| Worktree | 每次 activity 的隔离执行空间 | loopwright worktree、loops worktree、atomic subagent worktree |
| Skills | 可动态加载的领域能力包 | Alife Skill、oh-my-opencode skill 检查 |
| Connectors | 外部工具、MCP、API、channel 连接 | openclaw channel plugin、oh-my-opencode connectors |
| Sub-agents | 子代理分工：规划者、实现者、验证者 | oh-my-opencode Atlas、atomic subagent、loops 双 agent |
| Memory | 跨轮次记忆与状态：运行状态、事实记忆、情感关系 | Alife Memory、Laputa、Mentle |

### 1.3 四个成本陷阱的应对

| 陷阱 | 应对策略 |
|---|---|
| Verification debt | 每个 activity 必须有 verifier，输出结构化信号 |
| Comprehension rot | 记忆压缩、context budget、阶段性总结 |
| Token blowout | Loop budget、max iterations、cooldown、kill switch |
| Cognitive surrender | L1/L2/L3 分级，高风险必须 human gate |

---

## 二、分层架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Diva 愿景层                             │
│  自主活动、陪伴、探索、好奇心、情感关系、自我成长               │
│  参考：Alife                                                   │
├─────────────────────────────────────────────────────────────┤
│                    Diva Loop Engine                            │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐  │
│  │ Trigger   │  │ Goal      │  │ Agent     │  │ Verify    │  │
│  │ Scheduler │  │ Generator │  │ Executor  │  │ Gate      │  │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘  │
│  ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌───────────┐  │
│  │ Memory    │  │ Decision  │  │ Governance│  │ Lifecycle │  │
│  │ Manager   │  │ Engine    │  │ Engine    │  │ Manager   │  │
│  └───────────┘  └───────────┘  └───────────┘  └───────────┘  │
│  参考：atomic, openharness, loopwright, cobusgreyling         │
├─────────────────────────────────────────────────────────────┤
│                    Channel / Gateway 层                       │
│  QQ、微信、Discord、Telegram、Slack 等                         │
│  参考：openclaw                                                │
├─────────────────────────────────────────────────────────────┤
│                 Tool / Sandbox / Spawn 层                     │
│  工具调用、子代理 spawn、沙箱执行、文件系统隔离               │
│  参考：oh-my-opencode, ralph-loop, loops                      │
└─────────────────────────────────────────────────────────────┘
```

### 2.1 新增 crate 建议

建议新增 **`agent-diva-loop`**，作为 Loop Engine 的核心 crate。其他 crate 的改动：

| 现有 crate | 与 Loop Engine 的关系 |
|---|---|
| `agent-diva-core` | 提供 cron/heartbeat 基础，Loop Engine 在此基础上做高级调度 |
| `agent-diva-tools` | 提供工具调用原语，Loop Engine 通过 Executor 调用 |
| `agent-diva-sandbox` | 提供隔离执行环境，Loop Engine 的每个 activity 可绑定 sandbox |
| `agent-diva-laputa` | 提供长期记忆存储，Loop Engine 的 Memory Manager 调用 |
| `agent-diva-channels` | 提供 channel 接入，Loop Engine 通过 Trigger Scheduler 接收事件 |
| `agent-diva-agent` | 提供 agent 执行能力，Loop Engine 的 Agent Executor 调用 |
| `agent-diva-skills` | 提供 skill 管理，Loop Engine 在启动 activity 前加载 skill |
| `agent-diva-mentle` | 提供治理/审计，Loop Engine 的 Governance Engine 调用 |

---

## 三、核心状态机

### 3.1 Activity 状态机

一个 Activity 是 Loop Engine 调度的基本单元。

```
          ┌──────────┐
          │  Idle    │
          └────┬─────┘
               │ trigger
               ▼
          ┌──────────┐
          │ Planning │
          └────┬─────┘
               │ plan ready
               ▼
          ┌──────────┐
          │ Executing│
          └────┬─────┘
               │ result
               ▼
          ┌──────────┐     ┌──────────┐
          │ Verifying│────▶│ Blocked  │
          └────┬─────┘     └──────────┘
               │ pass
               ▼
          ┌──────────┐
          │ Completed│
          └────┬─────┘
               │
               ▼
          ┌──────────┐
          │  Archived│
          └──────────┘
```

状态说明：

- **Idle**：活动未启动，等待触发
- **Planning**：生成或加载计划
- **Executing**：执行计划中的任务
- **Verifying**：验证执行结果
- **Blocked**：需要人工干预或外部信息
- **Completed**：已完成并通过验证
- **Archived**：归档，进入记忆

### 3.2 自由循环 vs 目标循环

```
自由循环（Free Loop）
  │
  ├─ 定时报点（Alife SystemEvent）
  ├─ AI 检查记忆、环境、用户状态
  ├─ AI 决定：继续等待 / 生成 Goal / 主动通知用户
  │
  ▼ 生成 Goal
目标循环（Goal Loop）
  │
  ├─ Planning：生成 plan
  ├─ Executing：spawn subagent / 调用工具
  ├─ Verifying：检查完成度和正确性
  ├─ Decision：继续 / 完成 / 阻塞 / 升级
  │
  ▼ 完成或归档
回到自由循环
```

---

## 四、核心组件设计

### 4.1 Trigger Scheduler（触发调度器）

职责：决定何时唤醒 Loop Engine。

来源：
- 用户输入（来自 channel）
- Cron 定时任务
- Heartbeat（自检查）
- Channel 事件（新消息、@提及）
- 系统事件（文件变更、CI 状态）
- AI 自触发（自由循环产生）

参考：Alife SystemEvent、openclaw CronService、openharness autopilot queue。

```rust
pub enum TriggerSource {
    User(ChannelId, MessageId),
    Cron(String),           // job_id
    Heartbeat,
    ChannelEvent(ChannelEvent),
    SystemEvent(SystemEvent),
    SelfGenerated,          // from Free Loop
}

pub struct Trigger {
    pub id: String,
    pub source: TriggerSource,
    pub priority: i32,
    pub timestamp: DateTime<Utc>,
    pub context: serde_json::Value,
}
```

### 4.2 Goal Generator（目标生成器）

职责：把 trigger 转化为可执行的 goal。

两种模式：
- **外部目标**：用户直接给出目标
- **内部目标**：自由循环中 AI 自己生成目标

参考：Alife 的"自由活动"、openharness 的 `RepoTaskCard`。

```rust
pub struct Goal {
    pub id: String,
    pub trigger_id: String,
    pub title: String,
    pub description: String,
    pub priority: i32,
    pub source: GoalSource,
    pub constraints: Vec<Constraint>,
}

pub enum GoalSource {
    External,
    SelfGenerated(MemoryContext),
}
```

### 4.3 Planner（计划器）

职责：把 goal 分解为可执行的 plan。

plan 包含：
- 任务列表（tasks）
- 依赖关系
- 验证命令
- 工具限制
- 人机边界级别

参考：atomic `workflow()`、loopwright `runPlanReview`、oh-my-opencode `boulder.json`。

```rust
pub struct Plan {
    pub id: String,
    pub goal_id: String,
    pub tasks: Vec<Task>,
    pub dependencies: Vec<(String, String)>, // (task_id, depends_on_task_id)
    pub verify_commands: Vec<String>,
    pub tool_policy: ToolPolicy,
    pub autonomy_level: AutonomyLevel, // L1 / L2 / L3
}

pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub expected_output: String,
    pub verify_commands: Vec<String>,
    pub worktree: Option<PathBuf>,
}
```

### 4.4 Agent Executor（执行器）

职责：执行 plan 中的任务。

执行方式：
- 内部 agent turn
- 子代理 spawn
- 外部 CLI（如 Claude Code、Codex）
- 工具调用

参考：oh-my-opencode BackgroundManager、loops runner、atomic subagent。

```rust
pub enum ExecutorKind {
    InternalAgent,
    Subagent(SubagentConfig),
    ExternalCli(String, Vec<String>),
    ToolCall(ToolCall),
}

pub struct AgentExecutor {
    pub kind: ExecutorKind,
    pub sandbox: Option<SandboxConfig>,
    pub tool_policy: ToolPolicy,
}

impl AgentExecutor {
    pub async fn execute(&self, task: &Task, context: ExecutionContext) -> Result<TaskResult, ExecutorError>;
}
```

### 4.5 Verifier / Gate（验证器）

职责：判断任务是否完成、是否正确。

验证方式：
- 结构化信号（如 `<diva-verify status="pass">`）
- 测试/类型检查/lint 命令
- 对偶 agent review（如 loops）
- 多 reviewer + reducer（如 atomic）
- 人工审批

参考：atomic `ralph-review-gate`、loops `review.ts`、loopwright mechanical gate、ralph-loop `<promise>`。

```rust
pub enum VerificationResult {
    Pass,
    Fail(String),
    NeedsHuman(String),
    Continue, // not done yet, need more iterations
}

pub struct Verifier {
    pub kind: VerifierKind,
    pub max_attempts: u32,
}

pub enum VerifierKind {
    SignalParser(String),       // regex or tag parser
    Command(Vec<String>),       // run tests/lints
    PeerReviewAgent(String),    // another agent reviews
    MultiReviewer { workers: Vec<String>, reducer: String },
    HumanGate,
}
```

### 4.6 Memory Manager（记忆管理器）

职责：保存和查询 Activity 的运行状态、事实记忆、用户关系等。

分层：
- **运行状态（Runtime State）**：`manifest.json` + `transcript.jsonl`，类似 loops 的 RunManifest
- **事实记忆（Fact Memory）**：Alife Memory 式的结构化记忆，存在 Laputa
- **关系记忆（Relationship Memory）**：用户偏好、情感状态、历史交互
- **世界记忆（World Memory）**：探索记录、学习到的知识

参考：Alife Memory、loops `manifest.json`、Laputa。

```rust
pub struct MemoryManager {
    pub runtime_store: RuntimeStore,
    pub fact_store: LaputaStore,
    pub relationship_store: RelationshipStore,
}

pub struct ActivityState {
    pub activity_id: String,
    pub goal: Goal,
    pub plan: Plan,
    pub current_task: Option<String>,
    pub status: ActivityStatus,
    pub events: Vec<ActivityEvent>,
}
```

### 4.7 Decision Engine（决策引擎）

职责：根据验证结果、记忆、约束，决定下一步。

决策：
- 继续执行下一个 task
- 重新执行当前 task
- 重新规划
- 阻塞，等待人工输入
- 升级（从 L1 到 L2/L3，或反之）
- 完成并归档

参考：atomic `reducer`、openharness `verification policy`、cobusgreyling `L1/L2/L3`。

```rust
pub enum Decision {
    Continue,
    Retry { task_id: String },
    Replan,
    Block(String),
    Escalate(AutonomyLevel),
    Complete,
    Abort(String),
}

pub struct DecisionEngine {
    pub governance: GovernanceEngine,
    pub memory: MemoryManager,
}
```

### 4.8 Governance Engine（治理引擎）

职责：安全边界、权限控制、审计。

能力：
- 工具限制（allowlist/denylist）
- 文件系统访问控制
- 网络访问控制
- 成本预算（token budget、iteration budget）
- 审批策略（L1/L2/L3）
- 审计日志

参考：atomic `permission-gate`、oh-my-opencode Atlas、cobusgreyling `loop-constraints.md`。

```rust
pub struct GovernanceEngine {
    pub tool_policy: ToolPolicy,
    pub file_policy: FilePolicy,
    pub budget_policy: BudgetPolicy,
    pub approval_policy: ApprovalPolicy,
}

pub struct BudgetPolicy {
    pub max_iterations: u32,
    pub max_tokens: u64,
    pub max_duration: Duration,
}
```

---

## 五、Channel / Gateway 设计

参考 openclaw，但不需要做那么重。Diva 的 Channel 层需要：

- 统一账户模型：每个 channel 一个账户
- 生命周期管理：start/stop/health/restart
- 事件 spool：高并发 channel 先持久化再处理
- 输出回写：把 agent 回复发送回 channel

```rust
pub trait ChannelAdapter {
    fn id(&self) -> ChannelId;
    async fn start(&self, account: AccountConfig) -> Result<(), ChannelError>;
    async fn stop(&self, account: AccountConfig) -> Result<(), ChannelError>;
    async fn send(&self, account: AccountId, message: OutboundMessage) -> Result<(), ChannelError>;
    async fn health(&self, account: AccountId) -> HealthStatus;
}
```

---

## 六、文件约定（参考 cobusgreyling）

建议在每个 workspace 下创建 `.diva/` 目录：

```
.diva/
├── DIVA_LOOP.md          # 声明本仓库的 loops
├── DIVA_STATE.md         # 当前状态
├── DIVA_CONSTRAINTS.md   # 安全约束
├── DIVA_RUN_LOG.md       # 运行日志
├── DIVA_BUDGET.md        # 预算
├── activities/
│   └── {activity_id}/
│       ├── manifest.json
│       ├── plan.md
│       ├── transcript.jsonl
│       └── result.md
└── steering.md           # 人工干预
```

### 6.1 DIVA_LOOP.md 示例

```markdown
# Diva Loops

## L1
- **Daily Triage**: 每天早上 9 点扫描仓库，生成问题报告
- **Memory Cleanup**: 每周清理过期短期记忆

## L2
- **Test Repair**: 当 CI 失败时，自动尝试修复测试并提交 PR（需人审批）

## L3
- **Dependency Update**: 每周自动更新依赖并合并（仅限 patch 版本）
```

### 6.2 DIVA_CONSTRAINTS.md 示例

```markdown
- 不直接推送到 main
- 不编辑 secrets/凭证
- 每次 loop 最多 10 轮
- L2 以上必须有人审批才能修改生产配置
- 单轮 token 预算 100k
```

---

## 七、治理模型：L1/L2/L3

| 级别 | 描述 | 示例 | 审批 |
|---|---|---|---|
| **L1** | 观察、报告、提议 | daily triage、状态报告、提醒 | 无需审批 |
| **L2** | 执行低风险修改，但需人审批 | 修复测试、生成文档、refactor | 自动执行，但结果需人确认 |
| **L3** | 完全自主执行 | 受信任的 cleanup、依赖更新 | 事后审计 |

升级规则：
- L1 → L2：需要连续 N 次成功执行并验证
- L2 → L3：需要人工授权 + 完整审计记录
- L3 → L2/L1：一旦失败或用户投诉，自动降级

---

## 八、与现有 Diva 模块的映射

| Diva 模块 | 在 Loop Engine 中的角色 | 参考来源 |
|---|---|---|
| `agent-diva-core` | Trigger Scheduler、Heartbeat | Alife SystemEvent、openharness |
| `agent-diva-tools` | Agent Executor 的工具调用 | atomic、oh-my-opencode |
| `agent-diva-sandbox` | Worktree / Sandbox 隔离 | loopwright、loops、atomic |
| `agent-diva-laputa` | Memory Manager 的事实记忆 | Alife Memory、loops manifest |
| `agent-diva-channels` | Channel / Gateway 接入 | openclaw |
| `agent-diva-agent` | Agent Executor、Subagent 管理 | oh-my-opencode Atlas、atomic subagent |
| `agent-diva-skills` | Skills 动态加载 | Alife Skill、oh-my-opencode skills |
| `agent-diva-mentle` | Governance、审计 | cobusgreyling constraints、atomic permission-gate |
| `agent-diva-loop`（新） | 统一 Loop Engine | 本草案 |

---

## 九、最小实现路径（MVP）

### Phase 1：自由循环（2-3 周）

- 实现 `agent-diva-loop` 的骨架：Trigger Scheduler + Goal Generator
- 实现 Alife 式的 SystemEvent：定时报点，让 AI 决定下一步
- 集成到现有 `agent-diva-core` 的 cron/heartbeat
- 输出：Diva 能周期性地主动发消息/提议

### Phase 2：目标循环（2-3 周）

- 实现 Planner + Agent Executor + Verifier
- 支持简单的 Goal Loop：用户给目标 → 计划 → 执行 → 验证 → 完成
- 参考 loops 的双 agent review 或 ralph-loop 的 signal
- 输出：Diva 能处理"帮我修这个 bug"并循环到完成

### Phase 3：治理与状态（2-3 周）

- 实现 Governance Engine：L1/L2/L3、工具限制、预算
- 实现文件化状态：`.diva/activities/{id}/manifest.json` + `transcript.jsonl`
- 实现 Memory Manager 与 Laputa 的集成
- 输出：Diva 的自主活动可审计、可恢复、可治理

### Phase 4：Channel 与多账户（2-3 周）

- 参考 openclaw，实现 Channel Health Monitor
- 支持 QQ/微信/Discord 等 channel 的事件接入和回复回写
- 输出：Diva 能在多个平台上主动活动和被动响应

---

## 十、关键设计原则

1. **Loop 是默认状态，不是特殊模式**。Diva 启动后应该一直在某个循环中，要么自由循环，要么目标循环。
2. **自由循环产生目标，目标循环产生结果**。两者不是并列关系，而是主从关系。
3. **每个 Activity 必须可审计**。状态文件化、事件 transcript、决策记录。
4. **治理必须可降级**。一旦 L3 出错，必须能自动降级到 L2/L1。
5. **人不应该是阻塞点**。L1/L2 应该尽量自动，只在 L3 或异常时请求人。
6. **记忆是核心**。没有跨轮次记忆，循环就是无状态脚本。
7. **先 L1，再 L2，最后 L3**。不要一上来就做完全自主。

---

## 十一、相关文档

- 调研报告：`agent-diva/docs/research/loop-engineering-references/2026-07-15-loop-engineering-survey.md`
- 本草案：`agent-diva/docs/research/loop-engineering-references/2026-07-15-diva-loop-architecture-draft.md`
- ralph-loop 分析：`agent-diva/docs/research/ralph-loop-analysis.md`
- oh-my-opencode 与 loops 分析：`agent-diva/docs/research/loop-engineering-references/omo-loops-loop-engineering-report.md`
- Alife 调研：`agent-diva/docs/research/alife-vs-diva-autonomous-flow.md`
- 本地源码：`C:\Users\Administrator\Desktop\morediva\.workspace\loops\`

---

*草案版本：v0.1*
*待讨论：Phase 1 的具体接口、Free Loop 与 Goal Loop 的切换阈值、Laputa 与 Memory Manager 的边界*
