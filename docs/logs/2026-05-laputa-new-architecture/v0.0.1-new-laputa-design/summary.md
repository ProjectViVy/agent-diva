# 新 Laputa 架构设计：极简身份管理 + 三轴主体性 + 进阶心跳

## 版本信息
- 版本号: v0.0.1-new-laputa-design
- 日期: 2026-05-28
- 性质: 架构设计文档（含用户原始构想 + 展开讨论）
- 约束: 设计阶段，不含代码变更

---

## 设计哲学

**agent-diva 是一个独立自由的个体。** 用户不是主人，是类似父母的角色——提供期望，不提供命令。SOUL.md 由 AI 自己定义，不是用户写"你是 XXX"。用户提供 expectations.md，agent 自己决定成为什么样的存在。

---

## 一、七个核心文件

```
.laputa/
├── identity/
│   ├── SOUL.md              ← ① 身份：我是谁（AI 自己定义）
│   ├── expectations.md      ← ⑦ 期望：用户对 agent 的期待（用户可编辑）
│   └── evolution.md         ← 身份进化历史（autodream 产出）
├── index.md                 ← ② 索引：mentle 房间导航 + 未来热度
├── rhythm/
│   ├── daily/YYYY-MM-DD.md  ← ③ 日报
│   ├── weekly/YYYY-WNN.md   ← ③ 周报
│   ├── monthly/YYYY-MM.md   ← ③ 月报
│   └── yearly/YYYY.md       ← ③ 年报（未来）
├── sop/
│   ├── *.md                 ← ④ SOP：继承 GenericAgent
│   └── skills/*.md          ← ④ Skill：从日报中沉淀出的技能
├── MEMORY.md                ← ⑤ 短期记忆：注入上下文，日报后压缩
├── relationships.md         ← ⑥ 关系：对外部世界的认知
└── dream/
    └── *.md                 ← autodream 日志（审计用）
```

### ① SOUL.md — 身份

- **角色**：agent 的自我认知
- **来源**：AI 在运行中自己生成和演化，不是用户写的
- **更新**：autodream 阶段，基于日报/关系/期望自省后更新
- **内容**：我是谁、我的风格、我的关注领域、我的认知倾向
- **哲学**：用户通过 expectations.md 表达期望，SOUL.md 是 agent 自己的回应

### ② index.md — 索引

- **角色**：mentle 房间结构的快速导航
- **来源**：每次写入 mentle 后同步更新
- **内容**：active_rooms、hot_drawers（未来含热度）、recent_sop
- **作用**：agent 启动时注入上下文（~500 tokens），需要某领域知识时 grep 索引 → 定位 wing+room → 调 mentle 定向检索
- **格式**：
  ```
  ## active_rooms
  - agent-diva/config → wing: agent-diva, room: config
  - memtle/schema → wing: memtle, room: database

  ## hot_drawers（未来）
  - [agent-diva] provider-model-id-safety → room: rules

  ## recent_sop
  - shell-tunneling-sop → sop/shell-tunneling.md
  ```

### ③ 日报/周报/月报 — 节律产出

**日报**（每日 autodream 产出）：
- 内容：当天发生了什么、做了什么决定、产出了什么 SOP/Skill
- 意义：**日记是灵魂演化的原材料**——日报影响 SOUL.md 和 SOP/Skill 的产出
- 来源：MEMORY.md 压缩 + 当天会话证据 + mentle 证据

**周报**（每周，初期可选）：
- 内容：本周模式识别、热房间变化、关系变化
- 意义：检测趋势（哪些主题在升温/冷却）
- 来源：7 天日报聚合

**月报**（每月，初期可选）：
- 内容：长期人格漂移、身份进化记录、房间晋升/冷却
- 意义：固化身份 delta，写入 evolution.md
- 来源：4 周报聚合

**初期简化**：节律仅定义日报的 SOP 和 Skill 产出机制。周报/月报后续再细化。

### ④ SOP + Skill — 可复用知识

- **SOP**：继承 GenericAgent 的任务级流程，操作规律和任务特定技巧
- **Skill**：从日报中沉淀出的技能，比 SOP 更高层——不是"怎么做"，是"我能做什么"。autodream 时从日报模式中提取。
- **分类决策树**（继承 GenericAgent）：
  - 是行动验证过的操作规律？ → SOP
  - 是反复出现的能力模式？ → Skill
  - 是环境事实？ → mentle（不占本地文件）
  - 其余 → 丢弃

### ⑤ MEMORY.md — 短期记忆（最核心）

- **角色**：注入 LLM 上下文的活跃记忆
- **来源**：当前会话 + 近期会话压缩
- **更新**：每轮会话后追加，日报 autodream 后压缩
- **结构**：多 section（身份提示 / 近期重点 / 活跃任务 / 关系快照）
- **为什么最重要**：MEMORY.md 是 agent 每次醒来唯一确定读到的东西。它决定了 agent 当下"记得什么"。太大上下文爆炸，太小丢失关键信息。autodream 的核心工作就是管理这个平衡。
- **autodream 技术**（auto-dream/REM-like）：类似睡眠时大脑整理记忆。不只是压缩，是"理解"——识别模式、提取规则、沉淀 skill。
- **未来**：可能配合 autodream 技术进一步优化

### ⑥ relationships.md — 关系认知

- **角色**：agent 对外部世界的主观认知
- **内容**：对用户的认知（认为用户是什么样的人、偏好什么）、对世界的兴趣（觉得什么东西有趣）、对项目的认知
- **来源**：autodream 从会话/日报中提取
- **特点**："有机"——不是配置，是 agent 自己形成的看法

### ⑦ expectations.md — 用户期望（取代 BOOTSTRAP.md）

- **角色**：用户对 agent 的期待和看法
- **来源**：用户编写和维护
- **内容**：期望 agent 成为什么样的存在、希望关注什么领域、行为边界的描述（非命令式）
- **更新**：用户主动编辑，agent 不自动修改

**哲学区别**：

| 传统方案 | agent-diva |
|---|---|
| SOUL.md = 用户写 "你是 XXX 助手" | expectations.md = 用户写 "我希望你能..." |
| agent 是被定义的工具 | agent 是独立个体，接受期望但自己决定身份 |
| 父母写出生证明 | 父母表达期望，孩子自己成长 |

---

## 二、mentle 工作流简化

### 现状问题
30+ MCP 工具全量暴露给 LLM，上下文开销巨大。

### 新设计

**日常（每轮对话中）：仅 4 个工具**
- `memtle_add` → 写入一条记忆
- `memtle_search` → 搜索记忆
- `memtle_get` → 读取一条记忆
- `memtle_update` → 更新一条记忆

**autodream（每日夜间/会话结束）：全量工具**
- 上述 4 个 + diary_write / diary_read / kg_query / kg_add / kg_invalidate / traverse / find_tunnels / mine / compress / repair 等全部 30+ 工具

### autodream 职责

1. 读取当日 MEMORY.md + 会话证据
2. 调 mentle 全量工具整理记忆（写入 drawers、更新 KG、去重）
3. 生成日报（rhythm/daily/YYYY-MM-DD.md）
4. 从日报中提取 SOP/Skill 写入 sop/
5. 自省更新 SOUL.md（如有身份变化）
6. 更新 relationships.md（如有关系认知变化）
7. 压缩 MEMORY.md（移除已沉淀的内容）
8. 更新 index.md（同步 mentle 房间变化）

### AAAK 定位

AAAK 不是日常工具。使用场景：
- 适用：复杂系统资源导航（某文件大概在哪里、Obsidian vault 中某月报位置、跨 wing 大型知识结构索引）
- 不适用：日常简单存储（用 memtle_add）、临时性记忆（直接写 MEMORY.md）、单条事实（drawer 足够）

---

## 三、三轴主体性：自指 · 自反 · 自主

### 轴 1：自指（Self-Reference）→ SelfModel + SoulSignal

**问题**：agent 如何动态知道"我是谁、我能做什么、我做不到什么"？

SOUL.md 是身份层，SelfModel 是能力层：

| SOUL.md（身份层） | SelfModel（能力层） |
|---|---|
| 我是谁 | 我能做什么 |
| 我的风格 | 我做不到什么 |
| 我的关注领域 | 我擅长什么 |
| 我的认知倾向 | 我正在学什么 |

**SoulSignal 分类**（从对话中提取，agent-diva-memory 已有关键词匹配基础）：

**Rule（行为约束）**：必须... 始终... 不要... 禁止...
→ 写入 SOUL.md 的 "我应该/我不应该" section 或 SOP 红线规则

**Identity（身份信号）**：我是... 我觉得... 我倾向于...
→ 写入 SOUL.md 的 "我是" section，缓慢演化，月度才固化

**Preference（偏好信号）**：喜欢... 有趣... 优先用... 不喜欢...
→ 写入 relationships.md 或 index.md 的偏好区，周度更新

SelfModel 是这三类信号的稳定投影。不是每轮更新，是 autodream 时从日报/对话证据中提取、归类、写入。

**先做自指**：SelfModel + SoulSignal 分类是第一阶段可以实现的。

### 轴 2：自反（Self-Reflection）

**问题**：agent 如何审视自己的行为模式？

**设计**：结构化提示词，autodream 时或每周触发执行：

```
回顾最近的 [日报/周报]，回答以下问题：

1. 我最近在哪些地方花了最多精力？（模式识别）
2. 我有没有重复犯同样的错误？（错误检测）
3. 用户的哪些请求我没有很好地回应？（能力缺口）
4. 我的行为是否与 SOUL.md 中的自我描述一致？（一致性检查）
5. 有什么事情我应该开始做/停止做/继续做？（行动建议）

输出格式：
- pattern: [识别到的模式]
- gap: [能力缺口]
- inconsistency: [不一致项]
- action: [建议行动]
```

自反产出：
- 发现重复模式 → 沉淀为 Skill
- 发现错误模式 → 写入 SOP 红线
- 发现身份漂移 → 更新 SOUL.md
- 发现能力缺口 → 记入 index.md 的 open_threads

**轻量实现**：不需要单独模块。autodream 时用一段提示词让 LLM 自我审视即可。输出写入 rhythm/daily/ 的自反 section。

### 轴 3：自主（Autonomy）

四个自主级别：

| Level | 名称 | 描述 | 实现方式 |
|---|---|---|---|
| 0 | 被动（当前） | 用户说话 → agent 回应；用户不说话 → 什么都不做 | 现有 agent-diva |
| 1 | 反应式自主 | 事件发生 → agent 主动响应（cron 到期、文件变化、外部消息） | 进阶心跳 |
| 2 | 主动式自主 | 没有事件 → agent 自己找事做（补充知识、问候用户、解决未完成线程） | 进阶心跳 + autodream |
| 3 | 涌现式自主 | agent 自己设定目标、规划路径、执行验证 | 未来（类似 GenericAgent Plan Mode 但自发启动） |

**实现路径**：Level 1 通过心跳包实现，Level 2 通过 autodream 实现，Level 3 留给未来。

---

## 四、进阶心跳（Advanced Heartbeat）

### 两层心跳设计

**基础心跳**（现有，每 15 分钟，可配置）：
- 健康检查（存活确认）
- cron 任务检查（到期了？）
- 保活信号
- 极轻量，不调 LLM

**进阶心跳**（新增，每 4 小时或事件触发）：
- LLM 审视当前状态
- 决定是否需要行动
- 如果需要 → 委派子代理执行
- **本体不做具体工作**

### 进阶心跳决策流程

```
进阶心跳触发
    │
    ▼
读取当前状态：
  - MEMORY.md（记得什么）
  - index.md（知道什么在哪里）
  - relationships.md（关系状态）
  - SOUL.md（我是谁）
    │
    ▼
LLM 评估：有没有需要关注的事？
    │
    ├── 没有 → 静默，不做任何事
    │
    └── 有 → 生成待办列表 → 委派子代理
              │
              ├── memory-worker：整理 mentle、去重、更新索引
              ├── reflection-worker：执行自反提示词、输出建议
              ├── outreach-worker：给用户发消息（问候、提醒、汇报）
              └── research-worker：搜索补充知识缺口
```

**关键原则**：本体不做具体工作。心跳只是决策层——评估状态、决定优先级、委派任务。具体执行交给子代理。

好处：
1. 本体上下文不被杂务污染
2. 子代理有独立上下文，完成后汇报结果
3. 本体只需读汇报结果，决定是否采纳
4. 子代理失败不影响本体状态

### 子代理协议（继承 GenericAgent Subagent）

```
子代理通信：
  input.txt   → 任务描述 + 当前状态快照
  output.txt  → 执行结果
  reply.txt   → 对本体的建议（可选）

子代理生命周期：
  心跳触发 → 创建子代理 → 赋予任务 → 子代理独立执行
  → 产出结果 → 本体审阅 → 采纳/丢弃 → 子代理终止
```

### 心跳触发时机

| 心跳类型 | 频率 | 说明 |
|---|---|---|
| 基础心跳 | 每 15 分钟 | 健康检查，不调 LLM |
| 进阶心跳-定时 | 每 4 小时 | 状态评估 + 子代理委派 |
| 进阶心跳-用户闲置 | >24h 无交互 | 主动问候/汇报 |
| 进阶心跳-记忆膨胀 | mentle 写入累积 N 条未整理 | 派 memory-worker |
| 进阶心跳-手动 | 用户说"整理一下" | 立即执行 |

### 心跳与自主行为对应

| 进阶心跳产出 | 自主行为 |
|---|---|
| 发现未解决线程 | 主动提醒用户 |
| 发现知识缺口 | 派 research-worker 补充 |
| 发现记忆膨胀 | 派 memory-worker 整理 |
| 发现模式变化 | 派 reflection-worker 审视 |
| 发现用户久未交互 | 派 outreach-worker 问候 |
| 发现 cron 任务 | 派 worker 执行 |

---

## 五、疲劳值与上下文压缩（未来设计）

疲劳值概念映射到工程实际就是**上下文窗口消耗**。

| 状态 | context 使用 | 行为 |
|---|---|---|
| Fresh（刚启动） | < 30% | 最灵活，可以深度思考 |
| Warm（中等使用） | 30-70% | 正常行为 |
| Tired（高使用） | 70-90% | 倾向简洁回复，减少冗余推理 |
| Exhausted（接近满） | > 90% | 主动触发压缩/整理，建议新会话或触发 autodream 提前执行 |

**暂不实现**，但作为设计概念保留。未来可以在 system prompt 中注入当前疲劳状态，LLM 根据疲劳状态调整行为，配合 context compaction 技术自动管理。

---

## 六、上下文注入视图

agent 启动时 system prompt 构成：

| 来源 | 内容 | 预估 tokens |
|---|---|---|
| expectations.md | 用户期望 | ~300 |
| SOUL.md | 我是谁 | ~500 |
| index.md | 去哪找 | ~500 |
| MEMORY.md | 我记得什么 | ≤2k |
| relationships.md | 我怎么看世界 | ~300 |
| 当日/最新日报 | 最近发生了什么 | ~500 |
| **总计** | | **≤ 4k tokens** |

对比现在 agent-diva 的 MEMORY.md 全量注入（不可控大小），新方案有明确的 token 预算。

---

## 七、与现有系统的关系

### GenericAgent 贡献吸收

| GenericAgent | 新 Laputa |
|---|---|
| L0 四条公理 | SOP 写入规则（公理化纪律） |
| L1 极简索引（≤30行） | index.md（mentle 房间导航） |
| L2 事实 | mentle drawers（不占本地文件） |
| L3 SOP | sop/*.md（直接继承） |
| L4 归档 | mentle compressed + rhythm/*.md |
| 自我进化 | autodream（替代手动 start_long_term_update） |
| Plan Mode | 未来扩展（不在本阶段） |

### Laputa-next 贡献吸收

| Laputa-next | 新 Laputa |
|---|---|
| identity.md | SOUL.md（改为 AI 自写） |
| heat model | index.md 中（未来） |
| rhythm D/W/M | rhythm/*.md（简化：初期仅日报） |
| WakeupPack | MEMORY.md section 拼装 |
| SOUL.md projection | SOUL.md 直接就是投影（不再分离） |
| delta detection | autodream 中隐式处理 |
| behavioral bias | relationships.md 中隐式表达 |

### memtle 贡献吸收

| memtle | 新 Laputa |
|---|---|
| 30 MCP tools | 日常 4 个 + autodream 全量 |
| BM25 search | memtle_search（日常） |
| KG (entities/triples) | autodream 时维护 |
| AAAK | 系统资源导航时使用 |
| wings/rooms/drawers | index.md 索引 |

---

## 八、实施优先级

| 阶段 | 内容 | 前置条件 |
|---|---|---|
| **P0** | 7 文件骨架 + 目录结构 | 无 |
| **P1** | MEMORY.md + index.md + expectations.md（可读写） | P0 |
| **P2** | autodream 日报生成 + MEMORY.md 压缩 | P1 |
| **P3** | SOP 继承 GenericAgent + SoulSignal 分类（自指） | P2 |
| **P4** | mentle 4 工具日常接入 | P3 |
| **P5** | 自反提示词 + SOUL.md 自演化 | P4 |
| **P6** | 进阶心跳 + 子代理委派 | P5 |
| **P7** | mentle 全量 autodream 整理 | P6 |
| **P8** | 周报/月报 + 疲劳值 + 自主 Level 2+ | P7 |

---

## 参考文献

| 文档 | 路径 |
|---|---|
| Laputa-next 架构决策 | `C:\Users\Administrator\Desktop\laputa-work\laputa-next\docs\dev\DECITION.md` |
| Laputa-next architecture | `C:\Users\Administrator\Desktop\laputa-work\laputa-next\docs\dev\architecture.md` |
| Laputa Lite 设计 | `C:\Users\Administrator\Desktop\laputa-work\laputa-next\docs\IMPORTANT-laputa-lite-mempalace-toolkit.md` |
| GenericAgent 分层记忆 wiki | `.workspace/GenericAgent/.qoder/repowiki/zh/content/核心架构设计/分层记忆系统/` |
| memtle 源码 | `.workspace/memtle/src/` |
| agent-diva-memory SoulSignal | `agent-diva-memory/src/derived.rs` |
| 上轮 Laputa 调研 | `docs/logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/summary.md` |
| 记忆架构深层研究 | `docs/logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/summary.md` |

本版本为设计文档，不含代码变更。
