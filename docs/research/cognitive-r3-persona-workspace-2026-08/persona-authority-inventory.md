# R3 Persona / WORLD 当前权威盘点

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 性质：当前实现事实；不定目标目录、DTO 或 D1 方案
- 依赖：R0 [`../cognitive-r0-current-state-2026-08/current-state-map.md`](../cognitive-r0-current-state-2026-08/current-state-map.md)
  §3–8、[`dependency-and-data-inventory.md`](../cognitive-r0-current-state-2026-08/dependency-and-data-inventory.md)
  §4.2；产品边界
  [`../persona-markdown-clean-break-2026-08/decision-record.md`](../persona-markdown-clean-break-2026-08/decision-record.md)
  P1–P12
- 本包不重写 R0 全景；只加深 Persona / WORLD / 首次初始化切片

## 1. 一句话现状

今天没有「四份 Markdown + WORLD」文档权威。四份人格是 `.laputa/sections/*.json`
（启动预种子 `null`），保存只建 `EvolutionProposal`，默认 Typed apply 写进 BML
而不回写 JSON。WORLD 已是独立 Markdown claims，但 Prompt **不**调用
`WorldStore::project`。首次引导有三套互相打架的判定，没有五权威原子直写 API。

## 2. 五份权威对照

| 产品对象 | 今天物理文件 | 正文类型 | 写入者 | 读者 | 失败回退 | 标签 |
| --- | --- | --- | --- | --- | --- | --- |
| Identity | `.laputa/sections/identity.json` | `serde_json::Value`；缺文件当 `Null` | `LaputaStorage::open` 预种子 `null`；Legacy apply `atomic_write_json`；Typed apply **不写此文件** | Frozen Core 捕获、GUI `formatContent`、AutoDream 读 Identity JSON、非默认 `LaputaMemoryProvider` | 捕获失败 → 空快照 | 源码事实 |
| Relationship | `sections/relationship.json` | 同上 | 同上 | 同上 | 同上 | 源码事实 |
| Commitment | `sections/commitment.json` | 同上 | 同上 | 同上 | 同上 | 源码事实 |
| Preferences | `sections/preferences.json` | 同上 | 同上 | 同上 | 同上 | 源码事实 |
| WORLD | `.laputa/cognitive/WORLD.MD` | Markdown claims（`# WORLD` + `## [domain] title`） | `initialize_dir` 种子 `# WORLD\n`；用户直编文件；`WorldStore::save` / `governed_upsert`；Consolidation `WorldGovernance::submit` | GUI 只读 `<pre>`；`WorldStore::project` **仅测试** | 缺文件 → 空 claims；解析失败 best-effort | 源码事实 |

R0 已标 DELETE 的 JSON section 权威、`memory_md` 左栏、右栏治理生命周期此处不重复长表。
概念 KEEP：四个对象 + WORLD。载体待 D1。

相邻但**不是**第五份 Frozen Core、也不是产品 Persona 文档：

| 对象 | 路径 | 本包处理 |
| --- | --- | --- |
| MEMRULES | `.laputa/cognitive/MEMRULES.MD` | GUI 与 WORLD 同组只读；`context_plane_invariants` 禁止进 Prompt。DECIDE 交给 D0/D1：是否仍挂在 Persona 左栏 |
| `memory_md` | `sections/memory_md.json` | R0/R4 DELETE；Persona 左栏 `long_term` 仍展示 |
| changelog section | `sections/changelog.json` + `changelog/<id>.json` | 治理记录，≠ 文档历史（见 §8） |
| BML `MemoryRecordKind::{Identity,…}` | `memory.sqlite3` | 与 JSON 平行的第二权威；D0 消解，本包不选 |

## 3. Frozen Core 捕获与 Prompt 投影

源码：`agent-diva-laputa/src/frozen_core.rs`，装配：
`agent-diva-agent/src/context.rs` `build_frozen_core_section`。

```text
ContextBuilder 首次装配 session
  → capture_frozen_core_for_session(workspace, session_key)
  → LaputaService::read_section × 4
  → content.is_null() ? "" : serde_json::to_string(Value)   // 紧凑 JSON
  → content_version(rendered) = "sha256:" + SHA-256(bytes)
  → 进程内 OnceLock<HashMap<(workspace_id, session_key), Snapshot>>
  → render(4000)：跳过空段；`## Frozen Core — {name}\n{compact_json}\n`
  → 四段皆空 ⇒ 追加 FIRST_RUN_ONBOARDING_BLOCK
```

事实：

- 会话内冻结：同 key 再捕获返回缓存；`release_frozen_core_session` 后才重捕。源码事实：`frozen_core.rs:117-158`、`agent_loop/loop_runtime_control.rs` reset/delete 调用 release。
- 捕获失败（打不开 service）→ 四段空字符串，不报错给调用方。源码事实：`frozen_core.rs:129-143`。
- `is_empty()` 只看四段字符串是否全空。`null` 种子因此算空。源码事实：`85-87`、测试 `290-320`。
- 非空 `{}` 会变成 `"{}"`，**不算空**，first-run 块不再出现。推断：GUI `JSON.stringify(value ?? {}, null, 2)` 把 `null` 显示成 `{}`，用户若保存空对象，会永久关掉 Prompt 引导。
- 预算 4000 字符，按 Identity → Relationship → Commitment → Preferences 顺序截断。源码事实：`91-114`、测试 `323-340`。
- 稳定前缀顺序（R0 §6 / R2）：MaskAndIdentity → **FrozenCore** → Skills → Memory L1。Mask 硬编码 identity header，**不读** Frozen Core / WORLD。源码事实：`context.rs:343-375`。
- 禁止注入：MEMRULES 全文、WORLD 全文、报告、退役人格文件。源码事实：`tests/context_plane_invariants.rs:7-11, 99-105`。
- `WorldStore::project` 生产 Prompt **零调用**。实验观察：`rg WorldStore::project` 仅 `world.rs` 定义/单测与 `context_plane_invariants.rs` 测试。

`LaputaSection.version` **不是**内容 CAS。它等于 `state.json` 的 `schema_version`（默认 `"1.0.0"`）。源码事实：`service.rs:553-563`。GUI 展示的 `authority_versions` 另算 `content_version`（见 §5）。

## 4. 旧保存链（用户点保存）

R0 §4.1 仍准确。补行号：

```text
SectionEditor.handleSave
  → 要求 dirty ∧ 合法 JSON.parse ∧ 非空 changeReason
  → writeLaputaSection(name, draft, reason)
  → POST /api/laputa/section/:name/write
       WriteLaputaSectionPayload { content, actor?, summary? }   // 无 base_revision
  → LaputaService::create_user_edit_proposal
       Identity → IdentityPatch
       Relationship → RelationshipUpdate
       Commitment → CommitmentSet
       Preferences → LearningNote
       非 JSON → SchemaIncompatible
  → MemoryGovernanceCoordinator.submit
       governance.db ApprovalRequest (capability=MemoryApply)
       governance.sqlite3 映射
  → GUI 右栏 PersonaLifecyclePanel：decide / apply / rollback
```

源码事实：`SectionEditor.vue:114-148`、`desktop.ts:605-614`、
`handlers/laputa.rs:134-138, 1096-1123`、`service.rs:357-436`。

保存**不写** section 文件。默认 Typed apply：`adapt_governed_proposal` → BML
`put_governed`，`finalize_typed_proposal` `write_authority=false`。Legacy apply
才 `atomic_write_json(section_file)`。推断（与 R0 一致）：GUI 再刷新仍读 JSON
种子 `null`；本会话 Frozen Core 也不变。

工具 `laputa_propose_section_write` 走同一提案链。Prompt first-run 块明确要求
这条路径。源码事实：`context.rs:38-46`。

## 5. `content_version` 不是写路径 CAS

| 用途 | 行为 | 标签 |
| --- | --- | --- |
| Frozen Core `section_versions` | 对紧凑 JSON 字符串做 SHA-256，前缀 `sha256:` | 源码事实 `frozen_core.rs:187-191` |
| `GET /persona-workspace` `authority_versions` | 同样哈希当前 snapshot 正文 | 源码事实 `handlers/laputa.rs:938-950` |
| GUI `sessionVersion === authorityVersion` | 只显示「本会话已生效 / 下会话生效」 | 源码事实 `PersonaMemoryView.vue:49-59` |
| `WriteLaputaSectionPayload` | 无 `base_revision` / `if_match` / `expected_version` | 源码事实 |
| `create_user_edit_proposal` | 不读、不比当前版本 | 源码事实 |
| BML `record_revision` | Memory 行 CAS；**不**绑 Persona 保存 | 源码事实 `typed_store.rs:844+` |
| Governance `expected_version` | 审批账本版本，不是文档 revision | 源码事实 |

实验观察：`rg base_revision|if_match` 在 laputa/manager/gui Persona 写路径无匹配。
`expected_version` 出现在审批 decide，不出现在 section write。

## 6. WORLD：独立 claim 权威，无 Prompt 接线

源码：`cognitive/world.rs`、`world_governance.rs`、`cognitive/mod.rs`。

### 6.1 文件与 schema

- 路径：`.laputa/cognitive/WORLD.MD`；种子 `DEFAULT_WORLD_TEXT = "# WORLD\n"`。源码事实：`mod.rs:32-43`。
- Claim：`## [domain] title` + `status/confidence/scope/source/updated` + 正文 ≤ 280 字。
- 状态：`confirmed | observed | inferred | hypothesis | stale`。
- `project(scopes, budget)`：默认预算 4000、硬顶 16000；有 scope 按文件序，无 scope 按 confidence。
- `actor==user` 可任意覆盖；非 user 不得覆盖 `confirmed+source=user`，只能标 stale 并追加 `[note:]`。

旁路文件（R0 DECIDE，本包保持 DECIDE）：

- `cognitive/world-proposals.json` — 非 user 的 pending 队列
- `cognitive/world-ledger.jsonl` — submit/approve/reject 审计

这套状态机是 **WorldGovernance 专用**（`PendingReview | Applied | Rejected`），
不是 Persona 决策里的 `pending|accepted|rejected|stale`，也不是
`EvolutionProposal`。源码事实：`world_governance.rs:1-38`。

### 6.2 写入者

| 写入者 | 行为 | 标签 |
| --- | --- | --- |
| `initialize_dir` | 缺文件则写 `# WORLD\n`，不覆盖已有 | 源码事实 |
| 人类直接改文件 | 无 API；`actor==user` 语义在 `governed_upsert` | 源码事实 |
| Consolidation | `WorldGovernance::submit(..., "consolidation", payload)` → 非 user → **pending**；`apply`/`reject` **无生产调用者** | 源码事实 `consolidation.rs:344-368`；`world_governance.rs:227` 仅单测 |
| GUI | **无写路由**。Persona 页 WORLD 是 `<pre>` 只读 | 源码事实 `PersonaMemoryView.vue:136-139`；R0：无 WORLD POST |
| Agent 工具 | 无 WORLD 写工具 | 实验观察 |

模块注释写「No agent-facing write API exists; humans are the only editors」
（`cognitive/mod.rs:9`），但 Consolidation 已是非人类写入者。源码事实优先于注释。

Consolidation 把 claim `status` 写成 `"active"`（`consolidation.rs:145`），不是合法
`ClaimStatus`。若将来有人调用 `apply()`，`to_claim` 会失败。推断：pending 队列
可能堆积且无法落地。WORLD 提示词也未要求模型输出 WORLD frontmatter（对照
`world_claim_routing` 测试）。

### 6.3 与产品 P9 的差

产品：WORLD 参加五份权威首次引导，但是独立 claim 权威，不得作为第五个 Frozen
Core 整体注入。当前：WORLD 被预种子，物理上永远「存在」；Prompt 既不注入全文
（有测试锁），也不调用有界 `project()`。GUI 把 WORLD/MEMRULES 塞进 Persona
左栏 `cognitive` 组，与四份 Frozen Core、`memory_md`、changelog 并列。

## 7. 首次初始化：三套判定打架

产品 P10–P11：五份权威**全部不存在** → 一次原子直写 + 首批历史；不预种子；
不走 Proposal/Approval；服务端返回 `uninitialized | ready | incomplete`。

当前实现（无 initialize 路由，实验观察：`handlers/laputa.rs` 无 `initialize` /
`uninitialized`）：

| 判定 | 位置 | 把什么当成「未初始化」 | 冲突 |
| --- | --- | --- | --- |
| 文件预种子 | `LaputaStorage::open` → `initialize_sections` + `initialize_dir` | 从不让五文件同时缺席；目录可预创建，**权威文件也被创建** | 与 P10 absence-only **直接相反** |
| Prompt 空 Frozen Core | `frozen_core.is_empty()` | 四段 JSON 皆空/`null` | WORLD 不参与；`{}` 会关掉引导 |
| GUI localStorage | `WELCOME_STORAGE_KEY = "agent-diva-welcome-v1"` | 没看过技术向导 | `WelcomeWizard` 收集 DeepSeek/Bocha 密钥，**不是**五权威。源码事实：`WelcomeWizard.vue`、`App.vue:1948, 2100` |

后果（推断）：

1. 新 workspace 打开后四份 `null` JSON + `# WORLD\n` 已在磁盘，P10 的
   `uninitialized` 永远不会成立。
2. 模型在**每个空 Frozen Core 会话**被催 `ask_user` + `laputa_propose_section_write`，
   再走审批链；与「一次原子直写、不进 Approval」相反。
3. 用户关掉 Welcome 向导与人格是否存在无关。
4. 部分文件损坏没有 `incomplete` 修复入口；`read_section` 把非法 JSON 当成字符串
   `Value`（`service.rs:545` `unwrap_or_else(|_| json!(content))`）。

测试把预种子锁成合同：`frozen_core.rs:290-320`、`cognitive/sections.rs:60-76`。
D1 必须连同这些测试一起替换，不能只加新 API。

## 8. changelog ≠ 文档历史

`ChangelogRecord`（`core/evolution/types.rs:259-274`）：`before` / `after` /
`diff` / `proposal_id` / `reverted` / `stale` / 30 天回滚窗。

- 列表默认 `page_size` clamp 1–100；GUI `HistoryModal` 再切 50 条。源码事实：
  `service.rs:578-579`、`HistoryModal.vue:42-43`。
- Diff 由 `proposals.rs:729-748` `unified_diff` 生成：固定 `@@ -1 +1 @@`，
  先全部 `-` 再全部 `+`，**不是**逐行 LCS/Myers。
- `rollback_changelog`：仅 `ChangelogAction::Apply`；30 天；
  `expected_current` 默认同 `after`；Typed 路径 `write_authority=false` 只改
  生命周期记录。源码事实：`service.rs:612-672`。
- 回滚是**治理反操作**（有时间窗、可改权威文件），不是产品 P6/P12
  「载入历史 = 本地草稿，再保存 = 新头」。
- `HistoryModal` 只能复制 `after`，不能载入编辑器。源码事实：
  `HistoryModal.vue:141-151`。

R0 将 `changelog/<id>.json` 标 DECIDE。本包细化：changelog 是提案 apply 审计，
**不能**直接升格为 Persona/WORLD 永久 append-only revision store（高冲突，见
`revision-diff-options.md`）。

## 9. persona-retire（兼容链，DELETE）

`persona_retire.rs` + CLI `persona-retire plan|propose|archive`：

- 扫描根目录 `SOUL.md` / `IDENTITY.md` → Identity；`USER.md` → Relationship；
  `MEMORY.md` / `BOOTSTRAP.md` 只归档。
- `propose` 生成 JSON patch：`{status, entries[], metadata.migration=persona_retirement, content_type: legacy_markdown}`，再走普通提案。
- `archive` 要求对应 section JSON 的 `metadata.migration == "persona_retirement"`
  才允许 `rename` 进 `.laputa/legacy/`。Typed 默认不回写 JSON 时，此检查会失败
  （推断）。

与 P8 clean break 冲突：新 runtime 不得探测、转换或 fallback 这些文件。R4
评估是否只提供一次性人工备份说明。

## 10. GUI 工作区 vs 产品三态

产品 P3：左导航四文档 + 单一中央（当前文档 | 待审变更 | 历史）；删除永久右栏。

当前（源码事实 `PersonaMemoryView.vue`、`SectionGroupList.vue`）：

```text
左 230px   frozen_core×4 + long_term(memory_md) + changelog + memrules/world
中         JSON SectionEditor（textarea + JSON 预览 tab）
           或 WORLD/MEMRULES 只读 <pre>
右 250px   PersonaLifecyclePanel（decide / apply / edit patch / rollback）
≤1050px    右栏 display:none；不是产品「单工作区标签」
```

- 无「待审变更」只读 before/after Diff。pending 只是一条 note + 右栏治理。
- 无中央历史态。历史是模态，50 条 changelog。
- `Ctrl/Cmd+S` 存在，但保存文案是「提交提案」，且要求 changeReason。
- 切换分区若 dirty 会 `appConfirm` 丢草稿。源码事实：`PersonaMemoryView.vue:98-105`。
- 非 Tauri 直接空数据（桌面症状链之一）。源码事实：`81-85`。

Manager 投影 `GET /api/laputa/persona-workspace` 一次返回 snapshot + 全部
proposals（带 governance）+ changelog 100 条 + authority_versions +
optional session Frozen Core + memrules/world 原文。RENAME 随 D1 DTO。

无 WORLD 写路由。无 first-run status/initialize 路由。

## 11. KEEP / RENAME / DELETE / DECIDE（仅 Persona/WORLD 细化）

R0 全仓矩阵仍是权威。这里只改本切片精度：

| 对象 | 标记 | 说明 | 交给 |
| --- | --- | --- | --- |
| 四个对象 + WORLD **概念** | KEEP | 产品边界 | D1 换载体 |
| Frozen Core 会话冻结语义 | KEEP 概念 / RENAME 实现 | 捕获 Markdown 而非 JSON | D1 |
| `atomic_write` 同目录 rename | KEEP 基础设施 | 可复用于 `.md` | D1 |
| `content_version` SHA-256 | RENAME | 今日只展示；可作 revision 候选之一 | D1 选项 |
| `sections/*.json` + `content: Value` | DELETE | P8 | R4 |
| JSON 编辑器 / `formatJson` / `JSON.parse` 门 | DELETE | P1/P7 | R3 评估 / R4 证明 |
| 预种子 `null` 与 `# WORLD\n` | DELETE | P10 | D1 |
| Prompt `FIRST_RUN_ONBOARDING_BLOCK` + `laputa_propose_section_write` 引导 | DELETE | P11 | D1 |
| `WELCOME_STORAGE_KEY` 当人格完成标记 | DELETE（就此用途） | 技术向导可独立留下 | D1 |
| `PersonaLifecyclePanel` 通用治理 | DELETE | P2 | R4 |
| 左栏 `memory_md` / changelog section | DELETE | 已在 R0 | R4 |
| persona-retire CLI / `legacy/` 自动链 | DELETE | P8 | R4 |
| `WorldStore::project` | DECIDE | 有界投影 API 已存在、未接线 | D1 |
| WorldGovernance 队列 + ledger | DECIDE | 与 Persona 专用四态不是同一状态机 | D1 |
| MEMRULES 挂在 Persona 页 | DECIDE | 产品左栏只有四文档 + WORLD | D0/D1 |
| `changelog/<id>.json` 当人格历史 | DECIDE → **建议不当** | 治理记录、30 天窗、naive diff | D1；R4 是否删除 |
| BML Identity 行 vs JSON 文件 | DECIDE | 双权威 | D0 |

## 12. 开放问题（交给 D1 / D0 / R4）

1. Markdown 权威目录名与五文件原子提交形态（本包选项文列 Hold，不选）。
2. Frozen Core 改读 Markdown 后，C1 稳定前缀哈希 / 预算 / 空态判定如何改（R2 已把插入约束写清；本包只证明今天注入的是紧凑 JSON）。
3. WORLD `project()` 是否进入 Prompt、scope 从哪来、与 Mask/Frozen Core 的相对位置。
4. WorldGovernance 是否整段删除，或只保留 user-confirmed 保护规则于直写路径。
5. Agent 何时创建 `PersonaChangeRequest`（产品明确本阶段不决定）。
6. 同一文档待审队列长度（同上）。
7. 用户机器上真实 `sections/`、`changelog/`、`WORLD.MD` 体积与人工备份 → R4。
8. BML `Identity` 等 kind 是否继续存在 → D0。
9. MEMRULES 归属 → D0/D1。
