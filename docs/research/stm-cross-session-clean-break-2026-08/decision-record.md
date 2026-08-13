# STM 跨会话活动上下文 Clean Break 决策

- 状态：`Approved Boundary / Research Hold / Implementation Pending`
- 记录日期：2026-08-13
- 修订：`2026-08-14` 明确 AutoDream 必须批处理整理 STM（直写，不走提案）；同日冻结注入文件名 `ACTMEM.MD`，STM/LTM 仅作概念称呼
- 性质：产品与领域边界已拍板；分层装配和自动化算法待专项调研

## 问题判断

当前代码和 GUI 同时存在三套被错误混合的记忆概念：

1. `memory_md` 被注册为 Laputa `MemoryMd` section，在 Persona 左栏显示为
   Long-Term Memory，并被 Evolution、Notebook、Proposal、Prompt 投影继续读写；
2. BML typed SQLite/FTS5 已经是生产长期 Memory 的唯一权威；
3. `working_memory` 实现为 per-session volatile checkpoint，会话结束即物理删除，却在
   GUI 中以普通 BML record kind 出现。

这造成两个直接错误：文件型 `memory_md` 与 BML 构成双长期记忆权威；session checkpoint
又被误当成原设计中的 STM。GUI 虽然能筛选 `working_memory`，但没有展示一个统一的
跨会话活动上下文。

已有冻结的认知分区依据明确把 `05 MEMORY.MD` 定位为“STM 权威检查点（当前目标、开放
回路、下一步、证据指针）”，并把 Layer 1 定义为有界 STM bootstrap：
归档批次 `research/2026-07-08-laputa-memory-history/laputa-garden-cognitive-sync-2026-08/gap-and-migration-proposal.md`。
本决策恢复这一产品语义，但不提前沿用 `MEMORY.MD` 文件名或旧物理实现。

## 已确认边界

### S1：长期 Memory 只有 BML 一个权威

- `.laputa/memory.sqlite3` 的 typed BML records 是普通长期记忆的唯一生产权威。
- 长期事实、经验、关系性事实、历史素材及其检索、软删除、版本和来源均归 BML。
- Memory CRUD 不走审批，遵循已经冻结的 `MEMORY-APPROVAL-CLEAN-BREAK` 决策。
- 不再保留一份文件型、section 型或 Prompt 全量注入型长期记忆。

### S2：非兼容删除 `memory_md` 长期记忆链路

以下对象属于同一破坏性删除范围，不做兼容：

- `LaputaSectionName::MemoryMd` 及 `memory_md` section/file/API/DTO；
- `ProposalType::MemoryPatch -> MemoryMd` 路由及用户编辑 proposal；
- Persona 左栏 `long_term / memory_md`；
- Evolution 和 Notebook 创建 `memory_patch` 写 `memory_md` 的入口；
- `LaputaMemoryProvider` 把 `memory_md` 当 Applied Long-Term Memory 注入 Prompt；
- `MEMORY.md`/`memory/MEMORY.md` 到 `memory_md` 的迁移、导入、fallback 和测试夹具；
- 与 `memory_md` 绑定的 governance、changelog 和 compatibility surface。

删除前建立保护性分支；保护分支只保存历史，不成为 runtime fallback。不自动把旧
`memory_md` 导入 BML，也不提供双读、双写或隐式迁移。

### S3：STM 是统一的跨会话活动上下文

STM 不是长期记忆集合，也不是某一个聊天 session 的 transcript/summary。它是一个
workspace/profile 级、由 Agent 自动维护、可被人类查看和修正的当前活动工作集：

- 当前目标；
- 开放回路和未完成事项；
- 已确认的下一步；
- 当前有效约束与关键决定；
- 与当前工作相关的 Skill/SOP 引用；
- 指向 BML、WORLD、会话、工具 artifact 或外部证据的最小指针；
- 最近更新时间、来源和自动化状态。

新 session 必须能够继续看到同一个 STM 的有界投影，因此 STM 跨 session 持续；它不是
“结束会话即清空”的缓存。STM 内容应随着任务完成、失效、替代和预算压力自动收敛，不能
无限增长成第二个 LTM。

### S4：现有 session checkpoint 不等于 STM

- 当前 `WorkingMemoryRequest/CheckpointWriteRequest`、`update_working_checkpoint` 和
  `MemoryRecordKind::WorkingMemory` 实现的是 session-scoped scratch/checkpoint。
- 该临时层可以在未来作为 STM 自动更新的输入之一，但不能继续作为 STM 的权威、GUI
  对象或跨会话连续性证明。
- 后续实现必须在命名和类型上区分 `SessionCheckpoint` 与 `STM`；禁止用一个
  `working_memory` enum 同时表达两种生命周期。
- session transcript、compaction checkpoint 和 canonical checkpoint 继续属于各自
  Session/Context 领域，不迁入 STM，也不因 STM 引入而合并。

### S5：STM 自动管理且不走审批

- Agent 的日常 STM 更新是运行时上下文维护，不创建 Proposal，不进入 Evolution、
  Governance Ledger 或聊天页 Approval Center。
- **AutoDream 必须修改和整理 STM**（批处理收敛：完成、失效、替代、预算）。这是
  与聊天 Agent 同一对象上的第二条触发，同样直写、不走提案。不是 `MemoryPatch`，
  也不是人格提案。算法与物理权威仍属下方 Research Hold。
- 用户在 STM 工作区的修正直接生效；误操作保护依赖明确版本、历史/撤销或可恢复删除，
  不依赖审批。
- STM 不能直接把推测升级为 BML 长期权威。任何长期沉淀仍须走 BML 的证据、来源和 CRUD
  边界；STM 到 BML 的具体晋升策略留待调研。
- 2026-08-14 独立测试：当前 `agent-diva-autodream` 零 STM 符号，整理路径测不到。

### S6：STM 管理入口归 Memory 页面

Persona 页面只管理 Laputa 权威 Markdown（`IDENTITY.MD` / `RELATIONSHIP.MD` /
`REDLINE.MD` / `USER.MD` / `DREAM.MD` / `DARK.MD` / `WORLD.MD`）及其内容审查/历史，
不显示 STM 或 `memory_md`。

Memory 页面右上角增加一个专用、可识别状态的入口，例如：

```text
[ STM · 3 个开放事项 ]  [ 刷新 ]
```

点击后进入 Memory 模块内部的完整 STM 工作区，不使用狭小 tooltip/popover，也不把 STM
继续伪装成一个 `working_memory` 筛选值。建议工作区第一版展示：

- 当前关注目标；
- 开放回路；
- 下一步行动；
- 当前约束和关键决定；
- 关联 Skill/SOP；
- 证据指针和来源会话；
- 最近自动更新时间；
- 容量/预算状态；
- STM 更新历史。

用户动作限定为查看来源、编辑/修正、完成开放事项、移除失效事项和查看历史。每个动作
必须真实修改 STM，不产生 Approval。普通长期记忆仍使用 BML 列表/详情工作区。

### S7：概念叫 STM/LTM，核心文件叫 `ACTMEM.MD`

- **STM / LTM 只是概念称呼**，因为短时活动 vs 长期事实好记、好分层。决策正文和口头可以继续这么说。
- **注入权威文件名冻结为 `ACTMEM.MD`**（activity memory）。不是 Persona 七文件，不是 BML，不是 Laputa section。
- **禁止**核心文件使用 `STM.MD`、`STMEM.MD`、`MEMORY.MD`。`STM` 来自远古 UPSP 调研叫法，要撇清，不进核心文件名。
- 旧 Garden `MEMORY.MD` / `05 MEMORY.MD` 全舍弃，不继承文件名。
- 本条冻文件名与称呼。落点与时效见 **S8**。

### S8：`ACTMEM.MD` 是全局一份文件；发言即写；空闲 10 分钟写胶囊

- **就是一个 Markdown 文件**，不是库、不是 BML 表。路径建议 `{config_dir}/actmem/ACTMEM.MD`，不进项目 `.laputa/`，不进 `memory.sqlite3`。
- **所有项目共用这一份**（跨 workspace / 跨项目全局）。胶囊分文件时可在文件名里带 workspace，以免 session 撞名。
- **用户一发言就写 Pulse 节。** 空闲 **10 分钟** 写该会话胶囊。
- Pulse：**原文可进，全文不进**（短用户原话保留；整段对话和助手全文不进 Pulse）。
- 字数顶：Pulse 1600、Work 1600、单胶囊 800。
- **子代理不进** ACTMEM，也不进其它 Laputa 人格/记忆生态。子代理上下文由主 Agent 装配（面具仍进）。本阶段不设计。
- **第一版不做 STM→BML 自动晋升**；晋升以后另议。
- **整份 ACTMEM（Pulse、Work、胶囊）走工具车道。** 不进 Frozen Core，不动态装配进
  Prompt。自动装配会把事情搞复杂，先用工具试。
- **日常只常驻一个查询工具。** 走现成 C4/C5e：**CORE** 里只有 `actmem`（读 Pulse /
  Work / 胶囊 / 目录，有界）。这是工具 schema 常驻，**不是**把 ACTMEM 正文装配进 Prompt。
- **管理工具要做，必须 DEFER。** 整理、改 Work、删条、折叠胶囊等经 `tool_search`
  发现后才挂上，不得进 CORE、不得撑稳定前缀。具体管理工具名单 D2 再列，不得先做成
  BML 那种一串常驻 CRUD。
- 系统自动写（发言 Pulse、空闲胶囊、AutoDream）不经过聊天工具。用户 Memory 页仍直改。

## 分层与装配 Research Hold

以下问题必须经过专项调研和测试后再拍板，本次不得由实现者自行猜测：

- STM 的物理权威采用 Markdown、typed SQLite record、独立表还是其他结构；
- profile/workspace/agent 的准确 scope，以及多 channel、多并发 session 的写入合并；
- 自动更新触发时机、输入来源、Action-Verified 门槛和失败恢复；
- 目标、开放回路、下一步、约束和证据指针的 schema；
- 容量预算、优先级、到期、完成、替代、压缩与淘汰规则；
- STM 历史保留、撤销和 no-op 语义；
- Layer 1 在 stable prefix、per-turn context、自动/响应式 compact 前后的装配位置；
- 与 Frozen Core、WORLD、BML Recall、Session checkpoint、Plan、Skills/SOP 的优先级；
- 新 session、session resume、subagent、cron/background job、崩溃恢复和多进程一致性；
- STM 到 BML/Skill 的晋升条件，以及如何证明不会把易变状态固化为长期权威；
- GUI 的最终信息结构、响应式退化、历史可视化和自动更新反馈。

专项调研必须同时盘点当前 C1–C5 Context Assembly、BML typed provider、session lifecycle、
canonical checkpoint、Plan/Background Task、GUI MemoryView 与 GenericAgent/Garden 的最新
设计，然后输出单一权威模型、状态机、失败模型、装配顺序、不变量和 E2E 测试矩阵。

## 最低测试与删除证明要求

- 删除 `MemoryMd`/`memory_md` 后全仓符号、路由、GUI、Prompt、测试夹具和迁移扫描为零；
- BML 继续承担普通长期记忆 CRUD/检索且不出现第二权威；
- STM 跨新 session、重启/resume 和至少两个 channel session 保持一致活动工作集；
- session checkpoint 结束清理不删除 STM；STM 清理不删除 BML 或 transcript；
- STM 有界投影，不整体注入历史、BML、WORLD 或 transcript；
- 自动更新失败保留上一份可用 STM，过期响应不得覆盖新 revision；
- 用户修正直接生效且不创建 Approval/Proposal/Governance；
- Memory 页存在唯一 STM 入口和完整工作区，Persona/Evolution/Notebook 不再暴露
  `memory_md`；
- workspace Rust gates、GUI tests/build 和真实桌面跨会话 smoke 全部通过。

## 被取代与保留的依据

- `gap-and-migration-proposal.md` 中“STM 是 Layer 1 活动工作集”的产品边界保留；
  `05 MEMORY.MD` 的具体文件名和物理实现不自动继承。
- `v0.0.6-wave2-layers-working-memory` 保留为 session checkpoint 实现证据，但它不能再
  证明跨会话 STM 已实现。
- `memory-write-paths-contract.md` 中“Working memory = session-scoped volatile”的对象应在
  后续重命名为 SessionCheckpoint，不再与 STM 同名。
- 归档的 `laputa-memory-final-architecture.md` 中 working memory 与 long-term memory 分离、BML
  typed authority 和有界指针原则继续有效。

本记录不授权立即修改分层装配代码。2026-08-14 另有一份**未批准**完整分层提案：
[`stm-layering-proposal.md`](./stm-layering-proposal.md)。用户评审前不得当架构合同，
也不得施工。

## 分层提案指针（未批准）

用户若采纳 `stm-layering-proposal.md`，再回头修订本文件的 Research Hold 条目。
在那之前：物理权威、装配位置、表述、触发算法仍按上方 Hold。
