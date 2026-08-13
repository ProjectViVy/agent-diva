# BML 存储层独立化调研（BML Layer Extraction Research）

- Status: **调研报告（决策已拍板：D，2026-08-08）**
- Date: 2026-08-08
- Scope: 评估把 BML（记忆存储层）从 `agent-diva-laputa` crate 抽出为独立层的可行性、方案与代价
- 决策链: AGENTS.md + TODOLIST.md D3 三层模型冻结（2026-08-08）→ 本调研 → 已拍板 D（§9）
- 调研方式: 定向精读验证（H1–H6）+ 桌面 Garden 仓库只读对照 + 既有文档核对

> **术语澄清（重要）**：本报告中的 Garden 侧 "Mentle"（`garden/mentle/` Go module，
> 材料/证据/检索/图）即 **BML 在 Garden 侧的对应物**。它**不是**
> mempalace-mentle（Python MemPalace，`.workspace/mempalace-py/`，ChromaDB+SQLite
> 参考实现），也不是 `.workspace/memtle/`（独立 Rust 存储工具）。diva 历史中的旧
> "Mentle"（臃肿记忆系统）已被 GMH-24 clean-break 移除，BML 是其继承者
> （AGENTS.md："The previous name 'Mentle' is retired; use BML"）。凡下文提及
> Garden 的 Mentle，均指 BML 的 Garden 侧对应物。

---

## 1. 背景与决策链

2026-08-08 冻结三层模型（TODOLIST.md D3）：

- **BML (Basic Memory Layer)** = 记忆存储层（Mentle 继承者，`.laputa/memory.sqlite3` 唯一权威）
- **Laputa** = 人格治理层（proposal / governed apply / audit / Frozen Core / cognitive）
- **Garden** = facade（用户可见记忆管理面，agent-diva 中未实现）

历史条款：GMH-23A（2026-07-24 冻结）将 "SQLite/FTS5 is owned by `agent-diva-laputa`"
（归档批次 `architecture/legacy/laputa-memory-final-architecture.md:137-138`）——当时 "Laputa"
语义 = 存储 + 治理统一体；D3 之后 "Laputa" 语义收窄为治理层，BML 单独命名。
归档的 `memory-framework-interfaces.md:19-22` 声明 "There is no `agent-diva-memory` crate"。

**代码现状：BML 存储实体物理合体在 `agent-diva-laputa` crate 内部，未独立成层。**

## 2. 现状耦合盘点

### 2.1 crate 内耦合（agent-diva-laputa/src/）

| # | 耦合类型 | 证据 | 说明 |
|---|---------|------|------|
| 1 | 存储核心自包含 | `typed_store.rs` 事务 `put_inner` :941-1153、`import_records` :725-835、`rollback_governed` :882-922、CAS :969-1022、FTS5 建表 :495-509 / 写 :808-820,1104-1124 / 读 :1173-1246 | `TypedMemoryStore` 自有 pool/锁/事务/CAS/FTS5，crate 内仅依赖 `LaputaPaths` + `atomic_write_json` |
| 2 | 治理→存储（3 处只读） | `service.rs:166,215`（open_existing_canonical + superseded_target_ids + list）、`recall.rs:26,64,70,126`（search/search_visible）、`governed_apply.rs:28,46`（仅错误类型） | 全部只读枚举/检索 |
| 3 | 存储→治理（类型级） | `memory_records.rs:20-21,93,162`（adapt 函数依赖 `EvolutionProposal`/`LaputaSection`/`SectionStatus`） | 适配器依赖治理类型，抽层需重排 |
| 4 | `put_governed` seam 未注册 | `typed_store.rs:856-857` "This seam remains unregistered until the GMH-24 write cutover" | 生产路径 governed apply 走文件 `ProposalRepository` + `governance.sqlite3`，**不碰** memory.sqlite3 事务（H1 ✅：`proposals.rs` 零 SQLite 引用） |
| 5 | 深度混合门面 | `typed_provider.rs:30-40` `TypedLaputaMemoryProvider` 同时持有 crud_store（TypedMemoryStore）+ coordinator（MemoryGovernanceCoordinator）+ proposal_sink（Arc&lt;LaputaMemoryProvider&gt;）+ recall + feedback | 唯一深度混合点（H2 ✅），抽层后可退化为纯组合层 |
| 6 | 两库物理分离 | `layout.rs:105-111`：`memory.sqlite3` 与 `governance.sqlite3` 是两个独立文件；`governed_apply.rs:96-98` 用 `LaputaPaths::governance_database()` 开库 | H6 ✅：治理库（governance.sqlite3 + 文件状态机）与 BML 库（memory.sqlite3）已物理分离 |

### 2.2 外部消费面（直接依赖 laputa 的 5 个 crate）

| 档位 | crate | 证据 |
|------|-------|------|
| 直碰存储类型 | agent-diva-agent | `memory_boundary.rs:38-70,602`（TypedMemoryStore::open_* / LaputaRecallService / TypedLaputaMemoryProvider） |
| 直碰存储类型 | agent-diva-manager | `handlers/laputa.rs:21,546-550,943-953,1130-1145`、`handlers/health.rs:95-110`、`server.rs:966-969`（测试） |
| 直碰存储类型 | agent-diva-migration | `workspace_identity.rs:4,19-57`、`typed_memory.rs:12-14,63-290`（adapt_* / MemoryRecordMigration / MemoryStoreIntegrity / MemoryStoreMetadata / atomic_write_json）、`experience.rs:11` |
| 直碰存储类型（仅测试） | agent-diva-autodream | `worker.rs:830,878,1002-1005`（测试用 TypedMemoryStore） |
| 仅走治理门面 | agent-diva-cli | `commands/persona_retire.rs`（LaputaService + persona 函数） |
| 仅走治理门面 | agent-diva-autodream 生产 | `inputs.rs:177,768`（LaputaStorage）、`outputs.rs`、`worker.rs`、`service.rs` |
| 仅走治理门面 | agent-diva-agent 部分 | `context.rs:491`、`consolidation.rs:9` |
| 零直接依赖 | agent-diva-tools / agent-diva-gui | tools 走 `core::memory::MemoryProvider` trait；GUI 经 manager HTTP |

**H3 修正（重要）**：manager 的直调**全部是存储层本质操作**——health 健康检查
（integrity()）、recover 补刀（metadata()）、rollback 端点（metadata() +
rollback_governed()）、错误映射（TypedMemoryStoreError）。这些不是"该门面化的债"，
而是 BML 层应当暴露的能力。抽层时这些调用点**跟随存储类型搬移（改 import 路径）**，
不是重构。

**H4 修正**：`autodream` 使用**自己的** `atomic.rs`（`autodream/src/atomic.rs:28`
`pub(crate) fn atomic_write_json`），不依赖 laputa 的 atomic；laputa 的 `atomic_write`
被 manager 使用（`handlers/laputa.rs:812`），`atomic_write_json` 被 migration 使用
（`typed_memory.rs:12`、`experience.rs:11`）。

**H5 ✅**：migration 是迁移成本上界——三个文件全直接依赖存储层类型，且含
adapt_* 适配器 + MemoryRecordMigration（v2 迁移）逻辑。

## 3. Garden 同构对照

### 3.1 Garden 仓库事实（`C:\Users\Administrator\Desktop\garden`，只读）

| 维度 | Garden（Go，三独立 module） | agent-diva 现状（Rust 单 crate） |
|------|---------------------------|--------------------------------|
| 存储 | `mentle/`：material/evidence/retrieval/graph/vector/storage + `facade/` | `agent-diva-laputa::typed_store` + `memory_records`（合体） |
| 治理 | `laputa/`：governance/（audit/governed/engine/rhythm/scheduler/store） | `agent-diva-laputa::service/proposals/governed_apply/cognitive` |
| facade | `garden/`：HTTP gateway/console/ingestion/recall orchestration | 未实现（GUI = BML P0 待做） |
| 边界声明 | README："No module holds authority over the others. Each degrades gracefully." | 无物理边界（逻辑分层已冻结 D3） |
| 边界执行 | ADR-0011 Gate E（E8）：grep 禁用 token（governance/audit_log/MEMRULES/WORLD/persona）验证 Mentle 不吸收治理 | 已有 authority_boundary_guard / direct_write_guard 测试（可扩展同款门） |

### 3.2 Garden 侧的关键语义（与本调研直接相关）

- ADR-0001（2026-08-01 proposed）决策 2："Mentle 是原文优先的 Material Universe /
  Evidence Lake……正常路径先持久化原始 SourceArtifact，再派生卡片、证据、摘要和检索
  索引；**Mentle 不取得 authority**"
- ADR-0001 决策 3："Laputa 不再拥有 LTM / LONGMEM.MD 长期内容层；历史材料、版本和
  证据由 Mentle 保留，避免双写和双真源"
- ADR-0011（2026-08-04 proposed）§1："governance stays with Laputa, evidence stays
  with Mentle"；"must not migrate into Mentle" 清单：KG 事实核查、人格自动晋升、
  heat 语义、用户长档案、权限/审计、图谱遍历策略

### 3.3 权威语义差异（重要发现）

Garden 的 "authority" = **治理权**（谁有权写、审计、晋升）；Mentle 明确不持有。
diva 的 "authority" = **权威记录实体**（`.laputa/memory.sqlite3` 是 "sole production
Memory authority"，记录级 trust/tombstone 语义在库内）。两者架构意图同构——
存储管内容持久化，治理管写权限与审计——但 diva 的 BML 比 Garden 的 Mentle 多承担了
"权威状态机"（tombstone/CAS/revision）的持久化职责。抽层**不改变**这一语义：BML
仍是记录状态的唯一持久化实体，治理权仍在 Laputa。

## 4. 抽层动机与反对理由

**动机**：
1. 职责隔离：D3 三层模型要求 BML/Laputa 边界清晰，当前合体使"谁拥有什么"靠文档而非结构
2. Garden facade 未来接入：facade 需要 BML 的稳定公开 API，穿透 laputa crate 私有实现成本高
3. 与 Garden 同构：Garden 三 module 强边界是既有先例（§3.1），diva 侧物理同构可降低认知税

**反对理由**：
1. 不新增用户能力：抽层是纯结构变更，G2D+ 真机验收与 BML UI（P0）优先级更高
2. 迁移成本：直碰存储类型的外部 crate 达 4 个（§2.2），一次性断链面大
3. 契约修订：GMH-23A "SQLite/FTS5 is owned by agent-diva-laputa" 与 §8 禁令需先论证/修订
4. 权威语义不变：物理 crate 边界不改变权威性（GMH-23A 已锁定 BML 实体唯一归属）

## 5. 方案选项

### A 全抽独立 crate `agent-diva-bml`
移动 `typed_store.rs`、`atomic.rs`、`lock.rs` + `LaputaPaths` 中 memory 相关路径；
`memory_records.rs` 适配器依赖治理类型（EvolutionProposal/LaputaSection），需拆分
（存储侧保留纯 adapt，治理侧保留 adapt_governed_proposal）；4 个外部 crate 改 import；
混合测试拆分（wave5_acceptance/governed_apply/feedback/propose_section_write）。
新 crate 命名用 **agent-diva-bml**（不用 agent-diva-memory，规避 §8 禁令字面）。

### B crate 内模块边界 + 文档声明（低配）
存储模块 pub 面收敛（typed_store/atomic/lock 收进 `bml/` 子模块或加 `#[doc(hidden)]`
中间层），AGENTS.md 声明"BML 逻辑层位于 agent-diva-laputa::bml"；新增边界测试
`bml_boundary_guard`：禁止治理层代码调用 `put`/`put_governed`/`import_records` 等
写接口（仿 ADR-0011 E8 grep 门 + 现有 direct_write_guard）。

### C 保持现状仅文档化
D3 语义已冻结，物理位置不动，本报告即"文档化"产物。

### D 中间态（推荐候选）：先 B 固化边界，远期视消费面收敛升 A
B 先行（低成本、零运行时风险、防回归门）；GMH-23A 修订 + Garden facade 落地时
（届时 facade 需要 BML 稳定 API，抽层收益显现）再执行 A。

## 6. 成本 / 风险 / 收益矩阵

| 维度 | A 全抽 | B 模块边界 | C 现状文档化 | D 先 B 后 A |
|------|--------|-----------|-------------|------------|
| 迁移成本 | 高（4 crate 断链 + 适配器拆分 + 混合测试拆分） | 低（模块内收敛 + 1 个边界测试） | 零 | B 的成本 + 未来 A 的成本 |
| 编译/依赖影响 | 中（workspace 依赖变更） | 无 | 无 | 阶段性 |
| 运行时风险 | 低（纯搬移，SQLite 句柄/锁语义不变） | 零 | 零 | 零→低 |
| 测试面 | 需拆分 5+ 个混合测试文件 | 新增 1 个负向测试 | 无 | 同 B 后同 A |
| 文档合规 | 需先修订 GMH-23A + §8 论证 | 仅增补声明 | 已完成（D3） | 分阶段 |
| 收益 | 物理边界 + 权威所有权清晰 + facade 可直连 | 逻辑边界 + 防回归门 | 最低（认知税持续） | 收益逐步释放 |
| 时机 | 当前投入与 G2D+/BML UI 竞争 | 可立即做 | — | **与未来 facade 同步** |

## 7. §8 禁令与 GMH-23A 论证

**§8 禁令原文**（归档的 `memory-framework-interfaces.md:346`）：
"Creating an `agent-diva-memory` crate **merely to match historical documents**."

论证：
- 禁令限定语为 "merely to match historical documents"（仅为对齐历史文档）。本次抽层的
  动机是 D3 职责分离 + Garden 同构，**不是**对齐历史文档，字面上不违反。
- 但新 crate 命名应使用 **agent-diva-bml** 而非 agent-diva-memory，避免撞禁令字面；
  同时该禁令条款与 GMH-20 基线声明（:19-22）应随决策追加 amendment 说明
  "BML 抽层系 2026-08-08 D3 架构决策的落地，非历史文档对齐"。

**GMH-23A 所有权条款原文**（归档的 `laputa-memory-final-architecture.md:137-138`）：
"SQLite/FTS5 is owned by `agent-diva-laputa`…"

论证：
- 该条款 2026-07-24 冻结时 "Laputa" = 存储 + 治理统一体；D3 后语义收窄。条款字面
  （"owned by agent-diva-laputa"）仍覆盖 BML 实体。
- 选 B/C：条款无需修订（所有权不变，仅增加分层声明）。
- 选 A：需修订条款为 "SQLite/FTS5 is owned by `agent-diva-bml`"——修订路径：
  归档的 `laputa-memory-final-architecture.md` 增加 Amendment 段 + TODOLIST 决策记录，标注
  "GMH-23A 条款按 D3 语义拆分修订，2026-08-08"。

## 8. 决策建议与验收标准

### 决策建议：**D（近期落 B，远期视 Garden facade 落地升 A）——已于 2026-08-08 拍板采纳（§9）**

理由：
1. 直碰存储类型的外部 crate 达 4 个，全抽一次性成本高，且当前不新增任何用户能力——
   G2D+ 真机验收与 BML UI（P0）的优先级高于结构重构。
2. GMH-23A 已锁定 BML 实体唯一归属（memory.sqlite3 权威不变），物理 crate 边界
   不改变权威性——B 方案已能把"谁拥有 BML"讲清楚，且成本极低、风险为零。
3. §8 禁令与 GMH-23A 契约修订需要决策流程，A 的流程成本在当前时机不划算。
4. Garden facade 落地（BML UI / 未来桌面端）是抽层的最佳时机：届时 facade 需要
   BML 稳定公开 API，抽层收益才真正显现，且可一次性完成消费面收敛。

**不做的代价**（选 C 或长期 B）：三层模型文档与代码结构持续错位（认知税）；
Garden facade 落地时需穿透 laputa crate 私有实现，届时抽层成本高于现在逐步收敛。

### 验收标准（本调研）

- [x] 报告十节齐全，H1–H6 逐项给出验证结论（含文件:行号）
- [x] Garden 对照表完成（README + ADR-0001/0011 + 目录结构）
- [x] §8 禁令与 GMH-23A 原文逐字核对
- [x] 决策建议单一明确 + 不做的代价
- [x] docs/research/README.md、TODOLIST.md 同步

## 9. 决策记录（2026-08-08 已拍板）

- [x] 决策：BML 抽层选 **D——先 B（crate 内模块边界 + 防回归门）固化边界，远期视 Garden facade 落地升 A（全抽）**。
- [ ] 若实施 B：执行 `bml_boundary_guard` 负向测试 + AGENTS.md 分层声明（排期待决策）
- [ ] 若未来升 A：GMH-23A 所有权条款修订 + §8 禁令 amendment（§7 已给论证与修订路径）
- 术语注：Garden 侧 Mentle = BML 的 Garden 侧对应物，非 mempalace-mentle（见文首术语澄清）

## 10. 附录：证据索引

| 证据 | 位置 |
|------|------|
| `put_governed` seam 未注册 | `agent-diva-laputa/src/typed_store.rs:856-857` |
| 治理→存储只读调用 | `agent-diva-laputa/src/service.rs:162-217`、`recall.rs:26,64,70,126`、`governed_apply.rs:28,46` |
| 适配器依赖治理类型 | `agent-diva-laputa/src/memory_records.rs:20-21,93,162` |
| 深度混合门面 | `agent-diva-laputa/src/typed_provider.rs:30-40` |
| 两库路径分离 | `agent-diva-laputa/src/layout.rs:105-111`、`governed_apply.rs:96-98` |
| manager 存储直调 | `agent-diva-manager/src/handlers/health.rs:95-110`、`handlers/laputa.rs:546-550,943-953,1130-1145`、`server.rs:966-969` |
| migration 直碰存储 | `agent-diva-migration/src/workspace_identity.rs:4,19-57`、`typed_memory.rs:12-14,63-290`、`experience.rs:11` |
| autodream 自有 atomic | `agent-diva-autodream/src/atomic.rs:28`（inputs.rs:177,768 仅用 LaputaStorage） |
| §8 禁令原文 | 归档 `architecture/legacy/memory-framework-interfaces.md:346`（基线声明 :19-22） |
| GMH-23A 所有权条款 | 归档 `architecture/legacy/laputa-memory-final-architecture.md:137-138`（GMH-23A 冻结契约 :14-35） |
| Garden 三 module | `C:\Users\Administrator\Desktop\garden\README.md`（Laputa/Mentle/Garden 职责表 + "No module holds authority"） |
| Garden Mentle 不持权威 | `C:\Users\Administrator\Desktop\garden\docs\architecture\0001-memoryos-vnext-architecture.md` 决策 2/3 |
| Garden 存储层边界执行 | `C:\Users\Administrator\Desktop\garden\docs\architecture\0011-recoverable-indexing-and-evidence-contract.md` §1 + Gate E（E8） |
| 三层模型冻结 | `AGENTS.md`（BML/Laputa/Garden 定义）、`TODOLIST.md` D3 |
