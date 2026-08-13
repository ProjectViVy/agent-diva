# R0 依赖与数据清单

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 用途：R4 deletion-proof 与 D0 权威图的直接输入
- KEEP / RENAME / DELETE / DECIDE 是**研究标记**，不是架构批准

## 1. 持久化路径

工作区相对路径。所有者见「Owner」。启动行为见 §5。

### 1.1 `.laputa/`（`LaputaPaths`，`agent-diva-laputa/src/layout.rs`）

| 路径 | 内容 | 格式 / schema | Owner | 标记 |
| --- | --- | --- | --- | --- |
| `state.json` | `schema_version: 1.0.0` | JSON | laputa layout | KEEP（布局元数据） |
| `sections/identity.json` | Frozen Core | `serde_json::Value`；缺省 seed `null` | laputa | DELETE（JSON 权威） |
| `sections/relationship.json` | Frozen Core | 同上 | laputa | DELETE |
| `sections/commitment.json` | Frozen Core | 同上 | laputa | DELETE |
| `sections/preferences.json` | Frozen Core | 同上 | laputa | DELETE |
| `sections/memory_md.json` | 文件型 LTM | JSON；**不 seed** | laputa | DELETE |
| `sections/{daily,weekly,monthly}.json` | 报告 section | JSON；不 seed | laputa | DECIDE（Notebook） |
| `sections/changelog.json` | changelog section 文件 | JSON；不 seed | laputa | DELETE（与 records 目录并存） |
| `cognitive/WORLD.MD` | 世界模型 claims | Markdown | laputa | KEEP 概念；物理待 R3 |
| `cognitive/MEMRULES.MD` | 认知规则手册 | Markdown + YAML | laputa | DECIDE |
| `cognitive/world-proposals.json` | WORLD 待审 | JSON | WorldGovernance | DECIDE |
| `cognitive/world-ledger.jsonl` | WORLD 账本 | JSONL | WorldGovernance | DECIDE |
| `memory.sqlite3` | BML 生产权威 | SQLite+FTS5，`SCHEMA_VERSION=1` | typed store | KEEP |
| `governance.db` | 统一 Approval 账本 | SQLite | Manager bootstrap | KEEP（危险工具）；Memory 域 DELETE |
| `governance.sqlite3` | proposal↔request 映射；`open_lazy` 备用账本 | SQLite | MemoryGovernanceCoordinator | DELETE（随 Memory 治理链） |
| `proposals/<id>.json` | `EvolutionProposal` | JSON | ProposalRepository | DELETE（对 Persona/Memory/旧 Evolution） |
| `changelog/<id>.json` | `ChangelogRecord` | JSON | laputa | DECIDE（Persona 历史另议） |
| `audit/<id>.json` | `AuditEvent` | JSON | laputa | DECIDE |
| `rollback/<id>.json` | 回滚请求 | JSON | laputa | DECIDE |
| `events.jsonl` | `LaputaEvent` | JSONL | laputa | DELETE（随提案总线） |
| `candidate-suppression.json` | 候选抑制 | JSON | laputa | DELETE |
| `recall-feedback.json` | 召回反馈 | JSON | laputa | DECIDE |
| `reports/{daily,weekly,monthly}/` | AutoDream 报告 | Markdown | autodream | DECIDE |
| `legacy/` | persona-retire 归档 | Markdown | CLI | DELETE 兼容链；R4 评估人工备份 |
| `migrations/workspace-identity-v1/` | 身份升级备份 | sqlite + manifest | typed store | KEEP（BML 内部） |
| `locks/` `staging/` | 锁与暂存 | — | laputa | KEEP 基础设施 |

BML 表（`typed_store.rs:436-503`）：`schema_meta`、`memory_records`、`memory_supersedes`、`memory_apply_journal`、FTS5 `memory_fts`。容量：`MAX_MEMORY_RECORDS=10_000`，`MAX_MEMORY_CONTENT_BYTES=32MiB`。WAL + FK。

`memory_records` 列：`memory_id` PK、`record_revision`、`kind`、`tenant_id`、`workspace_id`、`session_id`、`trust`、`sensitivity`、`created_at`、`effective_at`、`expires_at`、`tombstone`、`content_bytes`、`record_json`。

### 1.2 `.agent-diva/` 与工作区根

| 路径 | 内容 | 标记 |
| --- | --- | --- |
| `.agent-diva/autodream/{lock,checkpoint,events.jsonl,runs/}` | AutoDream 运行 | DELETE / DECIDE（R1：主链退役，报告是否独立） |
| `.agent-diva/audit` | 审计 JSONL | KEEP |
| `skills/<name>/SKILL.md` | Skill 权威 | KEEP |
| `memory/MEMORY.md` `HISTORY.md` | `MemoryManager` + workspace seed | DELETE |
| 根 `MEMORY.md` | 迁移源 / 归档 | DELETE |
| `SOUL.md` `IDENTITY.md` `USER.md` `BOOTSTRAP.md` | persona-retire 源 | DELETE |

### 1.3 进程 / 会话 store

| 位置 | 内容 | 标记 |
| --- | --- | --- |
| 进程内 `OnceLock<HashMap>` | Frozen Core session 快照 | RENAME/重做（随 Markdown 权威） |
| Session store | `canonical_checkpoint_v1` | KEEP（会话上下文） |
| BML `WorkingMemory` + `session_id` | 会话结束物理 DELETE | RENAME（≠ STM） |

## 2. 写入者矩阵

| 写入者 | 写什么 | 路径 | 标签 |
| --- | --- | --- | --- |
| `LaputaStorage::open` | 四份 null JSON + WORLD/MEMRULES | sections / cognitive | 源码事实 |
| GUI `SectionEditor` | **只建提案** | `POST /api/laputa/section/:name/write` | 源码事实 |
| `laputa_propose_section_write` | Frozen Core 提案 | typed_provider | 源码事实 |
| Typed apply（默认） | BML `put_governed` | `memory.sqlite3`；**不写** section JSON | 源码事实 |
| Legacy apply | section JSON | `atomic_write_json` | 源码事实 |
| `memory_add` | LongTerm 立即 Applied | BML | 源码事实 |
| `memory_update` | MemoryPatch | proposals + MemoryMd 路由 | 源码事实 |
| `memory_remove` / GUI remove | Deprecation 提案 | proposals | 源码事实 |
| `update_working_checkpoint` | WorkingMemory | BML + session_id | 源码事实 |
| `memory_distill` | `skills/<name>/SKILL.md` | 文件系统 | 源码事实 |
| AutoDream 生产 | MemoryPatch 等提案 | proposals；**不**直接 memory_add | 源码事实 |
| AutoDream worker `memory_add` | 仅测试 | `autodream/src/worker.rs` | 源码事实 |
| Consolidation | WORLD claim | WorldGovernance | 源码事实 |
| 用户直编 WORLD.MD | 文件 | `actor==user` 无限制 | 源码事实 |
| CLI `persona-retire propose` | Frozen Core 提案 | proposals | 源码事实 |
| CLI `persona-retire archive` | 移入 `legacy/` | 依赖 JSON `metadata.migration` | 源码事实 |
| Offline import | MEMORY.md → LongTerm | `agent-diva-migration` | 源码事实 |
| `run_startup_gc` | 清孤儿 WM | **无生产调用** | 源码事实 |

BML 写 API 边界（`just bml-boundary-check`）：治理模块不得直接调 `put` / `put_governed` / `import_records` / `rollback_governed` / session GC。豁免：`typed_store.rs`、`typed_provider.rs`、`memory_records.rs`、`migration.rs`。

## 3. 读取者 / Prompt 投影

| 读者 | 源 | 注入位置 |
| --- | --- | --- |
| `ContextBuilder::build_frozen_core_section` | section JSON 快照 | `ContextSection::FrozenCore` |
| Typed `system_prompt_block` | BML AppliedAuthority L1 | `MemoryPolicyAndIndex` |
| `LaputaMemoryProvider` | 五段 JSON 含 MemoryMd | 非默认生产 |
| `WorldStore::project` | WORLD claims | **无生产调用者** |
| GUI Persona | `GET /api/laputa/persona-workspace` | 编辑器 |
| GUI Memory | `/api/bml/memories` | 列表 |
| AutoDream inputs | MemoryMd + Identity JSON | 反射输入 |
| Frozen Core 空 | 常量 `FIRST_RUN_ONBOARDING_BLOCK` | 稳定前缀 |

## 4. 符号 / 路由 / DTO / 事件（deletion-proof 扫描面）

### 4.1 必须随 `memory_md` 清零

- Rust：`LaputaSectionName::MemoryMd`、`as_str()="memory_md"`、`ProposalType::MemoryPatch` 路由、`all_v1()` 第 5 项、`section_file_stem`、`is_writable_apply_target`、`record_kind_for_section` MemoryMd 臂、`LaputaMemoryProvider::authority_sections` 第 5 项、`section_title` “Long-Term Memory”、AutoDream 默认 `laputa_sections`、Notebook `classify_memory_report` 默认 MemoryPatch
- HTTP：`POST /api/laputa/section/memory_md/write` 及 manager 测试 URI
- TS：`LaputaSectionName` 的 `'memory_md'`、`SectionGroupList` `long_term`、i18n `memory_md` / `long_term`
- 文件：`.laputa/sections/memory_md.json`、`MEMORY.md` / `memory/MEMORY.md` seed
- 不要误删：BML `MemoryRecordKind::LongTerm`、GUI Memory 页 `long_term` kind、`canonical_checkpoint_v1`

### 4.2 必须随 Persona JSON 权威清零

- `LaputaSection.content: serde_json::Value`、`section_content_type` 返回 `"json"`
- `FrozenCoreSnapshot` 紧凑 JSON 字符串、`content_version` 对 JSON 哈希
- `SectionEditor` `JSON.parse` / `formatJson` / `jsonError`
- `PersonaMemoryView.formatContent` → `JSON.stringify`
- `create_user_edit_proposal` Identity/Relationship/Commitment/Preferences 臂
- `ProposalType::{IdentityPatch,RelationshipUpdate,CommitmentSet,LearningNote}`
- `laputa_propose_section_write` JSON 字符串合同
- CLI persona-retire 的 `{status,entries,metadata.migration}` JSON patch
- HTTP `GET/POST /api/laputa/section/:name[/write]` 的 JSON body 合同
- 测试：`frozen_core.rs`、`cognitive/sections.rs`、`context_plane_invariants.rs`、`propose_section_write.rs`、`create_user_edit_proposal.rs`、`apply.rs`、agent `context.rs` Frozen Core 测试、`agent_loop.rs:2901` first-run、`SectionEditor.spec.ts`、`SectionGroupList.test.ts`

产品现行权威名见 Persona 决策 P1（七份，含 `DARK.MD`；`IDENTITY.MD` 含身体）。下表
代码符号仍是旧实现名，不是产品名。

### 4.3 必须随旧 Evolution/治理链清零（对 Persona/Memory）

- `MemoryGovernanceCoordinator`、`MemoryApply` capability、Approval `domain=memory`
- GUI `PersonaLifecyclePanel`、`GovernanceActionBar`、`ProposalInbox/Detail` 作为 Persona/Memory 入口
- Evolution 文案捆 personality/memory/SOP/skill
- `ProposalType::SopCreate` → Identity
- AutoDream → Evolution 主链（报告是否留下：DECIDE）
- `governance.sqlite3` 映射表 `memory_proposal_governance`
- Chat `openEvolutionProposal` / Memory 页 `open-approval`

保留：`/api/approvals` 的 command/plan；M3；SkillsLoader。

### 4.4 事件

| 总线 | 符号 | 传输 | 标记 |
| --- | --- | --- | --- |
| `AgentEvent` | 对话/工具/Plan | `/api/chat` SSE | KEEP |
| Unified approvals | `approval.requested/resolved/updated` | `/api/approvals/events` | KEEP（去掉 memory 域） |
| Command HITL | `command_approval_requested` | `/api/command-approvals` | DECIDE（与 M3 合并） |
| `LaputaEventKind` | Proposal/Changelog/Error/BufferOverflow | `/api/laputa/events/:kind` | DELETE（随旧治理） |
| AutoDream events | 文件 JSONL | `/api/autodream/runs/:id/events` | DELETE / DECIDE |
| `useEventStream.ts` | 旧 9100 SSE | 未接入 | DELETE |

### 4.5 Manager / Tauri 路由索引

| 方法 | 路径 | 标记 |
| --- | --- | --- |
| GET | `/api/laputa/persona-workspace` | RENAME（随 Persona DTO） |
| GET | `/api/laputa/snapshot` | DELETE / RENAME |
| GET/POST | `/api/laputa/section/:name[/write]` | DELETE JSON 写；Persona 直写另设计 |
| GET | `/api/laputa/cognitive/:kind` | DECIDE |
| * | `/api/laputa/proposals/**` | DELETE（对 Persona/Memory/旧 Evolution） |
| * | `/api/laputa/changelog/**` | DECIDE |
| GET | `/api/bml/memories` `/:id` | KEEP |
| POST | `/api/bml/memories/:id/remove` | RENAME（应直接 CRUD，不建提案） |
| * | `/api/autodream/runs/**` | DELETE / DECIDE |
| * | `/api/approvals/**` | KEEP（去掉 memory） |
| * | `/api/skills` | KEEP |

Tauri 对应 `laputa_*` / `bml_*` 同标记。

## 5. 启动、恢复、失败处理

| 场景 | 行为 | 标签 |
| --- | --- | --- |
| 缺 `.laputa/` | `open` 建目录 + 种子 | 源码事实 |
| 缺 `memory.sqlite3` | Typed `open_canonical` 创建；Garden list 缺库返回空、不创建 | 源码事实 |
| BML 完整性失败 | `DegradedMemoryProvider` | 源码事实 |
| Typed apply 崩溃 | apply journal + `recover_apply_outcome` / 重放 `put_governed` | 源码事实 |
| Legacy apply 崩溃 | section 回滚到 `before` | 源码事实 |
| Frozen Core 捕获失败 | 空快照 | 源码事实 |
| 共享账本缺 pending | `recover_memory_approvals` 迁入；不导入 allow receipt | 提交事实 `78e2bcf5` |
| 已批准但无 receipt | `needs_attention` | 提交事实 |
| 孤儿 WorkingMemory | `run_startup_gc` 未挂启动 | 源码事实 |
| WORLD 解析失败 | best-effort 空 claims | 源码事实 |
| MEMRULES 缺失 | `load_or_default` 内置文本 | 源码事实 |

## 6. 测试夹具（删除时必清）

`memory_md` / MemoryPatch：

- laputa：`tests/apply.rs`、`create_user_edit_proposal.rs`、`context_plane_invariants.rs`、`governance_proof_loop.rs`、`wave5_acceptance.rs`、`storage.rs`、`proposals.rs`、`service.rs`、`migration.rs`、`governed_apply.rs`、`authority_boundary_guard.rs`
- autodream：`tests/worker.rs`、`tests/service.rs`、`src/inputs.rs` 测试
- manager：`server.rs` memory_md write 测试；`autodream_laputa_e2e.rs`
- gui：`desktop.laputa.test.ts`、`SectionGroupList.test.ts`、`NotebookView.test.ts`、`EvolutionView.test.ts`、`ChatGovernanceCard.test.ts`
- tools：`laputa_propose_section_write.rs` 示例 `"section":"memory_md"`（与工具描述 “Frozen Core only” 不一致）

Persona JSON / Frozen Core：§4.2 列表。

失败基线防回归（保留为证伪，直到新架构替换）：

- `agent-diva-gui/src/utils/errorMessage.test.ts`
- `PersonaMemoryView.test.ts` / `EvolutionView.test.ts` 禁止错误条含 `[object Object]`

## 7. KEEP / RENAME / DELETE / DECIDE

| 对象 | 标记 | 理由 | 交给 |
| --- | --- | --- | --- |
| BML `memory.sqlite3` + `/api/bml` list/get | KEEP | 已冻结 LTM 权威 | — |
| `memory_add` 直接 put | KEEP | Memory 不审批 | — |
| Frozen Core + WORLD + DREAM + DARK 概念 | KEEP | 产品现行七份权威，见 Persona 决策 P1 | D1 换载体 |
| SkillsLoader + `skills/*/SKILL.md` | KEEP | Skill runtime | D3 接线 |
| Chat Approval Center + M3 | KEEP | 运行时安全 | — |
| C1–C5 / `canonical_checkpoint_v1` | KEEP | 会话上下文 ≠ STM | R2 核对装配 |
| BML 边界门禁 | KEEP | 治理不得直写 store | — |
| `memory_md` / `MemoryMd` / `MemoryPatch→MemoryMd` | DELETE | STM 决策 S2 | R4 |
| Persona 左栏 `long_term` | DELETE | 同上 | R4 |
| `LaputaMemoryProvider` 注入 MemoryMd | DELETE | 同上 | R4 |
| `MEMORY.md` seed / `MemoryManager` 默认 | DELETE | 兼容链 | R4 |
| Persona JSON section + `content: Value` + JSON 编辑器 | DELETE | Persona Markdown 决策 | R3/R4 |
| Persona 右栏通用治理生命周期 | DELETE | P2 | R3/R4 |
| first-run 走 `laputa_propose_section_write` | DELETE | 应五份原子直写 | D1 |
| AutoDream 作为 Evolution 主链 | DELETE / DECIDE | R1；报告是否独立 | R4/D3 |
| EvolutionView 四页签 + Proposal inbox | DELETE（对 Evolution） | 对象错位 | D3 |
| `MemoryGovernanceCoordinator` + Approval `domain=memory` | DELETE | 产品边界 | R4 |
| `governance.sqlite3` 映射库 | DELETE | 双账本残留 | R4 |
| `ProposalType` Identity* / LearningNote / SopCreate | DELETE | Evolution 不得承载 Persona/SOP | R4 |
| GUI `working_memory` 当 STM | RENAME | 必须与未来 STM 分名 | R2/D2 |
| BML `MemoryRecordKind::{Identity,…}` | DECIDE | 与 Persona JSON 平行 | D0 |
| WORLD `project()` / WorldGovernance | DECIDE | 投影与写入者 | R3 |
| MEMRULES | DECIDE | 手册 vs 写路径门 | R3 |
| AutoDream 报告 / Notebook | DECIDE | 是否独立 | R4/D3 |
| `run_startup_gc` 未接线 | DECIDE | session 生命周期 | R2 |
| SelfEvolution 配置未接 cron | DECIDE | 节律归属 | D3 |
| `/api/command-approvals` 旧轨 | DECIDE | 与统一 Approval 合并 | 运行时，非本 EPIC 主链 |
| `useEventStream.ts` | DELETE | 未接入遗留 | 清理 |

## 8. 明确重叠（D0 必须消解）

未声明重叠会阻断 Research Gate 的「边界无重叠」条。此处只登记，不选方案：

1. Persona JSON 文件 vs BML Identity/Relationship/Commitment/Preference 行
2. `memory_md.json` vs BML LongTerm
3. session `WorkingMemory` vs 产品 STM vs `canonical_checkpoint_v1`
4. `governance.db` vs `governance.sqlite3`
5. Frozen Core Prompt vs Typed L1 vs `LaputaMemoryProvider` 文件权威块
6. WORLD Markdown vs consolidation WorldGovernance vs（未接线的）Prompt 投影
7. Skill 文件树 vs `memory_distill` vs Evolution 提案 vs Settings Skills
8. Approval Center memory 域 vs Evolution inbox vs Persona 右栏

## 9. 本清单未覆盖

- 生产 profile 抽样（用户机器上的真实 `.laputa` 体积与内容）→ R4
- GenericAgent 上游文件级 diff → 已在 R1
- STM 方案实验 → R2
