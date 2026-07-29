# agent-diva-loop 设计草案 v2：愿景与工程兼容的最小化 Loop Engine

> 版本：v2.0
> 日期：2026-07-15
> 目标：为 agent-diva 设计一个专门承载"自主活动"的 crate
> 核心哲学：愿景（自由循环）运行在工程化 Loop Engine（目标循环）的基础之上，两者在同一个包内兼容、增量演进

---

## 一、为什么需要 `agent-diva-loop`

Diva 的下一阶段要走向长期自主运行的 AI 伴侣。这个方向上有两类参考：

- **愿景型**：Alife，强调 AI 主动存在、探索世界、陪伴用户
- **工程型**：atomic、loopwright、openharness、oh-my-opencode，强调目标驱动的可验证循环

我们的判断是：**Diva 不是工程 Loop Engine，但要实现 Alife 的愿景，必须有自己的工程化 Loop Engine 作为底座**。否则自由循环会退化成无状态、不可验证、不可审计的随机 poke。

因此新增 `agent-diva-loop`，它同时提供：

- **Free Mode**：让 Diva 在没有外部目标时主动感知、提议、产生目标
- **Loop Engineering Mode**：让 Diva 在有目标时可靠地计划、执行、验证、完成

两种模式不是两套系统，而是共享同一个底层抽象：`Activity`。

---

## 二、核心哲学

### 2.1 愿景运行在工程之上

```
┌─────────────────────────────────────┐
│         Diva 愿景层                  │
│   自由循环、主动存在、陪伴、探索        │
│   = Free Mode                         │
├─────────────────────────────────────┤
│         Diva 工程层                    │
│   目标循环、计划、执行、验证、决策      │
│   = Loop Engineering Mode             │
├─────────────────────────────────────┤
│      共享基础设施                      │
│   Activity 状态机、调度、记忆、治理    │
└─────────────────────────────────────┘
```

Free Mode 是默认状态。当它决定要做一件具体的事时，就生成一个 `Goal`，进入 Loop Engineering Mode。Engineering Mode 完成后，回到 Free Mode。

### 2.2 最小化优先

MVP 阶段不追求完整覆盖所有 Loop Engineering 构件，只做：

1. 一个能持续运行的 `LoopEngine`
2. 一个可文件化恢复的 `Activity`
3. Free Mode 能定时 poke 并产生决定
4. Engineering Mode 能处理简单目标并循环
5. 一个最简的 L1/L2 治理

其他能力（worktree、multi-channel gateway、复杂 reviewer、CI 集成）后续增量添加。

### 2.3 增量演进

`agent-diva-loop` 内部接口是 trait-based 的，每个组件都可替换：

- Planner 可以从简单 prompt 开始，后面换成结构化规划器
- Executor 可以从内部 agent turn 开始，后面加子代理、外部 CLI
- Verifier 可以从 XML 信号开始，后面加测试、对偶 agent、human gate

---

## 三、包结构

```
agent-diva-loop/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── engine.rs          # LoopEngine 主入口与生命周期
    ├── activity.rs        # Activity 统一状态机
    ├── scheduler.rs       # Cron / Heartbeat / 事件触发
    ├── free_mode.rs       # 自由循环：Poke、感知、产生目标
    ├── engineering_mode.rs # 工程循环：Plan → Execute → Verify → Decide
    ├── goal.rs            # Goal 抽象
    ├── planner.rs         # Planner trait 与简单实现
    ├── executor.rs        # Executor trait 与简单实现
    ├── verifier.rs        # Verifier trait 与简单实现
    ├── decision.rs        # Decision engine
    ├── governance.rs      # L1/L2/L3 治理
    ├── memory.rs          # Memory trait 与文件化实现
    └── persistence.rs      # manifest.json / transcript.jsonl 读写
```

---

## 四、统一抽象：Activity

Activity 是 Free Mode 和 Engineering Mode 的公共载体。无论 AI 是被 poke 唤醒，还是收到用户目标，都先创建为一个 Activity。

```rust
// src/activity.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityMode {
    Free,        // 自由循环：无外部目标，AI 自己决定
    Engineering, // 工程循环：有明确目标，循环执行
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivityStatus {
    Idle,
    Sensing,    // 自由模式：感知环境、读取记忆
    Planning,   // 生成计划
    Executing,  // 执行任务
    Verifying,  // 验证结果
    Blocked,    // 需要人工干预
    Done,       // 完成
    Archived,   // 归档
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub mode: ActivityMode,
    pub status: ActivityStatus,
    pub goal: Option<String>,
    pub plan: Option<String>,
    pub current_step: Option<String>,
    pub iterations: u32,
    pub max_iterations: u32,
    pub context: serde_json::Value,
    pub autonomy_level: AutonomyLevel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

---

## 五、LoopEngine

`LoopEngine` 是包的入口。它负责启动调度器、接收触发、创建 Activity、选择模式。

```rust
// src/engine.rs
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct LoopEngine {
    config: LoopConfig,
    state: Arc<RwLock<LoopState>>,
    scheduler: Arc<dyn Scheduler>,
    free_mode: Arc<dyn FreeMode>,
    engineering_mode: Arc<dyn EngineeringMode>,
    memory: Arc<dyn Memory>,
    governance: Arc<dyn Governance>,
}

impl LoopEngine {
    pub async fn new(config: LoopConfig) -> Result<Self, LoopError>;

    /// 启动引擎。默认进入 Free Mode，等待 poke 或外部触发。
    pub async fn start(&self) -> Result<(), LoopError>;

    /// 收到外部触发（用户消息、channel 事件、cron 等）
    pub async fn on_trigger(&self, trigger: Trigger) -> Result<String, LoopError>;

    /// 用户提交一个目标，进入 Engineering Mode
    pub async fn submit_goal(&self, goal: Goal) -> Result<String, LoopError>;

    /// 用户向某个活动提供输入（解除阻塞或授权）
    pub async fn human_input(&self, activity_id: &str, input: HumanInput) -> Result<(), LoopError>;

    /// 停止引擎，优雅关闭所有活动
    pub async fn shutdown(&self) -> Result<(), LoopError>;
}
```

---

## 六、Free Mode：愿景层

Free Mode 让 Diva 在没有外部目标时持续存在。它回答的问题是：**"现在没有用户找我，我该做什么？"**

### 6.1 Poke 机制

参考 Alife SystemEvent，但做最小化：

```rust
// src/free_mode.rs
use std::time::Duration;

pub struct PokeConfig {
    /// 基础间隔
    pub base_interval: Duration,
    /// 随机抖动
    pub jitter: Duration,
    /// 用户活动后是否重置
    pub reset_on_user_activity: bool,
}

impl Default for PokeConfig {
    fn default() -> Self {
        Self {
            base_interval: Duration::from_secs(90),
            jitter: Duration::from_secs(30),
            reset_on_user_activity: true,
        }
    }
}
```

### 6.2 FreeDecision

每次 poke 或用户空闲时，AI 做出决定：

```rust
#[derive(Debug, Clone)]
pub enum FreeDecision {
    /// 什么都不做，继续等待
    Wait,

    /// 主动给用户发一条消息/提议
    Propose(String),

    /// 生成一个工程目标，进入 Engineering Mode
    SpawnTask(Goal),

    /// 自由探索（如读新闻、学习文档、整理记忆）
    Explore(String),
}

#[async_trait]
pub trait FreeMode: Send + Sync {
    /// 在 Free Mode 下执行一次"感知+决定"循环
    async fn sense_and_decide(
        &self,
        activity: &mut Activity,
        memory: &dyn Memory,
    ) -> Result<FreeDecision, LoopError>;
}
```

### 6.3 典型流程

```
Free Mode Activity 创建
    │
    ├─ 读取记忆（最近对话、用户状态、未完成目标）
    ├─ 读取环境（屏幕、仓库状态、channel 事件）
    ├─ 决定：
    │   ├─ Wait → 安排下一次 poke
    │   ├─ Propose → 通过 channel 给用户发消息
    │   ├─ SpawnTask → 进入 Engineering Mode
    │   └─ Explore → 执行轻量探索，完成后回到 Wait
    │
    └─ 更新状态，记录 transcript
```

---

## 七、Loop Engineering Mode：工程层

Engineering Mode 让 Diva 可靠地执行具体目标。它回答的问题是：**"我有一个目标，怎么循环到完成？"**

### 7.1 五阶段循环

```
Planning → Executing → Verifying → Deciding → (Done / Blocked / Continue)
```

### 7.2 接口

```rust
// src/engineering_mode.rs
#[async_trait]
pub trait EngineeringMode: Send + Sync {
    /// 执行一轮工程循环
    async fn step(&self, activity: &mut Activity) -> Result<ActivityStatus, LoopError>;

    /// 判断活动是否已完成
    fn is_terminal(&self, status: &ActivityStatus) -> bool;
}
```

### 7.3 组件拆分

```rust
// src/planner.rs
#[async_trait]
pub trait Planner: Send + Sync {
    async fn plan(&self, goal: &str, context: &Context) -> Result<Plan, PlannerError>;
}

// src/executor.rs
#[async_trait]
pub trait Executor: Send + Sync {
    async fn execute(&self, step: &Step, context: &Context) -> Result<ExecutionResult, ExecutorError>;
}

// src/verifier.rs
#[derive(Debug, Clone, PartialEq)]
pub enum VerifyResult {
    Pass,
    Fail(String),
    NeedsHuman(String),
    Continue,
}

#[async_trait]
pub trait Verifier: Send + Sync {
    async fn verify(&self, step: &Step, result: &ExecutionResult) -> Result<VerifyResult, VerifierError>;
}

// src/decision.rs
#[derive(Debug, Clone)]
pub enum Decision {
    Continue,
    Retry(String),
    Replan,
    Block(String),
    Escalate(AutonomyLevel),
    Complete,
    Abort(String),
}

#[async_trait]
pub trait DecisionEngine: Send + Sync {
    async fn decide(
        &self,
        activity: &Activity,
        verify_result: &VerifyResult,
    ) -> Result<Decision, DecisionError>;
}
```

### 7.4 结束信号

MVP 采用简单 XML 标签，让执行 agent 在输出中显式声明状态：

```xml
<diva-done/>
<diva-blocked reason="..."/>
<diva-continue next="..."/>
```

后续可扩展为结构化 JSON 或 agent 协议（如 ACP）。

---

## 八、调度与触发

```rust
// src/scheduler.rs
#[derive(Debug, Clone)]
pub enum Trigger {
    Poke,                       // Free Mode 的定时报点
    Cron(String),               // cron job
    UserMessage(Message),       // 用户消息
    SystemEvent(SystemEvent),   // 系统事件
    GoalSubmitted(Goal),        // 用户或 Free Mode 提交目标
    HumanInput(String),         // 人工干预
}

#[async_trait]
pub trait Scheduler: Send + Sync {
    /// 获取下一个触发事件
    async fn next(&mut self) -> Trigger;

    /// 安排下一次 poke
    async fn schedule_poke(&mut self, at: DateTime<Utc>);
}
```

---

## 九、治理：L1 / L2 / L3

MVP 只实现 L1 和 L2，L3 作为占位。

```rust
// src/governance.rs
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum AutonomyLevel {
    L1, // 只读、报告、提议，不修改外部状态
    L2, // 低风险修改，需结果确认或审批
    L3, // 完全自主（MVP 阶段不启用）
}

pub struct Governance {
    pub level: AutonomyLevel,
    pub allowed_tools: Vec<String>,
    pub denied_tools: Vec<String>,
    pub max_iterations: u32,
    pub token_budget: u64,
}

impl Governance {
    pub fn can_use_tool(&self, tool: &str) -> Result<(), GovernanceError>;
    pub fn can_write_file(&self, path: &std::path::Path) -> Result<(), GovernanceError>;
}
```

升级规则：

- L1 是默认
- L2 需要用户明确授权或连续成功历史
- L3 在 MVP 阶段始终不启用

---

## 十、记忆与持久化

### 10.1 Memory trait

```rust
// src/memory.rs
#[async_trait]
pub trait Memory: Send + Sync {
    /// 为当前 activity 加载相关上下文
    async fn load_context(&self, activity: &Activity) -> Result<MemoryContext, MemoryError>;

    /// 记录活动事件
    async fn record_event(
        &self,
        activity_id: &str,
        event: ActivityEvent,
    ) -> Result<(), MemoryError>;

    /// 压缩活动历史，减少上下文长度
    async fn compress(&self, activity_id: &str) -> Result<(), MemoryError>;
}
```

### 10.2 文件化持久化

MVP 阶段用文件系统，不引入数据库。

```
.diva/
├── loops.json              # 活跃 loop 配置
├── state.json              # 当前 LoopEngine 状态
└── activities/
    └── {activity_id}/
        ├── manifest.json      # 活动元信息
        ├── plan.md            # 计划（Engineering Mode）
        └── transcript.jsonl   # 事件流
```

```rust
// src/persistence.rs
pub struct FilePersistence {
    root: std::path::PathBuf,
}

impl FilePersistence {
    pub fn save_manifest(&self, activity: &Activity) -> Result<(), PersistenceError>;
    pub fn append_transcript(&self, activity_id: &str, event: ActivityEvent) -> Result<(), PersistenceError>;
    pub fn load_activity(&self, activity_id: &str) -> Result<Activity, PersistenceError>;
}
```

---

## 十一、两种模式的交互

```
LoopEngine 启动
    │
    ▼
┌─────────────┐
│  Free Mode  │ ◀──────────────┐
│  (默认状态)  │                │
└──────┬──────┘                │
       │                        │
       ├─ 用户给目标 ──▶┌──────────────┐
       │                 │ Engineering  │
       ├─ AI 生成目标 ──▶│ Mode         │
       │                 └──────┬───────┘
       │                        │
       │                        ▼
       │                 ┌──────────────┐
       │                 │ Done / Blocked│
       │                 └──────┬───────┘
       │                        │
       └────────────────────────┘
              回到 Free Mode
```

统一入口是 `Activity`：

- Free Mode 创建 `mode = Free` 的 Activity
- Engineering Mode 创建 `mode = Engineering` 的 Activity
- 当 Free Mode 决定 `SpawnTask(Goal)` 时，创建新的 Engineering Activity
- 当 Engineering Activity 结束时，LoopEngine 回到 Free Mode 等待下一次 poke

---

## 十二、MVP 实现计划

### Phase 1：骨架 + Free Mode（1-2 周）

- 创建 `agent-diva-loop` crate
- 实现 `Activity`、`LoopEngine`、`FilePersistence`
- 实现 `PokeScheduler` 和 `FreeMode` trait
- 实现最简 `FreeMode`：每次 poke 随机决定 Wait / Propose
- 与 `agent-diva-core` cron 集成
- 输出：Diva 能周期性主动发消息

### Phase 2：Engineering Mode（1-2 周）

- 实现 `Planner`、`Executor`、`Verifier`、`DecisionEngine` trait
- 实现最简 `Planner`：把 goal 拆成 1-3 步
- 实现最简 `Executor`：调用内部 agent turn
- 实现最简 `Verifier`：解析 `<diva-done/>` / `<diva-blocked/>`
- 实现 `submit_goal` 入口
- 输出：Diva 能循环执行简单目标

### Phase 3：治理 + 记忆（1-2 周）

- 实现 `Governance` 的 L1/L2
- 实现 `Memory` 与 `agent-diva-laputa` 的集成
- 实现状态文件化恢复
- 输出：Diva 的循环可审计、可恢复、可治理

### Phase 4：与现有模块集成（1-2 周）

- 接入 `agent-diva-channels` 收发消息
- 接入 `agent-diva-tools` 的 spawn
- 接入 `agent-diva-sandbox` 做隔离
- 输出：Diva 能在多 channel 上主动活动和被动响应

---

## 十三、与现有 Diva 模块的关系

| 模块 | 提供的能力 | `agent-diva-loop` 如何使用 |
|---|---|---|
| `agent-diva-core` | cron、heartbeat、事件系统 | LoopEngine 启动和触发源 |
| `agent-diva-tools` | 工具调用、spawn 子代理 | Executor 执行具体步骤 |
| `agent-diva-sandbox` | 隔离执行环境 | Engineering Mode 可选绑定 sandbox |
| `agent-diva-laputa` | 长期记忆 | Memory 的事实记忆后端 |
| `agent-diva-channels` | channel 接入 | 接收用户输入、发送主动消息 |
| `agent-diva-agent` | agent 执行 | Executor 内部调用 |
| `agent-diva-mentle` | 治理/审计 | Governance 后端 |

`agent-diva-loop` 是这些能力的编排层，不替代它们。

---

## 十四、设计原则

1. **一个 Activity 走天下**：Free Mode 和 Engineering Mode 统一用 Activity 抽象
2. **愿景是默认态**：Engineering Mode 完成后必须回到 Free Mode
3. **文件化优先**：MVP 阶段用文件系统，降低依赖
4. **trait-based 增量**：每个组件都可替换，后面可以升级
5. **治理前置**：L1 是默认，任何写操作都需要授权
6. **结束信号显式**：用 `<diva-done/>` 等标签，不让 AI 自己决定何时完成
7. **可审计**：每个 Activity 都有 manifest 和 transcript

---

## 十五、后续可扩展方向

| 阶段 | 扩展内容 | 参考来源 |
|---|---|---|
| v0.2 | 多 reviewer 验证 | atomic `ralph-review-gate` |
| v0.3 | git worktree 隔离 | loopwright, loops, atomic |
| v0.4 | Channel Health Monitor | openclaw |
| v0.5 | CI 轮询与集成 | openharness |
| v0.6 | 多 agent 委派与并发 | oh-my-opencode Atlas |
| v0.7 | Loop 文件约定（LOOP.md / STATE.md） | cobusgreyling |
| v1.0 | ACP / A2A 协议接入 | 之前调研的 ACP/A2A 资料 |

---

## 十六、相关文档

- 调研总报告：`agent-diva/docs/research/loop-engineering-references/2026-07-15-loop-engineering-survey.md`
- 早期架构草案：`agent-diva/docs/research/loop-engineering-references/2026-07-15-diva-loop-architecture-draft.md`
- ralph-loop 分析：`agent-diva/docs/research/ralph-loop-analysis.md`
- oh-my-opencode + loops 分析：`agent-diva/docs/research/loop-engineering-references/omo-loops-loop-engineering-report.md`

---

*本草案为 v2.0，完全独立起草，与之前草案不重复，但基于同一批调研材料。*
