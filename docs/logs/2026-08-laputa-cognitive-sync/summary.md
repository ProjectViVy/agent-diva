# LAPUTA-COGNITIVE-SYNC 实施总结（S0–S7）

分支：`feat/laputa-cognitive-sync`（自 `agent-diva-pro` 切出，未 push）
权威提案：`docs/research/laputa-garden-cognitive-sync-2026-08/gap-and-migration-proposal.md`
冻结决策：Q1–Q5 + D1/D2（2026-08-07 用户确认，全部冻结）。

## 提交序列

| Commit | 切片 | 内容 |
|--------|------|------|
| `88195ffa` | S0 | fix: complete BoundedReflectionInput initializers in manager tests |
| `41e60a7e` | S1 | feat: add cognitive dir with MEMRULES.MD governance rules |
| `5788eddf` | S2 | feat: add WORLD.MD claim store with scope-budget projection |
| `b22b50e8` | S2 | feat: add governed WORLD claim upsert path with user-claim protection |
| `1c97d7be` | S3 | feat: freeze Frozen Core sections at session start |
| `49e778e1` | S4 | feat: add persona retirement migration through Frozen Core proposals |
| `57044ba8` | S4 | refactor: retire BOOTSTRAP.md injection from prompt assembly |
| `989a18a2` | S4 | refactor: remove soul file governance machinery |
| `ef0b89b2` | docs | mark S1–S4 complete in TODOLIST |
| `e61630b8` | S5 | refactor: hard-remove 5 obsolete LaputaSectionName variants |
| `2ec310d3` | docs | mark S5 complete in TODOLIST |
| `23ea4b1e` | S6 | refactor: remove rhythm patch proposal targets from governance |
| `856335b2` | S6 | refactor: ban rhythm and report content from startup prompt injection |
| `6def944e` | S6 | feat: move report artifacts to .laputa/reports with one-time migration |
| `2457239b` | S7 | test: add context plane negative invariant matrix |

## 各切片要点

- **S0**：Wave 5 S2（`16aa46ed`）给 `BoundedReflectionInput` 新增
  `superseded_memory_digests` 字段漏更新 manager 侧 4 处测试构造点，补齐后基线恢复。
- **S1**：`agent-diva-laputa/src/cognitive/`；workspace 级
  `.laputa/cognitive/MEMRULES.MD` 缺省种子（R1–R7，InitializeDir 语义永不覆盖），
  启动加载 + 缺失兜底；人类编辑 only，永不注入 prompt/ContextView。
- **S2**：`WORLD.MD` claim 存储（`## [domain] title` + status/confidence/scope/
  source/updated + ≤280 字符正文）；治理写路径为专用 `WorldGovernance`
  服务（`WorldUpsertProposal` candidate→PendingReview→apply 管线 +
  governance ledger，AutoDream 首切片即可提）；confirmed+source=user claim 保护
  （治理写仅可标 stale+追加备注）；投影默认 4000、上限 16000 字符，永不整体注入。
- **S3**：Identity/Relationship/Commitment/Preferences 四区会话启动快照
  （`FrozenCoreSnapshot::capture`），会话内治理写入下一会话生效。
- **S4**：人格文件层退役——SOUL/IDENTITY/USER 内容经 Frozen Core proposal 审批迁移，
  批准后源文件入 `.laputa/legacy/`；删除 SoulStateStore/SoulGovernanceSettings/
  soul watchlist/HISTORY.md append；BOOTSTRAP 装配除名。Mask 按 D1 保留为覆层，
  不进 Laputa 治理。
- **S5**：`LaputaSectionName` 14→8，硬删 HistoryMd/JournalReflective/ProposalInbox/
  ReportIndexes/AaakSummaries；`ProposalType::HistoryPatch`/`JournalNote` 同步移除；
  落盘历史反序列化走 unknown-variant 稳定失败/容错跳过（只读历史不崩），
  写已删 section 返回稳定失败码；`.laputa/sections/` 残留文件惰性保留。
- **S6**（Q5=b 边界四句：报告≠记忆、永不注入、agent 可选读取自决、生成权威=
  report_system）：删 Daily/Weekly/MonthlyPatch proposal 目标与记忆 kind 映射；
  移除 `## Rhythm Signals` 启动注入；agent 读报告仅走现有 read_file，不主动提示
  不索引；D2 产物 `.agent-diva/autodream/reports/` → `.laputa/reports/` 一次性
  迁移（只搬文件、幂等、冲突保留 legacy 副本作回滚源）。
- **S7**：ADR-0004 §6 八行矩阵移植为
  `agent-diva-laputa/tests/context_plane_invariants.rs` 负向回归测试
  （marker 字符串法，见 verification.md）。

## 影响

- prompt 装配不再读取 SOUL/IDENTITY/USER/BOOTSTRAP/legacy MEMORY/HISTORY 与任何
  报告/节律内容；认知文件（MEMRULES/WORLD）仅服务于治理写路径与受限投影。
- 注册表收敛后，旧 section/proposal 名字在所有入口（解析、serde、路由、写入）
  均为稳定失败。
- 报告产物统一落 `.laputa/reports/{daily,weekly,monthly}`，旧路径自动迁移。
