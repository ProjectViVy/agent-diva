# 自主活动架构 —— 想法记录

> **状态**: 想法记录(informal notes),日后再聊
> **日期**: 2026-06-18
> **作者**: 大湿(对话中给出的设计)+ MiniMax-M3 整理 + 子代理调研
> **关联**:
> - [`../DECISION.md`](../../DECISION.md) §6 自主活动
> - [`../research/alife-plugin-module-di.md`](../research/alife-plugin-module-di.md)
> - [`../research/alife-function-skill-memory.md`](../research/alife-function-skill-memory.md)
> - [`../research/alife-proactive-selfupgrade.md`](../research/alife-proactive-selfupgrade.md)
> - [`../research/alife-user-presence-scan.md`](../research/alife-user-presence-scan.md)
> - [`../research/diva-alife-integration-plan.md`](../research/diva-alife-integration-plan.md)
> - `/Users/mastwet/Desktop/morediva/papers/` 下 7 篇论文

---

## 0. 核心结论(一句话)

**diva 自主活动 = 4 层架构 + 3 态用户存在模型**。Heartbeat 只是触发器,SOUL.md 是治理顶层,Rhythm 是行为选择(含用户存在感知),Harmless 是横切安全护栏。

---

## 1. 4 层架构(用户设计,2026-06-18 提出)

```
┌─────────────────────────────────────────────┐
│  SOUL.md  (Laputa 治理,顶层规则)             │   回答"什么是自主"
├─────────────────────────────────────────────┤
│  Harmless (cross-cutting 安全护栏)            │   回答"什么不能做"
├─────────────────────────────────────────────┤
│  Rhythm   (摸鱼/讲故事/自由活动)               │   回答"做不做/做哪种"
├─────────────────────────────────────────────┤
│  Heartbeat (timer + IsIdle 触发器)            │   回答"什么时候问"
└─────────────────────────────────────────────┘
```

### 各层职责与关键决策

| 层 | 职责 | 关键决策 | 状态 |
|----|------|---------|------|
| SOUL.md | 顶层规则,通过 Laputa 提案治理 | **缓存策略**:SOUL.md 变更 = session 重建(避免破坏 prompt cache) | 待定 |
| Harmless | 横切护栏:涉密/PII/超时/token 预算 | **不单设 crate**,挂在 core + tools + laputa 各处 | 待定 |
| Rhythm | 行为选择:Skip/TellStory/DoTask/RespondDirect | **默认有 Skip**(鼓励摸鱼) | 待定 |
| Heartbeat | 触发器:timer + IsIdle + 退避 | **与 Rhythm 解耦**,trigger 只管"该不该问" | 待定 |

---

## 2. 3 态用户存在模型(用户设计,2026-06-18 提出)

alife 单一提示词太粗,用户提出**两态 → 三态**精细化:

| 状态 | 触发条件 | Heartbeat 行为 | 提示词风格 |
|------|---------|---------------|----------|
| **Active** | 用户 [T_active] 分钟内说过话 | 不触发 | (无) |
| **Distracted** | 沉默 [T_active, T_gone) | 慢频触发(10 min 一次,无退避) | "用户似乎开小差了,你看看现状作出决策" |
| **Gone** | 沉默 ≥ T_gone | 快频触发(90s + 3^n 退避,封顶 4) | "用户已离开 [N] 分钟,你想做什么做什么" |

### 阈值默认值(建议,待拍板)

| 配置项 | 默认 | 备注 |
|--------|------|------|
| `presence.active_threshold` | 5 min | 5 min 没说话 = 开小差 |
| `presence.gone_threshold` | 30 min | 30 min = 大概率真离开 |
| `heartbeat.distracted_interval` | 10 min | 慢频,无退避 |
| `heartbeat.gone_base_interval` | 90 s | alife 默认 |
| `heartbeat.gone_backoff` | ×3^n, 封顶 4 | alife 默认 |
| `heartbeat.token_budget_per_day` | 100k | 防意外耗光 |

### 关键设计原则

- **状态机在基建层,不在 prompt 里** —— AI 只看到合适的提示词,不知道"我现在是哪个状态"
- **Distracted 默认跳过** —— "看看现状作出决策"允许 AI 决定**啥也不做**(主动不发言)
- **Gone 才有"自由活动许可"** —— Distracted 不给"自由"二字,避免轻量打扰

---

## 3. Heartbeat 与 alife 对位(子代理调研)

alife 的 SystemEventService.cs 设计:

```
[1] Tick        — .NET PeriodicTimer 1s
[2] Check busy  — functionService.IsIdle (XmlFunctionCaller 忙不忙)
[3] Inject      — functionService.Poke() → 进 ConcurrentQueue,2s 后 flush
[4] Wake        — LLM 看到 UpdatePrompt 消息
[5] Act         — AI 调工具 / 发消息 / 沉默
[6] Backoff     — 沉默 → 间隔 ×3^n,封顶 4;说话 → 重置回 90s
```

**alife 与 diva 关键差异**:

| 维度 | alife | diva 应该 |
|------|-------|----------|
| 触发频率 | 单一 90s + 退避 | 两态:Distracted 10 min / Gone 90s + 退避 |
| 用户存在 | **没检测**(见 SA4 报告) | 必须新建 presence 层 |
| UpdatePrompt 风格 | 单条统一 | Distracted / Gone 两个不同提示词 |
| EWake/EWait | 有(AI 自我调度) | 借鉴(SA3 §A) |
| 死亡循环风险 | WPF 主线程 + 信号量(作者注释承认) | Rust + tokio 重新设计,Send/Sync 边界 |
| Poke 队列 | ConcurrentQueue + 2s flush | 类似实现,但避免 alife 的死锁 |

**alife 没有 user presence 检测**: 全仓 grep 13 关键词零命中,WPF 原生 Window.Activated 未订阅,GetLastInputInfo 未引用。**diva 必须自己造**。

---

## 4. Park 2023 对比(论文调研)

Park 2023 Generative Agents 25 个 agent 在 Smallville 自动做事,实现机制:

- **不是定时器,是"沙盒虚拟时钟 + 预生成 plan 流"**
- LLM 一次性生成"日计划"(5-8 块 → 1 小时块 → 5-15 分钟块)
- 每个 tick:agent 感知 + 检索 + 决定"继续 plan OR 反应"
- **没有 wall-clock,没有"用户在场"概念**

| 维度 | Park 2023 | alife | diva (本设计) |
|------|-----------|-------|--------------|
| 时间模型 | 沙盒虚拟时钟 | 墙钟 | 墙钟 |
| Plan 来源 | 预生成 day plan | 无 plan,每次现场决定 | 借鉴:可选 week plan baseline |
| 触发 | 每 tick 全部思考 | timer + IsIdle | 两态 timer |
| 行为选择 | 继续 vs 反应 | 自由 | 三态 + Rhythm 层 |
| 多 agent | 25 共享世界 | 1 | 1(暂) |
| 时间一致性 | ✅(plan 框定) | ❌ | 折中:plan baseline + 自由补 |

**借鉴 Park 的可能性**:diva 可以有"周计划"baseline(低频生成,laputa 持久化),触发时优先遵循 plan,plan 之外才自由发挥。**避免每次触发 AI 都"重新做人"**。

---

## 5. 关键决策点(待你拍板)

### 决策 1:不跑 agent-loop 的 Rhythm 行为(用户已说"不用刻意设计")

> "不跑agent-loop这个不用刻意设计"

理解:Rhythm 行为可以是简化的轻量 LLM 调用,**不强制走 agent loop**。但具体实现是单次 LLM 直接出文本,还是预定义几个"轻量路径"(TellStory / RespondDirect)?

### 决策 2:用户 idle 时间窗默认值

| 选项 | 含义 |
|------|------|
| A. 5 min / 30 min(我建议) | 中等保守 |
| B. 1 min / 10 min(激进) | 更早进入自主模式 |
| C. 15 min / 2 h(保守) | 更晚打扰 |

### 决策 3:SOUL.md 缓存策略(用户已选 C)

> 我建议走 C:SOUL.md 变更 = session 重建(老对话归档,新对话用新 SOUL)
> 符合 Laputa "重大变更要走提案"语义,缓存零影响

### 决策 4:Distracted 提示词文案(已给定)

> "用户似乎开小差了,你看看现状作出决策"

需要定:要不要给 AI 提供"检查 checklist"(看什么、不看什么)?

### 决策 5:本波优先级

之前提的整合报告里 W1-W8 计划,**现按你的 4 层重排**:

| 周 | 任务 | 4 层对应 |
|----|------|---------|
| W1 | Heartbeat 触发器(timer + IsIdle) + Presence 层 | 底 2 层 |
| W2 | SOUL.md 接入(Laputa 提案 + cache 策略) | 顶层 |
| W3 | Rhythm 层(3 态机 + 提示词 + 摸鱼选项) | 第 3 层 |
| W4 | Harmless guard(PII/timeout/token 预算/审计) | 横切 |

### 决策 6:跨 session 意图持久化(未决)

如果 AI 决定"我今天下午想画画",这个意图:
- 存 laputa?autodream?
- 下次 session 启动时读?
- 影响下次 Rhythm 决策?

**未解决,先不做**,放到 v2 设计。

---

## 6. 实现要点速查(给未来开工用)

### 文件改动清单(预估)

```
agent-diva-pro/
├── agent-diva-core/
│   └── src/
│       ├── presence/
│       │   └── mod.rs              (新) UserPresenceTracker
│       ├── heartbeat/
│       │   ├── state_machine.rs    (新) UserPresenceState + classify()
│       │   ├── service.rs          (改) trigger 前调 classify()
│       │   └── types.rs            (改) 加 active_threshold / gone_threshold
│       └── prompts/
│           ├── distracted.rs       (新) Distracted 状态提示词
│           └── gone.rs             (新) Gone 状态提示词
├── agent-diva-tools/
│   └── src/
│       ├── wait_tools.rs           (新) e_wait + e_wake(借鉴 alife)
│       └── timeout_guard.rs        (新) 每个 Tool 的超时包装
├── agent-diva-agent/
│   └── src/
│       ├── rhythm.rs               (新) Rhythm 层:Skip/TellStory/DoTask/RespondDirect
│       └── content_filter.rs       (新) PII/涉密 检测(可配 regex)
├── agent-diva-laputa/
│   └── src/
│       └── heartbeat_hook.rs       (新) 自主行为写 changelog
└── agent-diva-gui/
    └── src-tauri/src/
        └── presence_settings.rs    (新) idle 阈值配置 UI
```

### 与 Authority Spine 边界

| 项 | 走 Spine? |
|----|---------|
| heartbeat 配置阈值(presence.active_threshold 等) | **走**(用户可 review) |
| SOUL.md 变更 | **走**(Laputa 提案核心) |
| Rhythm 行为选择 | **不走**(纯运行时行为) |
| Harmless 规则集(PII 列表等) | **走**(用户能加白/黑名单) |

### 与现有 diva-pro 模块的接口

- `agent-diva-core/heartbeat/` —— 现有 766 LOC heartbeat,**重写**(加 state machine + presence check)
- `agent-diva-core/cron/` —— 现有 1275 LOC cron,**保留**(常规任务调度,自主活动不混用)
- `agent-diva-agent/planning/` —— 现有 5200 LOC planning,**保留**(DoTask 走这个,Skip/TellStory 不走)
- `agent-diva-autodream/` —— 现有 rhythm.rs,**独立**(AutoDream 反思不混用)
- `agent-diva-laputa/` —— 加 presence + SOUL.md 接入

---

## 7. 不做的事(显式排除)

| 不做 | 理由 |
|------|------|
| XmlFunctionCaller 移植 | diva 走 OpenAI 兼容 tool calling 是对的,只取"显隐分桶"思想 |
| 多级 cache 记忆压缩 | Laputa 已有提案/合并,alife `Filter` 会绕过 Laputa |
| 多开/赛博世界 | DECISION.md §9 已 defer,单一人格哲学 |
| Roslyn 式运行时编译 | Rust 无 JIT 等价物 |
| WPF Window.Focus / GetLastInputInfo 直接移植 | 跨平台不友好,diva 走 tokio,初版用纯时间窗 |

---

## 8. 参考文献 / 代码

### 论文

- `/Users/mastwet/Desktop/morediva/papers/2304_03442.md` —— Park 2023 Generative Agents
- `/Users/mastwet/Desktop/morediva/papers/2305_02750.md` —— Deng 2023 Proactive Survey
- `/Users/mastwet/Desktop/morediva/papers/2410_12361.md` —— Lu 2024 Proactive Agent (ICLR 2025)
- `/Users/mastwet/Desktop/morediva/papers/2303_11366.md` —— Shinn 2023 Reflexion
- `/Users/mastwet/Desktop/morediva/papers/2412_14352.md` —— Dong 2024 Self-Improvement Survey
- `/Users/mastwet/Desktop/morediva/papers/2210_03629.md` —— Yao 2022 ReAct
- `/Users/mastwet/Desktop/morediva/papers/2212_08073.md` —— Bai 2022 Constitutional AI

### alife 代码关键路径

- `/Users/mastwet/Desktop/morediva/.workspace/alife/sources/Alife.Function/Alife.Function.SystemEvent/SystemEventService.cs` —— 自主报点核心
- `/Users/mastwet/Desktop/morediva/.workspace/alife/sources/Alife/Alife.Framework/Models/ChatBot.cs` —— Poke 队列 + ChatStreaming
- `/Users/mastwet/Desktop/morediva/.workspace/alife/sources/Alife/Alife.Framework/Models/Module/InteractiveModule.cs` —— Module 基类

### diva-pro 现状

- `agent-diva-pro/agent-diva-core/src/heartbeat/` —— 766 LOC,两阶段 Decide→Execute
- `agent-diva-pro/agent-diva-agent/src/planning/` —— 5200 LOC,5 层
- `agent-diva-pro/agent-diva-core/src/cron/` —— 1275 LOC
- `agent-diva-pro/agent-diva-autodream/src/rhythm.rs` —— AutoDream 反思
- `agent-diva-pro/agent-diva-laputa/` —— 权威存储
- `agent-diva-pro/agent-diva-gui/` —— Tauri v2

---

## 9. 下次接着聊的入口

日后再聊时,从这些点开始:

1. **决策点 2-4 的拍板**(idle 阈值 / Distracted 提示词细节 / 本波优先级确认)
2. **SOUL.md 怎么从无到有起一个草案**(Laputa 治理下的 SOUL.md 应包含哪些 section?)
3. **Harmless guard 的最小可用集**(v1 该有哪几条?)
4. **Presence 层的 Rust 设计骨架**(要不要起个 spike?)
5. **Park "周计划 baseline"借鉴**(要不要做 v2 增强?)
6. **要不要拉 alife 的 `开发规范/` 看作者设计灵感来源**
