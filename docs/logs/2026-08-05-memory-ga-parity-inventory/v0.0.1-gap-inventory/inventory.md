# GenericAgent 对齐 — Memory / Laputa / AutoDream 完整残缺项盘点

> Status: Accepted inventory (code-free delivery)  
> Date: 2026-08-05  
> Scope: Memory Layer × Laputa Governance × AutoDream  
> Primary reference: `.workspace/GenericAgent`  
> Secondary references: Hermes memory tool (对比用), 现有 ADR `docs/architecture/laputa-memory-final-architecture.md`, `docs/architecture/memory-framework-interfaces.md`, `docs/architecture/autodream-architecture-2026-06-12.md`

---

## 0. 执行摘要

### 0.1 一句话结论

agent-diva **已有**记忆/治理/AutoDream 的**底座与治理骨架**，但相对 GenericAgent 的“**Agent 可直接管理记忆、分层可导航、任务结算可蒸馏、工作记忆可检查点**”产品闭环，**关键用户面与若干联通缝合点仍残缺**。  
在用户说“请记住 X”时，系统**没有一等公民记忆管理工具**；Laputa/Typed 路径下回合同步多为 **pending proposal**，不等于已生效权威。

### 0.2 目标定义：何谓“功能性完全对齐”

对齐 **不是** 源码移植 GA，而是下列能力在 Diva 架构内**行为等价可用**：

| 能力域 | 对齐定义（用户可观察） |
|--------|------------------------|
| A. 即时记忆管理 | Agent 可 add/list/search/update/delete（或等价动作），“记住/忘记”有确定结果 |
| B. 分层记忆 | 顶层极小常驻 + 深层按需加载；有管理 SOP / 清理纪律 |
| C. 工作记忆 | 任务内 checkpoint / key_info 与长期记忆分离 |
| D. 主动蒸馏 | 任务结束可触发 long-term update；后台节律蒸馏（AutoDream）可用 |
| E. 启动注入 | 会话启动可见稳定长期记忆投影 |
| F. 中回合召回 | 相关问题时能注入 recall 结果 |
| G. Laputa 治理 | 提案→审→apply→changelog→rollback 全链路可用，且与记忆写入一致 |
| H. AutoDream | 手动/自动触发→输入→反思→候选提案→报告 可用，并进入 G |
| I. 提示与工具一致 | system prompt 不承诺不存在的工具；状态文案不谎称已持久 |

**Diva 差异保留（不可丢）：**

- 权威不由模型任意 overwrite 文件；写权威走 Laputa / typed store 治理。
- AutoDream 只产证据与提案，不直接成为 authority。
- 审批中心（command/plan/memory）统一 HITL。

### 0.3 状态图例

| 标记 | 含义 |
|------|------|
| ✅ 已具备 | 代码与主路径可用（可能缺 polish） |
| 🟡 半成品 | 有实现但未接通 / 语义偏差 / 仅测试可用 |
| ❌ 缺失 | 无产品路径或无 agent 可达入口 |
| ⚠ 矛盾 | 文档/提示与实现冲突 |
| 🔒 有意不做 | 架构明确拒绝（需用 Diva 等价物替代） |

---

## 1. 对照基线：GenericAgent 记忆与节律能力清单

来源：`.workspace/GenericAgent`（`ga.py`, `memory/*`, `reflect/*`）。

### 1.1 分层记忆（L0–L4）

| GA 层 | 载体 | 职责 | Agent 如何用 |
|-------|------|------|--------------|
| L0 | `memory/memory_management_sop.md` | 写什么/不写什么/怎么 patch | `start_long_term_update` 注入 |
| L1 | `global_mem_insight.txt` | ≤30 行存在性索引 + RULES | 启动/周期性注入 |
| L2 | `global_mem.txt` | 环境事实库 | file_read / file_patch |
| L3 | `memory/*_sop.md` / 脚本 | 任务级精简经验 | 按需 file_read |
| L4 | `memory/L4_raw_sessions/` | 原始会话归档 + salient mining | scheduler 静默 + 脚本 |

配套 SOP：`memory_cleanup_sop.md`（L1 瘦身/ROI）、`salient_mining_sop.md`（历史重点挖掘）。

### 1.2 Agent 工具面（记忆相关）

| 工具 | 作用 |
|------|------|
| `do_start_long_term_update` | 开启长期记忆结算：注入 L0 + 全局记忆，要求最小 patch |
| `do_update_working_checkpoint` | 更新 `key_info` / `related_sop` 工作记忆 |
| `do_file_read` / `do_file_patch` / `do_file_write` | 实际读写记忆文件 |
| 通用 file 工具 + 路径约定 | 记忆即 workspace 文件树 |

### 1.3 工作记忆与节奏

- 每轮/隔轮注入 `### [WORKING MEMORY]`（history fold + key_info）
- 每 7 轮提示 checkpoint；每 10 轮 re-inject global memory；每 25 轮要求写文件 checkpoint
- 回复强制 `<summary>` 进入 long working history

### 1.4 后台节律（GA reflect）

- `reflect/scheduler.py`：定时任务 + **每 12h L4 archive**
- 自治 SOP：`autonomous_operation_sop`、goal hive 等（超出最小记忆闭环，记入扩展项）

### 1.5 核心公理（对齐时必须保留精神）

1. **Action-Verified Only** — 无行动验证不写长期记忆  
2. **Sanctity of Verified Data** — 验证过的事实禁静默丢弃  
3. **No Volatile State** — 禁易变状态  
4. **Minimum Sufficient Pointer** — 上层只留最短指针  

---

## 2. agent-diva 现状快照（证据）

### 2.1 记忆层代码地图

| 组件 | 路径 | 现状 |
|------|------|------|
| `MemoryProvider` trait | `agent-diva-core/src/memory/provider.rs` | ✅ 生命周期契约：startup / prefetch / sync_turn / session_end |
| `MemoryManager` | `agent-diva-core/src/memory/manager.rs` | ✅ legacy 读写 MEMORY.md/HISTORY.md；prefetch 直接 Failed |
| `MemoryRecord` typed 模型 | `agent-diva-core/src/memory/record.rs` | ✅ 类型齐全（kind/trust/tombstone/provenance） |
| Consolidation | `agent-diva-agent/src/consolidation.rs` | 🟡 内部 `save_memory` schema，**不暴露给主 agent**；默认 ≥100 消息 |
| ToolAssembly | `agent-diva-agent/src/tool_assembly.rs` | ❌ **无 memory 工具**；有 file/shell/web/cron/plan |
| agent-diva-tools | `agent-diva-tools/src/*` | ❌ 无 memory 模块 |
| Prompt 记忆指引 | `agent-diva-agent/src/context.rs` | ⚠ 声称 “available memory tools”，工具不存在 |
| memory_boundary | `agent-diva-agent/src/memory_boundary.rs` | ✅ mode 选择 legacy/shadow/typed + degraded |
| Config | `agent-diva-core/src/config/schema.rs` | 🟡 缺 `memory` 段 → **Legacy**；`Default` → Typed（行为取决于配置） |

### 2.2 Laputa 代码地图

| 组件 | 路径 | 现状 |
|------|------|------|
| file-first service | `agent-diva-laputa/src/service.rs` | ✅ proposal CRUD / apply / sections |
| `LaputaMemoryProvider` | `memory_provider.rs` | 🟡 读 applied；`sync_turn` **只建 pending proposal**；prefetch 空操作 |
| Typed store | `typed_store.rs` | ✅ SQLite + FTS 骨架 |
| Typed provider | `typed_provider.rs` | 🟡 启动渲染 applied typed；prefetch shadow recall；**sync_turn 仍走 proposal_sink** |
| Governed apply | `governed_apply.rs` | ✅ 治理 apply / receipt 链路存在 |
| Migration | `migration.rs` / `memory_records.rs` | ✅ offline import 能力 |
| Manager API | `agent-diva-manager` laputa handlers | ✅ 快照/提案/apply 等 |
| GUI PersonaMemory | `PersonaMemoryView.vue` 等 | 🟡 人审/编辑面存在；非 agent 自管 |

### 2.3 AutoDream 代码地图

| 组件 | 路径 | 现状 |
|------|------|------|
| crate | `agent-diva-autodream` | ✅ service/worker/inputs/reflection/outputs/reports |
| 提案输出 | `outputs.rs` → Laputa | ✅ 候选写入 Laputa proposals |
| Manager routes | `/api/autodream/runs*` | ✅ 手动触发/查询/取消 |
| GUI | locales + desktop API | 🟡 监控/触发 UI 文案存在；完整验收未闭合 |
| 自动阈值触发 | checkpoint `session_threshold_enabled` | 🟡 字段存在，产品自动触发联通需核对 |
| 与 MemoryProvider 关系 | — | 🟡 AutoDream 走 Laputa proposal，**不经 agent memory tools** |
| TODOLIST | GMH-52 全量验收等 | 🟡 仍有开放验收项 |

### 2.4 已有架构立场（不可违背）

来自 `laputa-memory-final-architecture.md` / `memory-framework-interfaces.md`：

- GenericAgent = **设计参考**，非运行时依赖  
- AgentLoop **不能**直接写 authority  
- 写权威：evidence → proposal → governed apply → applied  
- AutoDream **永不**直接成为 authority  
- Typed SQLite 是 profile-local 权威存储方向；legacy Markdown 为兼容/导入  

→ 对齐 GA 功能时，**必须**用 Diva 等价物（memory tools + Laputa），禁止退回“模型随意 overwrite MEMORY.md 即权威”。

---

## 3. 完整残缺项盘点（主表）

### 3.1 域 A — Agent 记忆管理工具面（相对 GA 最大缺口）

| ID | 残缺项 | GA 对照 | Diva 现状 | 状态 | 影响 | 目标等价实现（方向，不写码） |
|----|--------|---------|-----------|------|------|------------------------------|
| A1 | 显式长期记忆结算入口 | `start_long_term_update` | 无工具；仅 consolidation 内部 | ❌ | “任务做完该记什么”无法主动触发 | `memory_distill` / `start_long_term_update` 工具：注入管理策略 → 产出 patch/proposal |
| A2 | 条目级 add | memory add / file_patch 追加 | 无 | ❌ | “请记住 X”无确定落点 | `memory` tool action=add → provider API |
| A3 | 条目级 replace/update | file_patch 最小改 | 无；legacy 整文件覆盖 | ❌ | 无法局部修正 | action=replace/update + CAS/revision |
| A4 | 条目级 remove/forget | 删除/patch | 无；typed 有 tombstone 模型但 agent 不可达 | 🟡/❌ | “忘掉 X”不可用 | action=remove → tombstone proposal/apply |
| A5 | list / read 当前记忆 | file_read L1/L2 | 无专用；可读文件但不保证权威 | ❌ | Agent 无法自查“我记得什么” | action=list/read 读 applied 投影 |
| A6 | search / recall 工具 | file_read + 关键词 | prefetch 半自动；无 tool | 🟡 | 用户问“你还记得…”不稳定 | `memory_search` 或 prefetch 强化 + tool 可调用 |
| A7 | 工具注册进 ToolAssembly | GA tools schema | 未注册 | ❌ | 模型调不到 | tools crate + assembly + mask 策略 |
| A8 | 提示词与工具一致 | GA 工具描述即行为 | ⚠ 假承诺 memory tools | ⚠ | 幻觉式“已记住” | 有工具才写指引；无则诚实降级文案 |
| A9 | 写结果诚实语义 | 文件写即成功 | `sync_turn` 在 Laputa 下 “Persisted”=提案创建 | ⚠ | 假成功 | 区分 `PersistedAuthority` / `ProposalCreated` / `Failed` |

**优先级：A1–A9 = P0（没有它们就谈不上“完全对齐”）。**

---

### 3.2 域 B — 分层记忆模型（GA L0–L4）

| ID | 残缺项 | GA 对照 | Diva 现状 | 状态 | 目标等价 |
|----|--------|---------|-----------|------|----------|
| B1 | L0 记忆管理 SOP 运行时注入 | `memory_management_sop.md` | 无运行时 L0 注入 | ❌ | 内置/可配置 Memory Management Policy（可存 Laputa 或 skills） |
| B2 | L1 极简索引常驻（≤N 行） | `global_mem_insight.txt` | 启动注入整块 applied / MEMORY 全文，无严格 L1 预算 | 🟡 | Startup 只注入 L1-like 索引 + RULES；细节 pointer |
| B3 | L2 事实库 | `global_mem.txt` | typed LongTerm/Preference 等 kind 有模型；无 L2 产品形态 | 🟡 | Preference/env facts 分区 + 管理策略 |
| B4 | L3 任务 SOP 库按需读 | `memory/*_sop.md` | skills 系统存在但≠记忆 SOP 库；无“记经验→SOP”闭环 | 🟡/❌ | Learning/SOP records 或 skills 固化路径 |
| B5 | L4 原始会话归档 | L4_raw_sessions + 12h cron | session store 有；无 GA 式 L4 压缩归档+salient mining | ❌ | session archive job + 供 AutoDream 输入 |
| B6 | L1↔L2/L3 同步纪律 | cleanup SOP | 无 | ❌ | 蒸馏/apply 后更新索引指针 |
| B7 | 记忆清理/GC（ROI 压缩） | `memory_cleanup_sop.md` | 无 agent 可达 GC；无 L1 瘦身 | ❌ | AutoDream/cron + 治理提案做压缩 |
| B8 | 禁止易变状态写入策略 | 公理 3 | 无强制校验 | ❌ | 写入校验器（内容分类 + 拒绝 volatile） |
| B9 | Action-Verified 写入策略 | 公理 1 | 无 tool-result 绑定强制 | ❌ | proposal evidence 必须挂 tool/session 证据 |
| B10 | 最小指针原则渲染 | 公理 4 | 常全文塞 prompt | 🟡 | Recall budget + pointer-first 渲染 |

**优先级：B1/B2/B9/B10 = P0；B3–B8 = P1。**

---

### 3.3 域 C — 工作记忆（Working Memory）

| ID | 残缺项 | GA 对照 | Diva 现状 | 状态 | 目标等价 |
|----|--------|---------|-----------|------|----------|
| C1 | `update_working_checkpoint` | 有 | 无（`update_plan` 是 TODO 清单，不是 working memory） | ❌ | session-scoped working state store + tool |
| C2 | WORKING MEMORY 注入 | 每轮/隔轮 | 无专用块；仅有 session messages + 偶发 prefetch | ❌ | turn 组装注入 checkpoint |
| C3 | key_info / related_sop 字段 | 有 | 无 | ❌ | 结构化 working fields |
| C4 | 周期提示（7/10/25 轮） | 有 | 无 | 🟡 | 可选 rhythm hints（可与 AutoDream 阈值共享） |
| C5 | `<summary>` 进 working history | 有 | 无 | 🟡 | 可选 turn summary 管道（注意与 compaction 边界） |
| C6 | 工作记忆≠长期权威 | 明确分离 | 概念上有 session vs long-term，但无工具边界 | 🟡 | 写路径分流：working 即时；long-term 经治理 |

**优先级：C1–C3/C6 = P0；C4–C5 = P2。**

---

### 3.4 域 D — 回合中召回 / Prefetch

| ID | 残缺项 | GA 对照 | Diva 现状 | 状态 |
|----|--------|---------|-----------|------|
| D1 | 意图门控召回 | 模型主动 file_read | `derive_prefetch_intent` 有；非问题类常跳过 | 🟡 |
| D2 | Legacy prefetch 可用 | 读文件 | MemoryManager prefetch = Failed | ❌ |
| D3 | Laputa file-first prefetch | — | 返回 Skipped/空，无检索 | ❌ |
| D4 | Typed FTS recall | — | shadow recall 有路径；是否生产注入需配置 | 🟡 |
| D5 | 召回结果信任标注 | — | 架构要求 labelled；实现完整度不一 | 🟡 |
| D6 | 召回反馈闭环 | GA 弱 | `record_recall_outcome` + feedback store 有骨架 | 🟡 |
| D7 | 用户显式 “recall X” 工具 | 模型自决 | 无 | ❌ |

**优先级：D2/D3/D4 生产可用 + D7 = P0；D5/D6 = P1。**

---

### 3.5 域 E — 巩固 / 压缩 / 晋升

| ID | 残缺项 | GA 对照 | Diva 现状 | 状态 |
|----|--------|---------|-----------|------|
| E1 | 主动任务结算蒸馏 | start_long_term_update | 无 | ❌ |
| E2 | 被动长对话 consolidation | 部分靠 L4/人工 | ≥100 messages 批处理 | 🟡 门槛高且非条目管理 |
| E3 | 巩固质量门 | SOP 纪律 | QualityGate 有 | ✅/🟡 |
| E4 | consolidation → Laputa 语义 | file patch | sync_turn → proposal 或 overwrite | ⚠ 模式分叉 |
| E5 | 巩固结果条目化 | 局部 patch | 常整段 memory_update | 🟡 |
| E6 | 与 AutoDream 边界清晰 | 反射/归档分工 | 文档有；运行边界易混 | 🟡 |

**优先级：E1/E4/E5 = P0。**

---

### 3.6 域 F — Laputa 治理层“完全可用”

| ID | 残缺项 | 目标“完全可用” | 现状 | 状态 |
|----|--------|----------------|------|------|
| F1 | 唯一写权威入口 | 所有记忆晋升经 proposal/apply 或 typed governed write | 多入口（legacy 直写 / proposal / GUI write_section） | 🟡 |
| F2 | Agent 写 → proposal 自动创建 | 工具调用可建 proposal + evidence | 无 agent tool；仅 sync_turn/autodream | ❌ |
| F3 | 人审闭环 | GUI/CLI 审批 memory 域 | GMH-30..33 大体完成；M3/GMH-52 验收开放 | 🟡 |
| F4 | Apply 后进 prompt | 下次启动/刷新可见 | applied 可读；同会话热更新策略未定 | 🟡 |
| F5 | Apply 同步 typed store | file-first 与 typed 一致 | 架构要求 clean-break；双轨风险 | 🟡 |
| F6 | Rollback | changelog rollback | API 存在 | 🟡 需验收 |
| F7 | Tombstone 非物理删 | 有 | 模型有；agent 路径无 | 🟡 |
| F8 | High-risk memory 需 evidence | 有策略倾向 | GUI 有 high-risk 限制 | 🟡 |
| F9 | 降级诚实 | open 失败 degraded 不静默 fallback | DegradedMemoryProvider 有 | ✅/🟡 |
| F10 | authority_mode 默认一致 | 配置语义统一 | missing→Legacy vs Default→Typed 分叉 | ⚠ |
| F11 | 14 section 投影策略 | 身份/关系/承诺/偏好/memory/history… | 部分 section 渲染；TBD pool 历史设计 | 🟡 |
| F12 | Agent 可读 pending vs applied 边界 | 默认不注入 pending | prompt 有原则；工具侧无“查 pending” | 🟡 |

**优先级：F2/F4/F5/F10 = P0；其余 P1 验收。**

---

### 3.7 域 G — AutoDream 特色“完全可用”

| ID | 残缺项 | GA 对照 | 目标 | 现状 | 状态 |
|----|--------|---------|------|------|------|
| G1 | 手动触发 | scheduler 任务 | 一键跑通 | Manager API + GUI 有 | 🟡 需端到端验收 |
| G2 | 自动阈值触发 | scheduler + 阈值 | session/消息阈值可靠触发 | checkpoint 字段有；联通待证 | 🟡/❌ |
| G3 | 输入收集多源 | L4/session/history | session+history+authority+feedback | inputs 模块有 | 🟡 |
| G4 | 反思引擎质量 | salient mining 精神 | 可产高价值候选 | Deterministic + 可插 LLM | 🟡 |
| G5 | 候选 → Laputa proposal | — | 必达 | outputs 有 | ✅/🟡 |
| G6 | 候选审查 UI | — | 接受/改/拒 | 与 Evolution/审批中心关系需闭合 | 🟡 |
| G7 | 日报/周报/月报 | 节律报告 | 可生成可展示 | reports/monthly 有 | 🟡 |
| G8 | 不阻塞主循环 | 独立线程/任务 | 不卡 chat | worker 设计有 | 🟡 需压测 |
| G9 | 失败可观测 | log | 状态/错误码/事件 | metrics/events 有 | 🟡 |
| G10 | 与 agent 即时记忆分工 | 即时 vs 节律 | 文档化+实现 | 边界未产品化 | ❌ |
| G11 | L4/历史挖掘等价 | salient_mining_sop | 情绪/活动/消失事项 | 无对等 SOP 作业 | ❌ |
| G12 | 蒸馏公理（验证后才记） | Action-Verified | CandidateGate 有规则 | 有 candidates gate | 🟡 需对齐公理 |

**优先级：G1–G6/G10 = P0 可用；G7–G9/G12 = P1；G11 = P2 增强。**

---

### 3.8 域 H — 跨层联通（最容易“半成品假完成”）

| ID | 缝合点 | 残缺描述 | 状态 |
|----|--------|----------|------|
| H1 | Agent tool → MemoryProvider → Laputa | 缺 tool，链路断在第一环 | ❌ |
| H2 | sync_turn 语义 → 用户可见状态 | Persisted≠applied | ⚠ |
| H3 | Apply → Typed store → 下次 prefetch | 需证明 apply 后 FTS/startup 一致 | 🟡 |
| H4 | AutoDream proposal → 审批中心 → apply | 需一条龙验收 | 🟡 |
| H5 | Consolidation vs AutoDream 双写冲突策略 | 谁先谁覆盖未产品化 | 🟡 |
| H6 | Working memory → 晋升 long-term | 无显式晋升工具 | ❌ |
| H7 | GUI PersonaMemory ↔ Typed authority | 编辑路径与 typed 一致性 | 🟡 |
| H8 | CLI headless 记忆审批 | 有部分；完整策略需验收 | 🟡 |
| H9 | 默认配置出箱即 Typed+可用 | 缺配置走 Legacy 的陷阱 | ⚠ |
| H10 | 提示/文案/i18n 与真实能力 | GUI 已写“已连通待最终验收” | 🟡 |

---

### 3.9 域 I — 非目标 / 有意不做（用等价物替代）

| ID | GA 能力 | Diva 立场 | 等价物 |
|----|---------|-----------|--------|
| I1 | 模型 file_write 即权威 | 🔒 禁止 | Laputa apply / typed governed write |
| I2 | 无审批任意改身份 | 🔒 禁止 | high-risk memory approval |
| I3 | 源码级 SOP 脚本树原样拷贝 | 🔒 不移植 | 内置 policy + skills + records |
| I4 | GA 全量 computer-use SOP 库 | 超出记忆对齐范围 | 不纳入本盘点 P0 |
| I5 | Goal Hive / 全自治运营 | 扩展域 | 后续 epic，非记忆闭环 P0 |

---

## 4. 用户旅程缺口（可验收场景）

| 旅程 | 期望 | 当前 | 缺口 ID |
|------|------|------|---------|
| U1 用户：“记住我叫花叔” | 立即 durable 或明确“已提交审批” | 口头/乱写文件/无反馈 | A2 A8 A9 F2 |
| U2 用户：“你还记得我叫什么？” | 召回或读权威 | 依赖会话上下文；跨会话不稳 | D4 D7 B2 |
| U3 用户：“忘掉昨天的临时路径” | tombstone/删除 | 无 | A4 F7 |
| U4 任务完成后 Agent 自结算经验 | 蒸馏工具 + 最小 patch | 无；等 100 条消息 | A1 E1 |
| U5 长任务中途不丢关键上下文 | working checkpoint | 无 | C1 C2 |
| U6 夜间/节律自动挖记忆 | AutoDream 跑完出提案 | 手动可有；自动与审查闭合待证 | G1–G6 |
| U7 用户在 GUI 批准提案 | 下次对话权威已变 | 需端到端证明 | F3 F4 H3 H4 |
| U8 用户查看“我的记忆” | PersonaMemory 真实非空且可编辑 | 有视图；空/未初始化/与 typed 不一致风险 | H7 F11 |

---

## 5. 相对 GA 的“功能对齐矩阵”（汇总）

| GA 能力簇 | 对齐度 | 说明 |
|-----------|--------|------|
| L0 管理 SOP | ~0% | 无运行时注入 |
| L1 索引注入 | ~20% | 有 startup 块，无 L1 纪律 |
| L2 事实库 | ~30% | 有存储模型，无管理 UX/工具 |
| L3 经验 SOP | ~20% | skills≠记忆 SOP 沉淀 |
| L4 归档挖掘 | ~15% | session 有，归档/挖掘无 |
| Working memory | ~10% | 仅会话消息 |
| 主动蒸馏工具 | 0% | 缺失 |
| 文件级自管记忆 | 有意不做 | 用治理工具替代 |
| 后台节律 | ~50% | AutoDream 半成品可用 |
| 治理（Diva 特色） | ~70% | 超 GA，但与 agent 写入口未缝合 |

**综合：底座 40–60%，用户可感知记忆管理 <20%，三件套联通未达“完全可用”。**

---

## 6. 建议落地波次（仅规划，本阶段不实施）

### Wave 0 — 诚实与契约（文档/配置/语义）

- 修 prompt 假承诺（A8）
- 统一 `authority_mode` 默认与缺失配置语义（F10）
- `sync_turn` 状态机诚实化设计（A9/H2）
- 写清 Working / Long-term / AutoDream 分工（G10/H5/C6）

### Wave 1 — Agent 记忆工具 P0（功能对齐核心）

- 实现 memory tool 面：add/list/search/update/remove + distill（A1–A7）
- 接到 MemoryProvider 扩展 API（不只 4 lifecycle hooks）
- Legacy：可直写 MEMORY 或建 proposal（产品二选一，推荐统一 proposal-first）
- Typed/Laputa：默认 proposal + 可选 auto-apply 低风险策略

### Wave 2 — 分层与工作记忆

- L1 预算注入 + pointer（B2/B10）
- Working checkpoint 工具与注入（C1–C3）
- L0 policy 文本（B1/B8/B9）

### Wave 3 — Laputa 完全可用缝合

- Apply → typed 一致性（F5/H3）
- GUI/CLI 审批与 agent 提案同源（F3/H4）
- Rollback/tombstone 验收（F6/F7）

### Wave 4 — AutoDream 完全可用

- 自动触发可靠（G2）
- 候选审查闭合（G6）
- 报告可见（G7）
- 与 Wave1 工具边界清晰（G10）
- 可选 L4/salient 等价（G11）

### Wave 5 — 巩固与清理

- consolidation 条目化 + 与 proposal 对齐（E4/E5）
- cleanup/GC 提案（B7）
- 全量验收 GMH-52 类清单并入

---

## 7. 验收清单（文档 `acceptance.md` 骨架）

### 7.1 Memory Agent 面

- [ ] 用户说“记住 X”，工具调用成功，返回明确状态（applied 或 pending+id）
- [ ] 新会话（或 refresh 策略定义后）能读到 X
- [ ] “忘记 X”后默认 prompt 不再出现 X（tombstone）
- [ ] list/search 与权威一致
- [ ] 无工具时不在 system prompt 承诺工具

### 7.2 Laputa

- [ ] 任意 agent/AutoDream 记忆变更可在提案列表看到
- [ ] 批准后 section/typed 一致
- [ ] 拒绝后不进 authority
- [ ] rollback 可恢复
- [ ] degraded 模式不静默写 legacy 冒充权威

### 7.3 AutoDream

- [ ] 手动触发完整 run → ≥0 提案或明确 noop 原因
- [ ] 自动阈值在配置阈值下触发
- [ ] 失败有错误码与事件
- [ ] 报告可打开
- [ ] 不阻塞主聊天

### 7.4 GA 分层精神

- [ ] 启动注入有硬预算（L1-like）
- [ ] 细节靠 search/read 按需
- [ ] 写入需证据/验证策略可观测
- [ ] working memory 不污染 long-term

---

## 8. 证据索引（写 `verification.md` 用）

| 主题 | 证据路径 |
|------|----------|
| GA 工具与工作记忆 | `.workspace/GenericAgent/ga.py` |
| GA 记忆 SOP | `.workspace/GenericAgent/memory/memory_management_sop.md` |
| GA 清理 SOP | `.workspace/GenericAgent/memory/memory_cleanup_sop.md` |
| GA L4 | `.workspace/GenericAgent/memory/L4_raw_sessions/*` |
| GA 调度 | `.workspace/GenericAgent/reflect/scheduler.py` |
| Diva MemoryProvider | `agent-diva-core/src/memory/provider.rs` |
| Diva MemoryManager | `agent-diva-core/src/memory/manager.rs` |
| 无 memory tools | `agent-diva-tools/src/*`, `tool_assembly.rs` |
| 假提示 | `agent-diva-agent/src/context.rs` |
| Consolidation | `agent-diva-agent/src/consolidation.rs` |
| Laputa sync_turn 提案化 | `agent-diva-laputa/src/memory_provider.rs` |
| Typed 写仍 proposal | `agent-diva-laputa/src/typed_provider.rs` |
| AutoDream | `agent-diva-autodream/src/*`, manager autodream routes |
| 架构约束 | `docs/architecture/laputa-memory-final-architecture.md` |
| 接口规范 | `docs/architecture/memory-framework-interfaces.md` |
| 开放验收 | `TODOLIST.md`（GMH-52 等） |

---

## 10. 风险与决策点（写文档时标为 Open Questions）

1. **即时 durable vs 永远 proposal-first？**  
   - 完全 GA 手感倾向低风险即时 apply + 高风险审批  
   - 纯治理倾向全部 pending（用户会感觉“记不住”）
2. **L3 经验落 skills 还是 MemoryRecord(Learning)？**
3. **Working memory 存 session store 还是独立文件？**
4. **consolidation 保留还是被 distill+AutoDream 取代？**
5. **authority_mode 出箱默认 Typed 还是 Legacy？**（与缺失配置兼容）

这些决策在实施 Wave 0/1 前必须冻结，否则“完全对齐”无法验收。

### 10.1 决策冻结（2026-08-06，用户拍板）

> Status: **FROZEN** — Wave 0/1 编码以此为契约，不再重开。
> 决策人：用户（2026-08-06 交互确认）；记录：QoderCN。

| # | 决策点 | 冻结结论 | 关键理由 |
|---|--------|----------|----------|
| 1 | 写入路径 | **混合分级**：低风险（用户明确要求记住的事实/偏好）即时 apply；高风险（删除/覆盖既有权威、敏感内容）走 proposal 审批 | 保住 GA「记住即成功」手感，同时守住「写权威走治理」架构立场；高风险仍有 HITL 兜底 |
| 2 | L3 经验载体 | **Skills 为主**：经验沉淀为普通 `SKILL.md`（可带可选 `kind: sop` 展示标记），不建独立 SOP 子系统 | 复用 2026-07-30 既有决策 `docs/architecture/skill-sop-unification.md`：产品对象只有 Skill |
| 3 | 工作记忆 | **Session store**：随会话生命周期、易失、非权威；任务结束由 distill 显式晋升为长期记忆 | 天然符合「工作记忆 ≠ 长期权威」；无脏数据残留，无 GC 负担 |
| 4 | consolidation | **降级为兜底**：distill 工具 + AutoDream 节律为主写路径；consolidation 条目化，仅当无显式蒸馏时触发 | 消除双写冲突；consolidation 保留为 last-resort |
| 5 | authority_mode 默认 | **统一默认 Typed**：缺失配置与默认语义一致走 Typed SQLite 权威；Legacy 仅显式 opt-in 或作导入源 | 对齐「typed 是生产权威方向」；消除 missing→Legacy 与 Default→Typed 的分叉（F10） |

**对 Wave 的实施约束：**

- Wave 1 `memory` 工具（add/update/remove）在 Typed/Laputa 下按风险分级路由：
  低风险 → typed governed apply；高风险 → proposal + 审批。Legacy 模式仍按
  proposal-first 语义（不直写 MEMORY.md 冒充权威）。
- Wave 2 工作记忆注入源 = session store；distill 晋升时把 checkpoint 摘要作为
  evidence 的一部分。
- Wave 5 consolidation 改造为「无 distill 时的兜底」，阈值策略在实现时细化。
- F10 修复随 Wave 0：缺失配置与 `Default` 语义统一为 Typed，文档同步。

### 10.2 Wave 1 补充决策冻结（2026-08-06，用户拍板）

> Status: **FROZEN** — Wave 1 编码以此为契约，不再重开。
> 决策人：用户（2026-08-06 交互确认）；记录：QoderCN。

| # | 决策点 | 冻结结论 | 关键理由 |
|---|--------|----------|----------|
| W1-1 | 工具形态 | **多独立工具**：`memory_add` / `memory_list` / `memory_search` / `memory_update` / `memory_remove` / `memory_distill` 各自注册 | 每个工具 schema 简单、可独立 mask/审批/测试，模型误用率低；与现有 update_plan/ask_user 风格一致 |
| W1-2 | 风险分级判定 | **按 action + 对象状态**：add 新事实（用户明确要求）= 低风险即时 apply；update/remove 触碰既有权威 = 高风险 proposal 审批；内容级敏感检测不做（P2 后续） | 规则静态透明、可预期；混合分级（§10.1 #1）的具体化 |
| W1-3 | distill 治理 | **新建即时，覆盖走审批**：distill 产出的新 skill 文件即时创建（可审计）；覆盖/修改已有 skill 走 proposal | 主动蒸馏是显式低风险请求；触碰既有权威仍走治理；与 skill-sop-unification 一致 |
| W1-4 | 会话可见性 | **结果返回，不自动注入**：apply 成功后工具结果返回新条目内容，模型可自行引用；下次会话 prefetch 可见；同会话热注入（F4）归 Wave 3 | Wave 1 范围可控，不扩散 context 组装改动 |

**对 Wave 1 的实施约束：**

- 六个工具在 ToolAssembly 注册，独立 schema/描述/权限 mask。
- `memory_add`：Typed/Laputa 下低风险即时 governed apply；Legacy 模式 proposal-first。
- `memory_update` / `memory_remove`：一律 proposal + 审批（触碰既有权威），
  不因内容判断跳过；remove 语义为 tombstone 非物理删。
- `memory_distill`：产出普通 `SKILL.md`（可选 `kind: sop` 展示标记）；新建即时，
  覆盖既有 skill 走 proposal；distill 的 working checkpoint 摘要作为 evidence。
- `memory_search`：Typed 走 FTS5 检索 applied 权威；Legacy 降级为线性扫描或
  明确返回 degraded（不静默失败）。
- 工具结果统一诚实语义：`applied` / `proposal_created`(含 id) / `failed`。

---
