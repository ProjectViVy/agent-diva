# DivaGeneric 架构设计

> 状态：设计固化文档，不包含本轮代码实现。
> 当前分支：`divageneric`
> 基线：`origin/vrm-memory-test`
> 目标：把 agent-diva 改造为类 GenericAgent 的可学习系统，同时保留 Laputa 人格连续性与 Mentle 可选存储后端。

## 1. 当前审批通过方案

DivaGeneric 的主方向是：

```text
DivaGeneric =
  GenericAgent 记忆纪律
  + agent-diva 模块化运行时
  + Laputa 人格连续性
  + Mentle 可选检索/存储后端
```

其中 GenericAgent 化是主目标，Laputa 不是主架构中心。Laputa 只负责人格、唤醒投影、节律和身份连续性，不接管通用记忆、知识沉淀、SOP 或工具决策。Mentle 也不是主架构中心，它是可选的深层事实、证据、索引和图检索后端。

当前批准方向要求：

- 不绕过 `MemoryProvider` 新建并行记忆管线。
- 不把 GenericAgent 原始文件名机械移植进 agent-diva；沿用本项目此前已经讨论并沉淀的 Diva/Laputa 文件名。
- 不让 Laputa 代替 Generic memory。
- 不让 Mentle 全量工具污染日常聊天上下文。
- 在线路径保持轻量，离线路径承担整理、去重、归档和学习候选处理。

## 2. 当前 agent-diva 架构事实

本节只记录当前代码已经存在的接缝，后续设计必须绑定这些接缝推进。

### 2.1 `agent-diva-core`

`agent-diva-core` 承载稳定领域边界：

- 配置 schema 与校验：`agent-diva-core/src/config/*`
- session 与历史持久化：`agent-diva-core/src/session/*`
- memory contract 与默认文件记忆：`agent-diva-core/src/memory/*`
- soul 状态：`agent-diva-core/src/soul/mod.rs`
- security、cron、event bus、heartbeat 等基础能力

当前最重要的记忆边界是 `agent-diva-core/src/memory/provider.rs` 的 `MemoryProvider`。它已经定义四个生命周期钩子：

- `system_prompt_block(&SystemPromptRequest) -> Result<SystemPromptResponse>`
- `prefetch(PrefetchRequest) -> Result<PrefetchResponse>`
- `sync_turn(SyncTurnRequest) -> Result<SyncTurnResponse>`
- `on_session_end(SessionEndRequest) -> Result<SessionEndResponse>`

这四个钩子分别对应：

- 启动/系统提示注入
- 在线意图召回
- 成功 turn 后持久化
- session 结束后的节律或清理

该 contract 已明确不能泄漏 MCP schema、CLI args、HTTP route 或后端模型类型。DivaGeneric 后续新增类型也必须保持这个原则。

### 2.2 `agent-diva-agent`

`agent-diva-agent` 是运行时核心：

- `agent-diva-agent/src/agent_loop.rs` 与 `agent-diva-agent/src/agent_loop/*`：AgentLoop 主循环
- `agent-diva-agent/src/context.rs`：`ContextBuilder`，负责 system prompt、skills、memory block、session history 装配
- `agent-diva-agent/src/tool_assembly.rs`：`ToolAssembly`，负责工具注册和 subagent 工具隔离
- `agent-diva-agent/src/mentle_runtime.rs`：`MentleRuntime`，负责 Mentle toolkit、HybridMemoryProvider、动态 Mentle 工具
- `agent-diva-agent/src/consolidation.rs`：旧会话段总结到 long memory/history
- `agent-diva-agent/src/subagent.rs`：子代理入口

当前在线路径中的关键事实：

- `AgentLoop` 在 turn 开始从用户消息推导轻量 `prefetch_intent`。
- 有意图时调用 `self.memory_provider.prefetch(...)`。
- `prefetch` 成功后把召回结果作为额外 system message 插入主 system prompt 之后。
- `prefetch` 失败只记录 warn，不阻断主回复。
- 回复完成后保存 session。
- 达到窗口后调用 `consolidation::consolidate(...)`。
- consolidation 通过 `MemoryProvider::system_prompt_block()` 读取既有记忆，并通过 `MemoryProvider::sync_turn()` 写入更新。

这说明 Generic Core 应进入现有生命周期，而不是替换 AgentLoop。

### 2.3 `agent-diva-tools` / `agent-diva-tooling`

`agent-diva-tooling` 提供 tool trait、registry 和基础错误类型：

- `agent-diva-tooling/src/base.rs`
- `agent-diva-tooling/src/registry.rs`

`agent-diva-tools` 提供具体工具实现：

- filesystem
- shell
- web
- cron
- spawn
- attachment
- MCP SDK bridge

工具层只负责暴露能力，不承载学习分类、长期记忆策略或人格演化决策。Generic Core 不应该被写成工具层策略。

### 2.4 `agent-diva-manager`

`agent-diva-manager` 是默认本地 gateway 与 HTTP/control plane，`agent-diva-cli` 依赖它。当前 manager runtime 会从 config 创建 `MentleToolRuntimeConfig`，并组装 `AgentLoop`。

因此 DivaGeneric 的配置面板、runtime flag、control API 可以后续由 manager/gateway 暴露，但核心学习策略不放在 manager。

### 2.5 `agent-diva-gui`

`agent-diva-gui` 是配置和可视化入口，不承载核心记忆逻辑。GUI 后续只负责：

- 展示 Generic/Laputa/Mentle 状态
- 配置 `generic.enabled`、学习模式、Mentle 工具模式
- 展示学习候选、用户确认、拒绝、归档和撤销

GUI 不直接写入 Generic memory 后端，必须通过 manager 或稳定 domain API。

### 2.6 已存在关键能力

当前分支已经具备 DivaGeneric 的关键骨架：

- `MemoryProvider` 四钩子生命周期。
- `ContextBuilder` 通过 provider 注入 startup memory block。
- `AgentLoop` 在线 prefetch 注入与失败降级。
- `consolidation` 通过 provider sync，不直接依赖具体后端。
- `MentleRuntime` 可以构造 `HybridMemoryProvider`。
- `HybridMemoryProvider` 已实现 Markdown 权威回退 + Mentle secondary write。
- `MentleToolRuntimeConfig` 支持 `off/read_only/full/custom`。
- `ToolAssembly::build_subagent_registry()` 清空 custom tools，子代理默认不继承 Mentle 工具。
- 当前 `SOUL.md`、`IDENTITY.md`、`USER.md`、`BOOTSTRAP.md` 已通过 `ContextBuilder` 注入 system prompt。

## 3. DivaGeneric 总体设计

DivaGeneric 增加一个概念层：Generic Core。

Generic Core 不替代 `AgentLoop`，不替代 `MemoryProvider`，不成为新工具系统。它只提供：

- 学习纪律
- 证据分类
- 分层索引策略
- 学习候选生成
- 用户确认后的落库目标选择
- L1 导航块渲染
- Laputa wakeup block 的组合协作

推荐未来实现位置：

```text
agent-diva-generic/
  src/lib.rs
  src/policy.rs
  src/layers.rs
  src/candidate.rs
  src/index.rs
  src/wakeup.rs
  src/provider_adapter.rs
```

职责边界：

- `agent-diva-generic`：策略、分类、索引、候选协议。
- `agent-diva-core`：只放跨 crate 稳定 domain types，例如未来被多个 crate 共用的 `LearningCandidate`、`MemoryLayer`。
- `agent-diva-agent`：只调用 Generic Core，不内嵌分类策略。
- `agent-diva-tools`：只暴露必要工具，不承载学习决策。
- `agent-diva-manager`：暴露配置、状态和控制面。
- `agent-diva-gui`：展示配置与人工确认入口。

如果后续实现阶段需要降级复杂度，可以先在 `agent-diva-agent/src/generic/` 建局部模块。但默认架构采用 workspace crate，因为学习策略是跨 AgentLoop、provider、manager、GUI 的长期边界。

## 4. GenericAgent L0-L4 到 agent-diva 的映射与文件名定稿

GenericAgent 的价值是分层纪律，但 DivaGeneric 的文件名必须沿用本项目此前讨论中已经形成的 Diva/Laputa 命名体系。也就是说：不照搬 GenericAgent 的 `global_mem_insight.txt`、`global_mem.txt`、`L4_raw_sessions/` 等原始文件名；但保留 L0-L4 的分层纪律，并把它映射到下列确定文件。

第一阶段确定的文件/目录名：

```text
.laputa/
  SOUL.md
  expectations.md
  index.md
  MEMORY.md
  relationships.md
  sop/
    *.md
  rhythm/
    daily/
    weekly/
    monthly/
  inbox/
    learning-candidates.jsonl
    decisions.jsonl
```

兼容期继续读取现有根目录文件：

```text
SOUL.md
IDENTITY.md
USER.md
BOOTSTRAP.md
memory/MEMORY.md
memory/HISTORY.md
```

兼容期规则：

- `.laputa/SOUL.md` 是目标身份文件；根目录 `SOUL.md` 是兼容输入/输出。
- `.laputa/expectations.md` 是用户期望文件；根目录 `USER.md` 与 `BOOTSTRAP.md` 只做迁移兼容。
- `.laputa/MEMORY.md` 是 wakeup/短期压缩记忆；`memory/MEMORY.md` 在过渡期仍是 Markdown 权威回退。
- `.laputa/index.md` 是 L1 导航索引，不再另起 `global_mem_insight.txt`。
- `.laputa/sop/*.md` 是 L3 SOP/Skill，不再默认放入 `memory/sop/`。
- `.laputa/rhythm/*` 是 L4 日报/周报/月报归档。
- `.laputa/inbox/*.jsonl` 是学习候选和决策审计，不直接进入日常 prompt。

### 4.0 分层总表

| 层级 | 含义 | 确定文件/目录 | 是否日常注入 prompt |
|---|---|---|---|
| L0 | 学习公理与写入纪律 | `.laputa/sop/memory-management.md`，必要规则同步到 `AGENTS.md` | 只注入短规则摘要 |
| L1 | 极小索引/导航 | `.laputa/index.md` | 是，限制 30 行以内 |
| L2 | 稳定事实/当前 wakeup 记忆 | `.laputa/MEMORY.md`，`.laputa/relationships.md`，后续 Mentle drawers | 是，但有 token 预算 |
| L3 | SOP/Skill/可复用流程 | `.laputa/sop/*.md` | 只注入索引，按需读取正文 |
| L4 | 原始证据/会话/节律归档 | `.laputa/rhythm/*`，`memory/HISTORY.md`，session store，后续 Mentle diary/evidence | 否，只通过 L1 或 prefetch 召回 |
| Inbox | 学习候选与决策审计 | `.laputa/inbox/learning-candidates.jsonl`，`.laputa/inbox/decisions.jsonl` | 否 |

### L0：学习公理和写入纪律

职责：

- 定义什么可以被学习。
- 定义什么必须先进入候选而不能直接落库。
- 定义“无证据不记忆”“未验证不升级 SOP/Skill”“用户敏感偏好需要确认”等规则。

确定文件名：

- `.laputa/sop/memory-management.md`
- `AGENTS.md` 中保留必要的跨项目强约束摘要
- `agent-diva-generic::GenericPolicy`
- 后续由 `consolidation` 和 autodream/rhythm worker 调用

`memory-management.md` 至少包含：

- action-verified：行动或证据验证优先。
- sacred-no-silent-delete：不可静默删除长期记忆，删除需要记录原因。
- no-volatile-state：易变状态不进入 L2/L3。
- minimal-sufficient-pointer：L1 只放最小充分指针。
- confirm-sensitive-learning：用户偏好、身份边界、关系判断默认先入候选。

禁止：

- 在 `AgentLoop` 中硬编码学习分类。
- 让 LLM 工具调用直接把未经确认的信息写入 L2/L3。

### L1：极小索引

职责：

- 给模型一个不超过 30 行的导航块。
- 只包含 pointers，不放大段正文。
- 指向 L2 facts、L3 SOP/Skill、Laputa rhythm、Mentle rooms/drawers。

确定文件名：

- `.laputa/index.md`
- `GenericIndex`
- `GenericCore::build_index_block`
- `ContextBuilder` 通过 `MemoryProvider::system_prompt_block()` 间接拿到渲染结果

形态示例：

```markdown
## active_rooms
- provider-routing -> mentle:project/provider-routing
- gui-settings -> mentle:project/gui-settings

## hot_drawers
- provider-model-id-safety -> .laputa/MEMORY.md#provider-model-id-safety
- mentle-tool-policy -> .laputa/sop/mentle-tool-policy.md

## open_threads
- generic-learning-candidates -> .laputa/inbox/learning-candidates.jsonl

## recent_sop
- memory-provider-routing -> .laputa/sop/memory-provider-routing.md
```

`index.md` 固定 section：

```markdown
- active_rooms: provider-routing, gui-settings, laputa-rhythm
- hot_drawers: provider-model-id-safety, mentle-tool-policy
- open_threads: generic-learning-candidates
- sop_pointers: memory-provider-routing.md, mentle-tool-selection.md
```

约束：

- L1 不存储完整事实。
- L1 不替代 Mentle search。
- L1 在 startup prompt 中短小稳定，必要时由 prefetch 再做深召回。
- L1 总行数上限默认 30 行，配置项为 `generic.index_max_lines`。

### L2：稳定事实

职责：

- 存放稳定事实、项目状态、用户确认过的关系事实、长期可检索材料。

确定文件名：

- `.laputa/MEMORY.md`
- `.laputa/relationships.md`
- 后续 Mentle drawers/rooms

职责拆分：

- `.laputa/MEMORY.md`：短期 wakeup 记忆和当前压缩状态，是每次启动最重要的“我现在记得什么”。
- `.laputa/relationships.md`：外部世界、用户、项目、协作者和关系认知。
- Mentle drawers/rooms：深层事实、证据、长材料、项目细节。
- `memory/MEMORY.md`：兼容期权威 Markdown 回退，不能突然废弃。

后续形态：

- 当 L2 膨胀到文件态不适合直接注入时，事实迁入 Mentle。
- `.laputa/MEMORY.md` 只保留 wakeup 摘要、关键关系和指向 Mentle 的 pointers。
- `HybridMemoryProvider` 负责把 Markdown fallback + Mentle snapshot 组合进 prompt/prefetch。

约束：

- L2 写入必须有 evidence ref。
- 自动提取只能生成候选，不能默认变成稳定事实。
- 用户关系、偏好、身份判断默认先写 `.laputa/inbox/learning-candidates.jsonl`，确认后再进入 `.laputa/relationships.md` 或 `.laputa/MEMORY.md`。

### L3：SOP / Skill

职责：

- 记录已验证的操作流程、工程规则、可复用技能和自动化步骤。

确定文件名：

- `.laputa/sop/*.md`
- 必要时同步为正式 Codex/agent skill，但 `.laputa/sop/*.md` 是第一落点

命名规则：

- 使用 kebab-case，例如 `.laputa/sop/provider-model-id-safety.md`。
- 每个 SOP 文件包含：目的、适用范围、步骤、验证方式、反例、最后验证时间。
- 只有已验证行为才能进入 SOP；一次性经验只能先进入候选。

约束：

- 必须来自已验证行为。
- 不能把一次失败探索直接升级为 SOP。
- 对工程流程有影响时需要 acceptance/verification 记录。
- L3 正文不默认全量注入 prompt，只通过 `.laputa/index.md` 暴露指针。

### L4：原始会话、日报、证据归档

职责：

- 保留原始 session、history、daily rhythm、证据和审计材料。

确定文件名：

- 当前 `SessionManager` / session store
- `memory/HISTORY.md`
- `.laputa/rhythm/daily/YYYY-MM-DD.md`
- `.laputa/rhythm/weekly/YYYY-WNN.md`
- `.laputa/rhythm/monthly/YYYY-MM.md`
- 后续 Mentle diary 或 evidence drawer

约束：

- L4 是证据层，不直接塞入日常 prompt。
- L4 通过 L1 指针或 prefetch 定向召回。
- daily rhythm 是 autodream 的主要输出，不是装饰性报告；它可以产出 L2/L3/Laputa delta 候选，但不能无审计地重写 L2/L3。

### Inbox：候选层

Inbox 不是 L0-L4 之一，但它是 DivaGeneric 防止污染长期记忆的必要缓冲层。

确定文件名：

- `.laputa/inbox/learning-candidates.jsonl`
- `.laputa/inbox/decisions.jsonl`

`learning-candidates.jsonl` 存放：

- 从 conversation、tool result、daily rhythm、Mentle recall 中提取出的待确认内容。
- suggested_layer：`L2Fact`、`L3SopOrSkill`、`LaputaPersona`、`Discard`。
- evidence_refs：session、history、rhythm、Mentle drawer 或文件路径。

`decisions.jsonl` 存放：

- 用户确认、拒绝、重分类、撤销。
- policy 自动低风险决策。
- 写入目标，例如 `.laputa/MEMORY.md`、`.laputa/relationships.md`、`.laputa/sop/*.md` 或 Mentle room/drawer。

## 5. Laputa 并行架构

Laputa 定位为人格连续性层，不是 memory backend。

第一阶段文件态定稿：

```text
.laputa/
  SOUL.md
  expectations.md
  index.md
  MEMORY.md
  relationships.md
  sop/
    memory-management.md
    *.md
  rhythm/
    daily/
    weekly/
    monthly/
  inbox/
    learning-candidates.jsonl
    decisions.jsonl
```

与当前 agent-diva 文件的兼容关系：

- 当前根目录 `SOUL.md`：兼容期可与 `.laputa/SOUL.md` 双向同步；目标态以 `.laputa/SOUL.md` 为准。
- 当前 `IDENTITY.md`：迁移到 `.laputa/SOUL.md` 的身份 section；短期仍由 `ContextBuilder` 读取。
- 当前 `USER.md`：迁移到 `.laputa/expectations.md` 和 `.laputa/relationships.md`；短期仍保持兼容读取。
- 当前 `BOOTSTRAP.md`：由 `.laputa/expectations.md` 取代；短期仍用于首次引导。
- 当前 `memory/MEMORY.md`：兼容期仍是 Markdown 权威回退；目标态由 `.laputa/MEMORY.md` 承接 wakeup 记忆。
- 当前 `memory/HISTORY.md`：继续作为 L4 历史日志；后续可由 `.laputa/rhythm/*` 和 Mentle diary 补强。

接入方式：

- Laputa 通过 `MemoryProvider.system_prompt_block()` 注入 wakeup/soul projection。
- Laputa rhythm 通过 `MemoryProvider.on_session_end()` 或未来 autodream worker 触发。
- Laputa 的 SOUL delta 不直接改 Generic L2/L3；需要通过 Generic Core 候选协议审查。

Laputa 不应该：

- 直接替代 `MemoryProvider` 生命周期。
- 接管 `consolidation` 的通用学习分类。
- 把人格 projection 和稳定知识事实混写。
- 以日常聊天工具形式暴露所有内部维护能力。

## 6. Mentle 工具暴露策略

当前 `agent-diva-agent/src/tool_config/mentle.rs` 中的 `MentleToolRuntimeConfig` 支持：

- `off`
- `read_only`
- `full`
- `custom`

`read_only` 当前只允许：

- `memtle_status`
- `memtle_search`

DivaGeneric 延续并收紧该策略。

### 6.1 日常聊天 profile

日常聊天目标是少工具、低干扰、可解释：

- 优先暴露 `memtle_status`
- 优先暴露 `memtle_search`
- 写入/更新/删除类工具只在 `custom` 白名单明确打开
- 默认不暴露 full 工具集

日常 prompt 只能提及当前 registry 中实际存在的 Mentle 工具。当前 `ContextBuilder::with_mentle_tools(...)` 和 `set_mentle_prompt_state(...)` 已经提供这个基础。

### 6.2 Autodream / 后台整理 profile

后台整理目标是完整维护：

- 可以使用 full Mentle 工具集。
- 用于 daily rhythm、去重、归档、索引更新、候选升级。
- 不进入日常聊天上下文。

因此 future worker 应创建独立 tool profile，而不是复用日常 `AgentLoop` registry。

### 6.3 Subagent 隔离

当前 `ToolAssembly::build_subagent_registry()` 会：

- 调用 `BuiltInToolsConfig::for_subagent()`
- 清空 `custom_tools`
- 禁用 spawn/cron/attachment 等不适合子代理继承的能力

因此 subagent 默认不继承 Mentle 工具。DivaGeneric 应保持这个设计。只有明确声明的后台 worker 或受控 subagent profile 才能获得 Mentle 工具。

## 7. 在线路径与离线路径

### 7.1 在线路径

在线路径保持轻量：

```text
Inbound message
  -> derive_prefetch_intent
  -> MemoryProvider.prefetch
  -> ContextBuilder.build_messages
  -> LLM
  -> tool calls
  -> final response
  -> session save
  -> consolidation threshold check
```

在线路径允许做：

- 意图召回
- L1 index 注入
- Laputa wakeup/soul projection 注入
- 小规模工具调用
- session 保存
- 达阈值后触发 consolidation

在线路径不做：

- 大规模去重
- 批量重写长期记忆
- 自动升级 SOP/Skill
- SOUL 大规模重写
- 全库 Mentle 整理

### 7.2 离线路径

离线路径承担学习整理：

```text
session/history evidence
  -> daily rhythm / autodream
  -> Generic classification
  -> LearningCandidate
  -> user or policy decision
  -> SOP/Skill/Laputa delta/Mentle drawer/index update
```

离线路径可以做：

- 批量归档
- 去重
- 日报/周报/月报
- SOUL delta 候选
- L1 index 更新
- L2 fact 写入
- L3 SOP/Skill 候选生成
- Mentle room/drawer 整理

失败策略：

- daily rhythm 失败不能破坏 session persistence。
- Mentle 后台整理失败不能影响 Markdown 权威回退。
- Laputa projection 失败必须降级为现有 `SOUL.md` / `MEMORY.md` 注入。

## 8. Plan Mode 架构

Plan Mode 是 DivaGeneric 的复杂任务编排层，用来承接“三步以上、有依赖、多文件协同、需要验证”的任务。它借鉴 GenericAgent 的四阶段流程，但不照搬纯文件协议，也不重写 agent-diva 的 `AgentLoop`。核心原则是：新增编排层，复用现有 loop、tool registry、subagent、event bus 和 memory provider。

Plan Mode 是 Diva 原生运行时能力，不是 Laputa 能力。Laputa 可以提供人格边界、用户期望、关系认知和沉淀目标，但不能拥有复杂任务执行状态机。换句话说：Plan 负责“怎么拆解、执行、验证任务”，Laputa 负责“以什么人格边界和长期偏好去做，以及做完后沉淀什么”。

职责归属：

- Diva 原生能力：`PlanOrchestrator`、`PlanStateStore`、审批门禁、step runner、verification verdict、pause/resume/retry、工具/subagent profile。
- Laputa 参与输入：读取 `.laputa/SOUL.md`、`.laputa/expectations.md`、`.laputa/relationships.md`，作为计划偏好和行为边界。
- Laputa 参与输出：计划完成后生成 `.laputa/inbox/learning-candidates.jsonl`、`.laputa/sop/*.md` 候选、`.laputa/rhythm/*` 摘要和 `.laputa/index.md` 指针。
- 禁止：把计划运行状态放进 `.laputa/`，或让 Laputa rhythm/autodream 直接驱动 step 执行。

### 8.1 触发条件与非目标

默认触发条件：

- 任务预计超过 3 个有依赖步骤。
- 涉及多个 crate、多个配置面或多个文档/代码文件。
- 需要先探索再决定实现方案。
- 需要用户确认计划或存在明显风险边界。
- 用户显式要求进入 Plan Mode。

不进入 Plan Mode 的情况：

- 简单问答。
- 单文件小修。
- 明确命令式任务且无需探索。
- 用户要求“直接做，不要计划”且风险可控。

Plan Mode 非目标：

- 不替代 `AgentLoop` 单回合执行。
- 不把所有任务都计划化。
- 不让 `MemoryProvider` 承载计划状态机。
- 不把计划状态混进 session message history。
- 不默认开启并行执行；第一阶段只做串行闭环。

### 8.2 确定文件名与目录结构

Plan Mode 文件必须沿用此前提到的 `plan.md`、`exploration_findings.md` 等概念，但归档到 Diva 原生运行时目录 `.diva/plans/<plan-id>/` 下，避免污染根目录，也避免把执行状态混入 Laputa 人格/记忆目录。

```text
.diva/
  plans/
    active.json
    <plan-id>/
      input.md
      exploration_findings.md
      plan.md
      state.json
      events.jsonl
      verification.md
      evidence/
        step-001.md
        step-002.md
```

文件职责：

- `active.json`：当前未完成计划索引，可为空。
- `input.md`：用户目标、约束、禁止事项、验收标准。
- `exploration_findings.md`：探索阶段产物，只记录事实、风险、代码接缝和候选方案，不写执行步骤。
- `plan.md`：用户可读计划，包含 checklist、依赖、验证方式和需要确认的决策。
- `state.json`：机器可读计划状态，和 session history 分离。
- `events.jsonl`：`PlanDrafted`、`StepStarted`、`StepBlocked`、`PlanCompleted` 等事件流。
- `verification.md`：验证阶段结论，必须给 `PASS`、`FAIL` 或 `PARTIAL`。
- `evidence/*.md`：每个 step 的证据、命令、输出摘要、文件变更摘要。

与 Laputa 的文件关系：

- `.diva/plans/*`：计划运行状态和执行证据，属于 Diva runtime。
- `.laputa/SOUL.md`：计划时可读的人格边界。
- `.laputa/expectations.md`：计划时可读的用户期望。
- `.laputa/relationships.md`：计划时可读的关系/项目认知。
- `.laputa/inbox/learning-candidates.jsonl`：计划完成后可写入的学习候选。
- `.laputa/sop/*.md`：经过验证后的流程沉淀目标。
- `.laputa/rhythm/*`：计划完成后的节律摘要目标。

### 8.3 四阶段状态机

Plan Mode 固定四阶段：

```text
Explore -> Plan -> Execute -> Verify
```

阶段职责：

- Explore：收集事实。可用 subagent 探索，但主 agent 不在此阶段承诺执行方案。
- Plan：生成 `plan.md`，标注 checklist、依赖、验证方式、风险点和需用户确认项。
- Execute：按 `plan.md` 串行执行 `[ ]` step，每完成一步写 evidence 并更新 `state.json`。
- Verify：独立验证。结论必须是 `PASS`、`FAIL` 或 `PARTIAL`，不能用含糊措辞替代。

硬门禁：

- 没有 `exploration_findings.md` 不进入 Plan。
- 没有用户确认或策略确认不进入 Execute。
- `plan.md` 中存在未完成 `[ ]` 时不能进入最终完成态。
- Verify 阶段必须读取 `plan.md` 和 `state.json`，确认 0 个未完成 required step。
- `FAIL` 或 `PARTIAL` 不能自动标记计划成功；必须生成 follow-up、rollback 或用户确认。

### 8.4 PlanOrchestrator 边界

推荐新增：

```text
agent-diva-agent/src/plan_mode/
  mod.rs
  orchestrator.rs
  store.rs
  types.rs
  events.rs
  verifier.rs
```

后续如果 Plan Mode 成为跨 manager/gui 的稳定能力，可把 domain types 上移到 `agent-diva-core`，但第一阶段不默认新增独立 crate。

职责边界：

- `AgentLoop`：仍负责单回合 LLM/tool 执行。
- `PlanOrchestrator`：负责计划生命周期、阶段门禁、step 调度、状态转移。
- `PlanStateStore`：读写 `.diva/plans/*`，不写 session message history。
- `SubagentManager`：可作为探索或验证 executor，但不拥有计划状态。
- `MemoryProvider`：只接收计划摘要、证据和最终可学习内容，不承载计划状态机。
- `GenericCore`：从计划 evidence 中生成学习候选，不直接控制计划执行。
- `manager/gui`：后续暴露计划状态、确认、暂停、继续、拒绝、重试入口。

### 8.5 计划数据类型草案

建议类型：

```rust
pub struct Plan {
    pub plan_id: String,
    pub title: String,
    pub goal: String,
    pub status: PlanStatus,
    pub steps: Vec<PlanStep>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct PlanStep {
    pub step_id: String,
    pub title: String,
    pub status: PlanStepStatus,
    pub depends_on: Vec<String>,
    pub evidence_path: Option<PathBuf>,
    pub verification: Option<VerificationVerdict>,
}

pub enum PlanStatus {
    Exploring,
    AwaitingApproval,
    Executing,
    Verifying,
    Completed,
    Failed,
    Partial,
    Paused,
}

pub enum VerificationVerdict {
    Pass,
    Fail { reason: String },
    Partial { reason: String },
}
```

事件类型：

- `PlanDrafted`
- `PlanApproved`
- `StepStarted`
- `StepCompleted`
- `StepBlocked`
- `StepRetried`
- `PlanPaused`
- `PlanResumed`
- `PlanVerificationCompleted`
- `PlanCompleted`

### 8.6 与 L0-L4 的关系

Plan Mode 自身不是新的记忆层。它产生的文件按用途进入 L0-L4：

- `input.md`、`exploration_findings.md`、`events.jsonl`、`evidence/*.md`：L4 evidence。
- `verification.md`：L4 evidence，可作为 L3 升级依据。
- 成功反复验证的流程：进入 `.laputa/sop/*.md`，即 L3。
- 稳定事实或项目状态：确认后进入 `.laputa/MEMORY.md`、`.laputa/relationships.md` 或 Mentle，即 L2。
- 计划执行中发现的规则：先入 `.laputa/inbox/learning-candidates.jsonl`，确认后可进入 L0/L3。
- `plan.md` 不直接成为 SOP；只有经过验证和抽象后的流程才能进入 L3。

### 8.7 工具与权限策略

第一阶段策略：

- 串行执行，不做依赖图并行。
- 每个 step 可声明 tool allowlist。
- 默认沿用当前 `ToolRegistry`，不新建工具执行内核。
- 高风险工具调用仍走现有安全/审批机制。
- subagent 默认不继承 Mentle 工具；Plan Mode 需要 Mentle 时由 step profile 明确配置。
- 验证 subagent 只读优先，除非用户明确授权修复。

### 8.8 用户交互与 GUI/Manager

CLI/聊天入口的最小交互：

- “进入计划模式”或复杂度门禁触发 plan draft。
- 展示 `plan.md` 摘要。
- 用户确认后执行。
- 用户可暂停、继续、拒绝、重试当前 step。
- 完成后展示 verification verdict。

manager/gui 后续入口：

- 当前 active plan。
- step 状态和 evidence。
- approval gate。
- pause/resume/retry。
- verification verdict。

### 8.9 配置建议

```toml
[generic.plan_mode]
enabled = true
auto_trigger = "complex_only"
require_approval = true
max_steps = 12
max_retries_per_step = 1
verification_required = true
parallel_execution = false
```

默认建议：

- `enabled = false`，实验分支或用户显式开启。
- `auto_trigger = "complex_only"`。
- `require_approval = true`。
- `parallel_execution = false`。

## 9. 接口与数据流设计

### 9.1 保留 `MemoryProvider` 四钩子为主边界

DivaGeneric 的所有 prompt 注入、召回、持久化和 session-end 节律都通过 `MemoryProvider` 或其组合 provider 接入。

推荐 provider composition：

```text
ContextBuilder / AgentLoop
  -> Arc<dyn MemoryProvider>
      -> GenericMemoryProvider
          -> FileMemoryProvider / MemoryManager
          -> LaputaProjection
          -> Mentle HybridMemoryProvider
          -> GenericCore
```

也可以先采用更小步方案：

```text
MemoryManager
  + GenericCore renderer
  + optional HybridMemoryProvider
```

但不能让 `ContextBuilder` 直接读 Generic private files，也不能让 `AgentLoop` 直接调用 Mentle schema 完成学习分类。

### 9.2 Generic Core 最小接口

后续建议新增：

```rust
pub trait GenericCore {
    fn classify_evidence(&self, evidence: EvidenceRef) -> Result<LearningDecision>;
    fn build_index_block(&self, request: IndexRequest) -> Result<GenericIndex>;
    fn propose_learning_candidate(
        &self,
        evidence: EvidenceRef,
    ) -> Result<Option<LearningCandidate>>;
    fn render_wakeup_block(&self, request: WakeupRenderRequest) -> Result<String>;
}
```

接口原则：

- 输入输出是 domain types。
- 不暴露 MCP schema。
- 不暴露 CLI args。
- 不暴露 Mentle backend concrete types。
- 不要求调用方知道底层是 Markdown、SQLite、Turso 还是文件索引。

### 9.3 推荐 public/domain types

后续新增或迁移的稳定类型：

```rust
pub enum MemoryLayer {
    L0Policy,
    L1Index,
    L2Fact,
    L3SopOrSkill,
    L4Evidence,
    LaputaPersona,
}

pub struct EvidenceRef {
    pub source: EvidenceSource,
    pub session_id: Option<String>,
    pub turn_id: Option<String>,
    pub path: Option<PathBuf>,
    pub excerpt_hash: Option<String>,
}

pub struct LearningCandidate {
    pub candidate_id: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub suggested_layer: MemoryLayer,
    pub content: String,
    pub confidence: f32,
    pub verification_state: VerificationState,
    pub status: CandidateStatus,
}

pub struct LearningDecision {
    pub decision_id: String,
    pub candidate_id: String,
    pub decision: DecisionKind,
    pub reason: Option<String>,
    pub target: Option<LearningTarget>,
    pub decided_at: String,
}

pub struct GenericIndex {
    pub rendered_markdown: String,
    pub pointers: Vec<IndexPointer>,
}
```

这些类型可以先放在 `agent-diva-generic`，等 manager/gui/provider 都需要共享时再上移到 `agent-diva-core`。

### 9.4 Config 建议

建议新增配置：

```toml
[generic]
enabled = true
index_max_lines = 30
learning_mode = "candidate_only"

[generic.autodream]
enabled = false

[generic.plan_mode]
enabled = false
auto_trigger = "complex_only"
require_approval = true

[mentle]
mode = "read_only"
```

`learning_mode` 建议枚举：

- `off`
- `candidate_only`
- `confirm_before_write`
- `policy_auto_low_risk`

默认建议：

- `generic.enabled = false` 或实验分支中显式 true
- `learning_mode = "candidate_only"`
- `generic.autodream.enabled = false`
- `generic.plan_mode.enabled = false`
- `mentle.mode = "read_only"`

### 9.5 Failure degrade 规则

所有 provider failure 必须 degrade：

- `system_prompt_block` 失败：渲染 degraded startup block，继续主对话。
- `prefetch` 失败：warn，跳过召回，继续主对话。
- `sync_turn` secondary backend 失败：Markdown 成功则视为主持久化成功。
- `on_session_end` 失败：记录失败，不回滚 session。
- Generic Core 分类失败：保留 evidence，不升级 candidate。
- Laputa wakeup 失败：回退 `SOUL.md` / `IDENTITY.md` / `USER.md` / `MEMORY.md`。
- Plan Mode 状态写入失败：停止进入下一阶段，保留 session 主对话可用。
- Plan verification 失败：标记 `FAIL` 或 `PARTIAL`，不自动声称完成。

## 10. 分阶段实施路线

### P0：文档固化与现状审计

目标：

- 固化本文件。
- 审计 `MemoryProvider`、`ContextBuilder`、`AgentLoop`、`ToolAssembly`、`MentleRuntime`、`HybridMemoryProvider`、`consolidation` 当前接缝。
- 标注哪些行为已经满足 DivaGeneric 基线。

产出：

- `docs/dev/genericagent/newedge/architecture.md`
- iteration logs

### P1：Generic Core 文件态模型和 policy

目标：

- 新增 `agent-diva-generic` crate。
- 定义 `GenericPolicy`、`MemoryLayer`、`EvidenceRef`、`LearningCandidate`、`LearningDecision`、`GenericIndex`。
- 实现文件态 candidate inbox 和 index renderer。

验收：

- 不改 AgentLoop 主流程也能单测 policy/classifier/index renderer。
- candidate 状态机覆盖 `inbox -> asked -> accepted|rejected -> archived`。

### P2：ContextBuilder 注入 L1 index + Laputa wakeup

目标：

- 通过 provider composition 注入 L1 index。
- Laputa wakeup/soul projection 仍通过 `MemoryProvider.system_prompt_block()` 进入 system prompt。
- `ContextBuilder` 只消费 compact rendered markdown。

验收：

- Generic/Laputa/Mentle 分别启用或关闭时 prompt block 正确。
- startup degraded 时仍回退 Markdown memory。

### P3：consolidation 改为候选生成

目标：

- `consolidation` 不直接重写长期记忆。
- LLM 输出转为 `LearningCandidate` 或 `SyncTurnRequest` 中的候选 evidence。
- 用户确认或 policy 决策后再写入 L2/L3/Laputa。

验收：

- consolidation 不绕过 `MemoryProvider`。
- failed candidate write 不影响 session pointer 前进策略。

### P4：daily autodream / rhythm

目标：

- 增加后台 daily worker。
- 输入 session/history evidence。
- 输出 daily rhythm、候选、index update、Laputa delta。

验收：

- daily rhythm 失败不破坏 session persistence。
- worker profile 与日常聊天 tool profile 分离。

### P5：Mentle 日常 4 工具与后台 full profile 分离

目标：

- 日常 profile 只暴露少量工具。
- 后台整理可用 full profile。
- `custom` 白名单作为高级配置。

验收：

- `off/read_only/custom/full` 工具白名单符合预期。
- Subagent registry 不暴露 Mentle 工具。
- prompt 只提及实际启用工具。

### P6：GUI / manager 配置面板补齐

目标：

- manager 暴露 Generic 状态、candidate inbox、decision API。
- GUI 展示学习候选、确认/拒绝/归档、Mentle mode、autodream 开关。

验收：

- GUI 不直接写 memory backend。
- 所有写入通过 manager/domain API。

### P7：Plan Mode 串行闭环

目标：

- 新增 `plan_mode` 模块。
- 实现 `.diva/plans/<plan-id>/` 文件结构。
- 实现 Explore -> Plan -> Execute -> Verify 串行状态机。
- 实现 approval gate、state persistence、verification verdict。

验收：

- 3-step 复杂任务可生成 plan draft、经确认后串行执行，并产出 evidence 与 `PASS/FAIL/PARTIAL`。
- 计划状态与 session message history 分离。
- 重启后可读取 `state.json` 并恢复到未完成 step。

### P8：高级心跳和 worker 委派

目标：

- heartbeat 根据时间、积压候选、session 活跃度触发 worker。
- worker 可委派 subagent，但必须使用受控 tool profile。

验收：

- 心跳失败不影响主 AgentLoop。
- worker 日志可审计。
- no full Mentle tools in normal chat。

## 11. 验收标准

本设计文档的验收标准：

- 能指导工程实现，无需实现者重新做架构判断。
- 每个设计点都有当前代码落点或明确新增模块位置。
- 保留 `MemoryProvider` 为唯一主记忆生命周期边界。
- Generic Core 不替代 AgentLoop。
- Laputa 不接管 Generic memory。
- Mentle 不成为强依赖。
- Mentle 全量工具不进入日常聊天上下文。
- 子代理默认不继承 Mentle 工具。
- 在线路径轻量，离线路径做批量整理。
- Plan Mode 是 Diva 原生能力，不是 Laputa 能力。
- Plan Mode 使用独立 `.diva/plans/*` 状态，不污染 session history，也不混入 `.laputa/`。
- Plan Mode 必须保留 approval gate 与 verification verdict。
- 所有失败路径都 degrade，不阻断主对话。

## 12. 后续实现测试要求

实现阶段至少覆盖：

- `ContextBuilder` 在 Generic/Laputa/Mentle 启用或关闭时生成正确 prompt block。
- `AgentLoop` prefetch failure 不阻断主回复。
- `consolidation` 只生成候选或通过 provider sync，不绕过 `MemoryProvider`。
- Subagent registry 不暴露 Mentle 工具。
- Mentle `read_only/custom/full/off` 工具白名单符合预期。
- Laputa startup degraded 时仍回退 Markdown memory。
- Daily rhythm 生成失败不破坏 session persistence。
- Plan Mode 复杂任务生成 `plan.md` 后等待审批，不直接执行。
- Plan Mode `PASS/FAIL/PARTIAL` verdict 与 `state.json` 一致。
- Plan Mode 重启后可恢复未完成 step。
- Provider native endpoint model ID 不被错误改写为 LiteLLM prefix。

## 13. 当前代码落点索引

| 设计点 | 当前落点 | 后续落点 |
|---|---|---|
| startup memory/wakeup 注入 | `agent-diva-core/src/memory/provider.rs`, `agent-diva-agent/src/context.rs` | provider composition + `agent-diva-generic` renderer |
| 在线召回 | `AgentLoop::process_inbound_message_inner`, `MemoryProvider::prefetch` | Generic L1 -> Mentle directed recall |
| turn 后持久化 | `MemoryProvider::sync_turn`, `consolidation.rs` | candidate inbox + confirmed write |
| session end rhythm | `MemoryProvider::on_session_end` | Laputa rhythm / autodream worker |
| Markdown 权威回退 | `MemoryManager`, `HybridMemoryProvider` | 保持 |
| Mentle runtime | `agent-diva-agent/src/mentle_runtime.rs` | 日常 profile / worker profile 分离 |
| Mentle 工具过滤 | `MentleToolRuntimeConfig` | 保持并补 GUI/manager |
| 子代理隔离 | `ToolAssembly::build_subagent_registry` | 保持 |
| SOUL 注入 | `ContextBuilder::append_soul_sections` | Laputa projection 兼容迁移 |
| Generic Core | 无 | 新增 `agent-diva-generic` |
| Plan Mode | 无 | 新增 `agent-diva-agent/src/plan_mode/` 与 `.diva/plans/*` |
